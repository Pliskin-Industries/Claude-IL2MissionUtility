"""Build a P14 probe flight as a ready-to-run Korea dogfight mission.

Combines, without the mission editor:
  * the Options block of the game's own Korea dogfight test mission
    (data/Multiplayer/Dogfight/_test_dogfight_IL-3.Mission, MissionType 2),
  * one probe .Group (indexes kept as written, so the replay's .trace.json
    still matches objective breadcrumbs by index),
  * one or more reference groups (e.g. TemplateExamples/K14 AFB_mp.Group),
    renumbered past the probe's indexes, with their language tables merged,
then runs IL-2's MissionResaver to write the .msnbin, language files and
.list the dedicated server loads.

usage:
  python tools/p14/build_probe_mission.py --game "C:\\Program Files\\IL2Series\\game"
      --probe target/p14/P14_Probe_0_traced.Group
      --ref "TemplateExamples/K14 AFB_mp.Group"
      --name P14_Probe_0_traced --title "P14 probe flight 0 (traced)"
      --out "C:\\Machine Intelligence\\P14 Flight 0\\mission"

Every group's .eng must sit next to it. Exit code 0 only if the resaver
reports success. Checked 2026-09-28: the loader rejects a quoted value that
holds ; = { } ' or a non-ASCII character (see probe_strings_are_loader_safe).
"""
import argparse
import os
import re
import shutil
import subprocess
import sys

INDEX_KEYS = ("Index", "LinkTrId", "MisObjID", "TarId", "CmdId")
ARRAY_KEYS = ("Targets", "Objects")
LC_KEYS = ("LCName", "LCDesc", "LCText")
LC_PROBE_OFFSET = 100      # Options uses 0, 1, 2
LC_REF_OFFSET = 1000       # then 1000 per reference group


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


DOGFIGHT_TEST = r"data\Multiplayer\Dogfight\_test_dogfight_IL-3.Mission"


def planes_block(game, country, scripts):
    """A Planes block for an Airfield of `country`, copied entry by entry from
    the game's own dogfight test airfield of that country: the ground-start
    ("Grouund Start", StartType 1) entry for each plane script named."""
    text = read_text(os.path.join(game, DOGFIGHT_TEST)).replace("\r\n", "\n")
    for a in re.finditer(r"\nAirfield\n\{\n(.*?)\n\}\n", text, re.S):
        body = a.group(1)
        if re.search(r"\bCountry = (\d+);", body).group(1) != str(country):
            continue
        entries = re.findall(r"\n    Plane\n    \{\n.*?\n    \}", body, re.S)
        chosen = []
        for script in scripts:
            hits = [e for e in entries if f"Planes\\{script}.txt" in e and "StartType = 1;" in e]
            if not hits:
                raise SystemExit(f"no ground-start {script} for country {country} in {DOGFIGHT_TEST}")
            chosen.append(hits[0])
        # The test file's airfield is at top level (2-space body); a reference
        # group's airfield sits one level deeper, so indent by two more spaces.
        block = "  Planes\n  {" + "".join(chosen) + "\n  }"
        return "\n".join("  " + l if l else l for l in block.split("\n"))
    raise SystemExit(f"no country {country} airfield in {DOGFIGHT_TEST}")


def add_airfield_planes(group_text, game, scripts):
    """Give every Airfield without a Planes list the chosen spawn planes."""
    text = group_text.replace("\r\n", "\n")
    added = []

    def fix(m):
        head, indent, body, tail = m.group(1), m.group(2), m.group(3), m.group(4)
        if re.search(r"^\s*Planes\s*$", body, re.M):
            return m.group(0)
        country = re.search(r"\bCountry = (\d+);", body).group(1)
        block = planes_block(game, country, scripts)
        body = re.sub(r"(\n\s*Callnum = -?\d+;)", lambda c: c.group(1) + "\n" + block, body, count=1)
        added.append((re.search(r'Name = "([^"]*)"', body).group(1), country))
        return head + body + tail
    text = re.sub(r"(\n(\s*)Airfield\n\2\{\n)(.*?)(\n\2\})", fix, text, flags=re.S)
    return text, added


def options_block(game, title_ids):
    src = os.path.join(game, r"data\Multiplayer\Dogfight\_test_dogfight_IL-3.Mission")
    lines = read_text(src).replace("\r\n", "\n").split("\n")
    end = next(i for i, l in enumerate(lines) if l == "}")  # closes Options
    block = "\n".join(lines[: end + 1])
    name, desc, author = title_ids
    block = re.sub(r"LCName = \d+;", f"LCName = {name};", block, count=1)
    block = re.sub(r"LCDesc = \d+;", f"LCDesc = {desc};", block, count=1)
    block = re.sub(r"LCAuthor = \d+;", f"LCAuthor = {author};", block, count=1)
    assert "MissionType = 2;" in block and "landscape_korea" in block, "not a Korea dogfight Options block"
    return block


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--game", required=True)
    ap.add_argument("--probe", required=True)
    ap.add_argument("--ref", action="append", default=[])
    ap.add_argument("--name", required=True)
    ap.add_argument("--title", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--spawn-planes", default="f80c10",
                    help="comma-separated plane scripts for reference airfields that have no Planes list")
    a = ap.parse_args()
    spawn_scripts = [s.strip() for s in a.spawn_planes.split(",") if s.strip()]

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
        text, added = add_airfield_planes(text, a.game, spawn_scripts)
        for airfield, country in added:
            print(f"spawn planes added to airfield {airfield} (country {country}): {', '.join(spawn_scripts)}")
        lc_offset = LC_REF_OFFSET * (i + 1)
        eng = read_eng(os.path.splitext(ref)[0] + ".eng")
        groups.append(shift_group(text, next_index, lc_offset))
        table.update({k + lc_offset: v for k, v in eng.items() if k})
        next_index = (max_index(groups[-1]) // 1000 + 1) * 1000 + 1000

    if problems:
        print("loader-unsafe strings:\n  " + "\n  ".join(problems))
        return 1

    os.makedirs(a.out, exist_ok=True)
    # MissionResaver keeps language files that already exist, so stale text
    # from an earlier build would survive in .chs/.fra/... Start clean.
    for ext in (".Mission", ".msnbin", ".list", ".eng", ".chs", ".fra", ".ger", ".rus", ".spa"):
        stale = os.path.join(a.out, a.name + ext)
        if os.path.exists(stale):
            os.remove(stale)
    mission = os.path.join(a.out, a.name + ".Mission")
    body = options_block(a.game, (0, 1, 2)).replace("\n", "\r\n")
    for g in groups:
        body += "\r\n\r\n" + g.replace("\r\n", "\n").replace("\n", "\r\n").rstrip()
    body += "\r\n\r\n# end of file"
    with open(mission, "w", encoding="utf-8", newline="") as f:
        f.write(body)
    write_eng(os.path.join(a.out, a.name + ".eng"), table)

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
