"""Build a P14 probe flight as a ready-to-run Korea dogfight mission.

Combines, without the mission editor:
  * the Options block of the game's own Korea dogfight test mission
    (data/Multiplayer/Dogfight/_test_dogfight_IL-3.Mission, MissionType 2),
  * one probe .Group (indexes kept as written, so the replay's .trace.json
    still matches objective breadcrumbs by index),
  * one or more reference groups (e.g. TemplateExamples/K14 AFB_mp.Group),
    renumbered past the probe's indexes, with their language tables merged,
  * optional --spawns JSON entries, copied as air starts from the test mission,
then runs IL-2's MissionResaver to write the .msnbin, language files and
.list the dedicated server loads. --no-resave writes only .Mission and .eng.

usage:
  python tools/p14/build_probe_mission.py --game "C:\\Program Files\\IL2Series\\game"
      --probe target/p14/P14_Probe_0_traced.Group
      --ref "TemplateExamples/K14 AFB_mp.Group"
      --name P14_Probe_0_traced --title "P14 probe flight 0 (traced)"
      --out "C:\\Machine Intelligence\\P14 Flight 0\\mission"

Every group's .eng must sit next to it. Exit code 0 requires resaver success
unless --no-resave is set. Checked 2026-09-28: the loader rejects a quoted value that
holds ; = { } ' or a non-ASCII character (see probe_strings_are_loader_safe).
"""
import argparse
import json
import math
import os
import re
import subprocess
import sys

INDEX_KEYS = ("Index", "LinkTrId", "MisObjID", "TarId", "CmdId")
ARRAY_KEYS = ("Targets", "Objects")
LC_KEYS = ("LCName", "LCDesc", "LCText")
LC_PROBE_OFFSET = 100      # Options uses 0, 1, 2
LC_REF_OFFSET = 1000       # then 1000 per reference group
START_TYPE_AIR = 0
START_TYPE_GROUND = 1


def read_text(path):
    with open(path, "rb") as f:
        raw = f.read()
    return raw.decode("utf-8-sig")


def read_eng(path):
    """id -> text from a UTF-16 LE (BOM) language file; empty if missing."""
    if not os.path.exists(path):
        return {}
    raw = open(path, "rb").read()
    text = raw[2:].decode("utf-16-le") if raw[:2] == b"\xff\xfe" else raw.decode("utf-8-sig")
    table = {}
    for line in text.splitlines():
        head, sep, rest = line.partition(":")
        if sep and head.strip().lstrip("-").isdigit():
            table[int(head)] = rest
    return table


def write_eng(path, table):
    body = "".join(f"{k}:{v}\r\n" for k, v in sorted(table.items()))
    with open(path, "wb") as f:
        f.write(b"\xff\xfe" + body.encode("utf-16-le"))


def max_index(text):
    return max(int(m) for m in re.findall(r"^\s*Index = (\d+);", text, re.M))


def shift_group(text, index_offset, lc_offset):
    """Renumber every MCU/object index reference and every LC id (0 stays 0)."""
    def one(m):
        key, value = m.group(1), int(m.group(2))
        if key in LC_KEYS:
            return f"{key} = {value + lc_offset if value else 0};"
        return f"{key} = {value + index_offset if value else 0};"

    keys = "|".join(INDEX_KEYS + LC_KEYS) if index_offset else "|".join(LC_KEYS)
    text = re.sub(rf"\b({keys}) = (\d+);", one, text)

    def arr(m):
        items = [s.strip() for s in m.group(2).split(",") if s.strip()]
        items = [str(int(s) + index_offset) for s in items]
        return f"{m.group(1)} = [{','.join(items)}];"

    if index_offset:
        text = re.sub(rf"\b({'|'.join(ARRAY_KEYS)}) = \[([^\]]*)\];", arr, text)
    return text


def check_loader_safe(text, label):
    bad = []
    for n, line in enumerate(text.splitlines(), 1):
        if ' = "' not in line:
            continue
        value = line.split(' = "', 1)[1].rsplit('"', 1)[0]
        if not value.isascii() or any(c in value for c in "=;{}'"):
            bad.append(f"{label}:{n}: {line.strip()}")
    return bad


def loader_unsafe_message(problems):
    return "loader-unsafe strings:\n  " + "\n  ".join(problems)


def reject_loader_unsafe(problems):
    if problems:
        raise SystemExit(loader_unsafe_message(problems))


DOGFIGHT_TEST = r"data\Multiplayer\Dogfight\_test_dogfight_IL-3.Mission"


def shipped_blocks(lines, kind, path, indent=""):
    """Locate blocks in the shipped mission's exact, indentation-based shape."""
    for start, line in enumerate(lines):
        if line != indent + kind:
            continue
        if start + 1 >= len(lines) or lines[start + 1] != indent + "{":
            raise SystemExit(f"{path}: {kind} block: missing opening '{{'")
        for end in range(start + 2, len(lines)):
            if lines[end] == indent + "}":
                yield start, end + 1
                break
            if lines[end].strip() and not lines[end].startswith(indent + "  "):
                raise SystemExit(f"{path}: {kind} block: missing closing '}}'")
        else:
            raise SystemExit(f"{path}: {kind} block: missing closing '}}'")


def required_block(lines, kind, path, indent="", parent="mission"):
    block = next(shipped_blocks(lines, kind, path, indent), None)
    if block is None:
        raise SystemExit(f"{path}: {parent} block: missing {kind} block")
    return block


def property_line(lines, key, path):
    if not lines:
        raise SystemExit(f"{path}: empty block: missing {key} key")
    indent = lines[0][:len(lines[0]) - len(lines[0].lstrip())] + "  "
    prefix = indent + key + " = "
    for index, line in enumerate(lines):
        if line.startswith(prefix):
            if not line.endswith(";"):
                raise SystemExit(f"{path}: {lines[0].strip()} block: {key} key lacks ';'")
            return index, prefix
    raise SystemExit(f"{path}: {lines[0].strip()} block: missing {key} key")


def block_value(lines, key, path):
    index, prefix = property_line(lines, key, path)
    return lines[index][len(prefix):-1]


def set_block_value(lines, key, value, path):
    index, prefix = property_line(lines, key, path)
    lines[index] = prefix + str(value) + ";"


class ShippedMission:
    """Read the test mission once for Options and both kinds of spawn points."""

    def __init__(self, game):
        self.path = os.path.join(game, DOGFIGHT_TEST)
        self.lines = read_text(self.path).splitlines()
        self.airfields = [self.lines[start:end] for start, end
                          in shipped_blocks(self.lines, "Airfield", self.path)]
        self.entities = [self.lines[start:end] for start, end
                         in shipped_blocks(self.lines, "MCU_TR_Entity", self.path)]

    def airfield(self, country):
        for block in self.airfields:
            if block_value(block, "Country", self.path) == str(country):
                return block.copy()
        raise SystemExit(f"{self.path}: Airfield block: no Country = {country} entry")

    def linked_entity(self, airfield):
        link = block_value(airfield, "LinkTrId", self.path)
        index = block_value(airfield, "Index", self.path)
        for block in self.entities:
            if (block_value(block, "Index", self.path) == link
                    and block_value(block, "MisObjID", self.path) == index):
                return block.copy()
        raise SystemExit(f"{self.path}: MCU_TR_Entity block: no Index = {link} "
                         f"with MisObjID = {index} for Airfield LinkTrId")


def airfield_planes(airfield, path):
    start, end = required_block(airfield, "Planes", path, "  ", "Airfield")
    planes = airfield[start:end]
    entries = [planes[first:last] for first, last
               in shipped_blocks(planes, "Plane", path, "    ")]
    return start, end, entries


def choose_plane(entries, script, start_type, country, path):
    script_path = f"luascripts\\worldobjects\\planes\\{script}.txt"
    for plane in entries:
        if (block_value(plane, "StartType", path) == str(start_type)
                and block_value(plane, "Script", path).strip('"').lower() == script_path.lower()):
            return plane.copy()
    raise SystemExit(f"{path}: Airfield Country = {country}, Plane block: "
                     f"no Script = {script_path} with StartType = {start_type}")


def validate_spawn_entry(entry, label):
    required = ("name", "country", "x", "z", "heading_deg", "altitude_m", "script")
    if not isinstance(entry, dict) or any(key not in entry for key in required):
        raise SystemExit(f"{label}: expected {', '.join(required)}")
    for key in ("name", "script"):
        value = entry[key]
        if not isinstance(value, str) or '"' in value or any(ord(c) < 32 for c in value):
            raise SystemExit(f"{label}: loader-unsafe {key}")
        reject_loader_unsafe(check_loader_safe(f'{key} = "{value}";', label))
    if type(entry["country"]) is not int:
        raise SystemExit(f"{label}: country must be an integer")
    for key in ("x", "z", "heading_deg", "altitude_m"):
        if type(entry[key]) not in (int, float) or not math.isfinite(entry[key]):
            raise SystemExit(f"{label}: {key} must be a finite number")


def build_spawn_airfield(mission, entry):
    airfield = mission.airfield(entry["country"])
    entity = mission.linked_entity(airfield)
    start, end, entries = airfield_planes(airfield, mission.path)
    chosen = choose_plane(entries, entry["script"], START_TYPE_AIR, entry["country"], mission.path)
    set_block_value(chosen, "Altitude", entry["altitude_m"], mission.path)
    airfield[start:end] = ["  Planes", "  {"] + chosen + ["  }"]
    set_block_value(airfield, "Name", f'"{entry["name"]}"', mission.path)
    for block in (airfield, entity):
        for key, source in (("XPos", "x"), ("ZPos", "z"), ("YOri", "heading_deg")):
            set_block_value(block, key, entry[source], mission.path)
    return "\n".join(airfield + [""] + entity)


def spawn_airfields(mission, path, next_index):
    """Copy each country's fakefield/entity and one matching air-start entry."""
    try:
        entries = json.loads(read_text(path))
    except (OSError, ValueError) as error:
        raise SystemExit(f"{path}: cannot read spawn list: {error}") from error
    if not isinstance(entries, list):
        raise SystemExit(f"{path}: spawn list must be a JSON array")
    groups = []
    for number, entry in enumerate(entries, 1):
        label = f"{path}: entry {number}"
        validate_spawn_entry(entry, label)
        text = shift_group(build_spawn_airfield(mission, entry), next_index, 0)
        reject_loader_unsafe(check_loader_safe(text, label))
        groups.append(text)
        next_index = max_index(text) + 1
    return groups


def planes_block(mission, country, scripts):
    """A Planes block for an Airfield of `country`, copied entry by entry from
    the game's own dogfight test airfield of that country: the ground-start
    ("Grouund Start", StartType 1) entry for each plane script named."""
    airfield = mission.airfield(country)
    _, _, entries = airfield_planes(airfield, mission.path)
    chosen = [choose_plane(entries, script, START_TYPE_GROUND, country, mission.path)
              for script in scripts]
    # The test file's airfield is at top level (2-space body); a reference
    # group's airfield sits one level deeper, so indent by two more spaces.
    block = "  Planes\n  {" + "".join("\n" + "\n".join(plane) for plane in chosen) + "\n  }"
    return "\n".join("  " + line if line else line for line in block.split("\n"))


def add_airfield_planes(group_text, mission, scripts, label):
    """Give every Airfield without a Planes list the chosen spawn planes."""
    text = group_text.replace("\r\n", "\n")
    added = []

    def fix(m):
        head, indent, body, tail = m.group(1), m.group(2), m.group(3), m.group(4)
        if re.search(r"^\s*Planes\s*$", body, re.M):
            return m.group(0)
        country = re.search(r"\bCountry = (\d+);", body)
        if country is None:
            raise SystemExit(f"{label}: Airfield block: missing Country key")
        name = re.search(r'Name = "([^"]*)"', body)
        if name is None:
            raise SystemExit(f"{label}: Airfield block: missing Name key")
        country = country.group(1)
        block = planes_block(mission, country, scripts)
        body = re.sub(r"(\n\s*Callnum = -?\d+;)", lambda c: c.group(1) + "\n" + block, body, count=1)
        added.append((name.group(1), country))
        return head + body + tail
    text = re.sub(r"(\n(\s*)Airfield\n\2\{\n)(.*?)(\n\2\})", fix, text, flags=re.S)
    return text, added


def options_block(mission, title_ids):
    start, end = required_block(mission.lines, "Options", mission.path)
    options = mission.lines[start:end]
    for key, value in zip(("LCName", "LCDesc", "LCAuthor"), title_ids):
        set_block_value(options, key, value, mission.path)
    block = "\n".join(mission.lines[:start] + options)
    assert "MissionType = 2;" in block and "landscape_korea" in block, "not a Korea dogfight Options block"
    return block


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", required=True)
    ap.add_argument("--probe", required=True)
    ap.add_argument("--ref", action="append", default=[])
    ap.add_argument("--spawns", help="JSON spawn list written by a probe writer")
    ap.add_argument("--no-resave", action="store_true",
                    help="write .Mission and .eng without running MissionResaver")
    ap.add_argument("--name", required=True)
    ap.add_argument("--title", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--spawn-planes", default="f80c10",
                    help="comma-separated plane scripts for reference airfields that have no Planes list")
    a = ap.parse_args()
    spawn_scripts = [s.strip() for s in a.spawn_planes.split(",") if s.strip()]
    shipped = ShippedMission(a.game)

    probe = read_text(a.probe)
    table = {0: a.title, 1: "P14 in-game probe. Generated by IL-2 Mission Utility; see handoff/P14-air-tasking.md.", 2: "IL-2 Mission Utility"}
    groups, problems = [], check_loader_safe(probe, a.probe)

    probe_eng = read_eng(os.path.splitext(a.probe)[0] + ".eng")
    groups.append(shift_group(probe, 0, LC_PROBE_OFFSET))
    table.update({k + LC_PROBE_OFFSET: v for k, v in probe_eng.items() if k})

    next_index = (max_index(probe) // 1000 + 1) * 1000 + 1000
    for i, ref in enumerate(a.ref):
        text = read_text(ref)
        problems += check_loader_safe(text, ref)
        text, added = add_airfield_planes(text, shipped, spawn_scripts, ref)
        for airfield, country in added:
            print(f"spawn planes added to airfield {airfield} (country {country}): {', '.join(spawn_scripts)}")
        lc_offset = LC_REF_OFFSET * (i + 1)
        eng = read_eng(os.path.splitext(ref)[0] + ".eng")
        groups.append(shift_group(text, next_index, lc_offset))
        table.update({k + lc_offset: v for k, v in eng.items() if k})
        next_index = (max_index(groups[-1]) // 1000 + 1) * 1000 + 1000

    if a.spawns:
        groups.extend(spawn_airfields(shipped, a.spawns, next_index))

    if problems:
        print(loader_unsafe_message(problems))
        return 1

    os.makedirs(a.out, exist_ok=True)
    # MissionResaver keeps language files that already exist, so stale text
    # from an earlier build would survive in .chs/.fra/... Start clean.
    for ext in (".Mission", ".msnbin", ".list", ".eng", ".chs", ".fra", ".ger", ".rus", ".spa"):
        stale = os.path.join(a.out, a.name + ext)
        if os.path.exists(stale):
            os.remove(stale)
    mission = os.path.join(a.out, a.name + ".Mission")
    body = options_block(shipped, (0, 1, 2)).replace("\n", "\r\n")
    for g in groups:
        body += "\r\n\r\n" + g.replace("\r\n", "\n").replace("\n", "\r\n").rstrip()
    body += "\r\n\r\n# end of file"
    with open(mission, "w", encoding="utf-8", newline="") as f:
        f.write(body)
    write_eng(os.path.join(a.out, a.name + ".eng"), table)

    if a.no_resave:
        print(f"wrote {mission} and {a.name}.eng (--no-resave)")
        return 0

    exe = os.path.join(a.game, r"bin\resaver\MissionResaver.exe")
    r = subprocess.run([exe, "-d", os.path.join(a.game, "data"), "-f", os.path.abspath(mission)],
                       cwd=os.path.dirname(exe), capture_output=True, text=True, errors="replace")
    for line in r.stdout.splitlines():
        if line.startswith(("Loading", "Prepare", "Saving")):
            print(line.replace(os.path.abspath(mission), a.name + ".Mission"))
    print("resaver exit", r.returncode)
    if r.returncode == 0:
        for f in sorted(os.listdir(a.out)):
            if f.startswith(a.name + "."):
                print(f"  {f}  {os.path.getsize(os.path.join(a.out, f))} bytes")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
