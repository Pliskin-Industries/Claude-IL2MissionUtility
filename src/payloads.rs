//! payloads.rs — aircraft payload and modification tables
//!
//! Parsed once from `assets/Payloads.txt` (`OnceLock`). A UI overlay
//! keyed by script filename, not Group AST parsing — the seat only
//! stores `payload_id` / `mod_mask`; this module owns the catalog and
//! the ModMask bit helpers. Updating the text file and rebuilding picks
//! up new aircraft. It does not write Plane properties (`template` does).
//!
//! ## Public API
//! * `fn catalog` / `struct PayloadCatalog` (`for_script`, munition lookup)
//! * `PayloadOption` / `ModSlot` / `ModOption` / `Munition` / `AircraftLoadout`
//! * `fn payload_preview` / `fn mods_preview` — one-line UI text
//! * `fn parse_mod_mask` / `fn encode_mod_mask`
//! * `fn select_exclusive` / `fn set_toggle` / `fn option_selected`
//!
//! ## Used by
//! * ui.rs (Template) — payload combo, mod checkboxes, preview lines

use std::sync::OnceLock;

use crate::model_spec::script_id;

const SOURCE: &str = include_str!("../assets/Payloads.txt");

static CATALOG: OnceLock<PayloadCatalog> = OnceLock::new();

pub fn catalog() -> &'static PayloadCatalog {
    CATALOG.get_or_init(|| parse_payloads(SOURCE))
}

#[derive(Clone, Debug, Default)]
pub struct PayloadCatalog {
    pub munitions: Vec<Munition>,
    pub aircraft: Vec<AircraftLoadout>,
}

#[derive(Clone, Debug)]
pub struct Munition {
    pub name: String,
    pub kind: String,
    pub description: String,
    pub mass: String,
}

#[derive(Clone, Debug, Default)]
pub struct AircraftLoadout {
    pub name: String,
    pub payloads: Vec<PayloadOption>,
    pub mod_slots: Vec<ModSlot>,
}

#[derive(Clone, Debug)]
pub struct PayloadOption {
    pub id: i32,
    pub columns: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct ModSlot {
    pub number: u32,
    pub options: Vec<ModOption>,
}

#[derive(Clone, Debug)]
pub struct ModOption {
    pub binary_id: String,
    pub description: String,
}

impl PayloadCatalog {
    pub fn for_script(&self, script: &str) -> Option<&AircraftLoadout> {
        let id = script_id(script);
        self.aircraft
            .iter()
            .filter(|ac| matches_aircraft(&ac.name, &id))
            .max_by_key(|ac| normalize_key(&ac.name).len())
    }

    pub fn munition_for_cell(&self, cell: &str) -> Option<&Munition> {
        let stripped = strip_quantity(cell);
        if stripped.is_empty() || is_blank_ordnance(stripped) {
            return None;
        }
        let needle = normalize_key(stripped);
        if needle.is_empty() {
            return None;
        }
        self.munitions
            .iter()
            .filter(|m| {
                let n = normalize_key(&m.name);
                !n.is_empty() && (needle == n || needle.starts_with(&n) || n.starts_with(&needle))
            })
            .max_by_key(|m| normalize_key(&m.name).len())
    }
}

impl PayloadOption {
    pub fn summary(&self) -> String {
        let parts: Vec<&str> = self
            .columns
            .iter()
            .map(|(_, v)| v.as_str())
            .filter(|v| !is_blank_ordnance(v) && !is_none_word(v))
            .collect();
        if parts.is_empty() {
            "Clean".into()
        } else {
            parts.join(" · ")
        }
    }

    pub fn description(&self, cat: &PayloadCatalog) -> String {
        let mut lines = Vec::new();
        for (header, value) in &self.columns {
            if is_blank_ordnance(value) {
                continue;
            }
            if is_none_word(value) {
                continue;
            }
            if let Some(mun) = cat.munition_for_cell(value) {
                let mut line = format!("{value} — {}", mun.description);
                if !mun.kind.is_empty() {
                    line.push_str(" (");
                    line.push_str(&mun.kind);
                    line.push(')');
                }
                if !mun.mass.is_empty() {
                    line.push_str(". ");
                    line.push_str(&mun.mass);
                }
                lines.push(line);
            } else {
                lines.push(format!("{header}: {value}"));
            }
        }
        if lines.is_empty() {
            "Clean — no external stores.".into()
        } else {
            lines.join("\n")
        }
    }
}

impl AircraftLoadout {
    pub fn payload(&self, id: i32) -> Option<&PayloadOption> {
        self.payloads.iter().find(|p| p.id == id)
    }

    pub fn has_payloads(&self) -> bool {
        !self.payloads.is_empty()
    }

    pub fn has_mods(&self) -> bool {
        self.mod_slots.iter().any(|s| !s.options.is_empty())
    }
}

pub fn payload_preview(script: &str, payload_id: i32) -> String {
    match catalog().for_script(script) {
        Some(ac) if ac.has_payloads() => ac
            .payload(payload_id)
            .map(|p| p.summary())
            .unwrap_or_else(|| format!("ID {payload_id}")),
        _ => {
            if payload_id == 0 {
                "—".into()
            } else {
                format!("ID {payload_id}")
            }
        }
    }
}

pub fn mods_preview(script: &str, mod_mask: &str) -> String {
    let Some(ac) = catalog().for_script(script) else {
        return if is_default_mask(mod_mask) {
            "—".into()
        } else {
            mod_mask.to_string()
        };
    };
    if !ac.has_mods() {
        return "—".into();
    }
    let bits = parse_mod_mask(mod_mask);
    let mut labels = Vec::new();
    for slot in &ac.mod_slots {
        for opt in &slot.options {
            let extra = extra_bits(&opt.binary_id);
            if extra == 0 {
                continue;
            }
            if bits & extra == extra {
                labels.push(opt.description.as_str());
            }
        }
    }
    if labels.is_empty() {
        "—".into()
    } else {
        labels.join(", ")
    }
}

pub fn parse_mod_mask(s: &str) -> u64 {
    let s = s.trim().trim_matches('"');
    if s.is_empty() {
        return 1;
    }
    u64::from_str_radix(s, 2).unwrap_or(1)
}

pub fn encode_mod_mask(bits: u64) -> String {
    format!("{bits:b}")
}

pub fn extra_bits(binary_id: &str) -> u64 {
    parse_mod_mask(binary_id) & !1
}

pub fn slot_bits(slot: &ModSlot) -> u64 {
    slot.options
        .iter()
        .fold(0u64, |acc, o| acc | extra_bits(&o.binary_id))
}

/// Select `option` in an exclusive slot, clearing the other options in that slot.
pub fn select_exclusive(mask: u64, slot: &ModSlot, option: &ModOption) -> u64 {
    (mask & !slot_bits(slot)) | extra_bits(&option.binary_id) | 1
}

pub fn set_toggle(mask: u64, option: &ModOption, on: bool) -> u64 {
    let extra = extra_bits(&option.binary_id);
    if extra == 0 {
        return mask | 1;
    }
    if on {
        mask | extra | 1
    } else {
        (mask & !extra) | 1
    }
}

pub fn option_selected(mask: u64, option: &ModOption) -> bool {
    let extra = extra_bits(&option.binary_id);
    if extra == 0 {
        false
    } else {
        mask & extra == extra
    }
}

/// Exclusive slot: which option is active. Empty (binary `1`) when no extra bits are set.
pub fn exclusive_selection<'a>(mask: u64, slot: &'a ModSlot) -> Option<&'a ModOption> {
    let current = mask & slot_bits(slot);
    if current == 0 {
        return slot
            .options
            .iter()
            .find(|o| extra_bits(&o.binary_id) == 0)
            .or_else(|| slot.options.first());
    }
    slot.options
        .iter()
        .find(|o| extra_bits(&o.binary_id) == current)
        .or_else(|| {
            slot.options
                .iter()
                .filter(|o| {
                    let e = extra_bits(&o.binary_id);
                    e != 0 && current & e == e
                })
                .max_by_key(|o| extra_bits(&o.binary_id).count_ones())
        })
}

fn is_default_mask(s: &str) -> bool {
    matches!(s.trim(), "" | "1" | "0")
}

fn is_blank_ordnance(s: &str) -> bool {
    let t = s.trim();
    t.is_empty() || t == "—" || t == "-" || t == "–"
}

fn is_none_word(s: &str) -> bool {
    let t = s.trim();
    t.eq_ignore_ascii_case("none") || t.eq_ignore_ascii_case("none (clean)")
}

fn strip_quantity(s: &str) -> &str {
    let s = s.trim();
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return s;
    }
    let rest = s[digits..].trim_start();
    let mut chars = rest.chars();
    match chars.next() {
        Some('×' | 'x' | 'X') => chars.as_str().trim_start(),
        _ => s,
    }
}

fn normalize_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn matches_aircraft(header: &str, script_id: &str) -> bool {
    let h = normalize_key(header);
    if h.is_empty() {
        return false;
    }
    if script_id == h {
        return true;
    }
    // F-51 → f51d, B-29 → simpleb29
    script_id.starts_with(&h) || script_id.contains(&h)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    chars.next();
                    cur.push('"');
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    fields.push(cur.trim().to_string());
    fields
}

fn is_title_line(line: &str) -> bool {
    let fields = parse_csv_line(line);
    fields.len() <= 1 && !line.trim().is_empty()
}

fn parse_payloads(src: &str) -> PayloadCatalog {
    let mut cat = PayloadCatalog::default();
    let mut pending_title: Option<String> = None;
    let mut mode = Mode::None;
    let mut payload_headers: Vec<String> = Vec::new();

    for raw in src.lines() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        if is_title_line(line) {
            pending_title = Some(line.to_string());
            mode = Mode::None;
            continue;
        }
        let fields = parse_csv_line(line);
        let head = fields.first().map(|s| s.as_str()).unwrap_or("");
        if head.eq_ignore_ascii_case("Munition") {
            mode = Mode::Munitions;
            pending_title = None;
            continue;
        }
        if head.eq_ignore_ascii_case("Payload ID") {
            let name = pending_title.take().unwrap_or_default();
            if name.is_empty() || is_section_banner(&name) {
                mode = Mode::None;
                continue;
            }
            payload_headers = fields.iter().skip(1).cloned().collect();
            let idx = ensure_aircraft(&mut cat, &name);
            mode = Mode::Payloads(idx);
            continue;
        }
        if head.eq_ignore_ascii_case("Modification") {
            let name = pending_title.take().unwrap_or_default();
            if name.is_empty() || is_section_banner(&name) {
                mode = Mode::None;
                continue;
            }
            let idx = ensure_aircraft(&mut cat, &name);
            mode = Mode::Mods(idx);
            continue;
        }
        match mode {
            Mode::Munitions => {
                if fields.len() >= 2 && !fields[0].is_empty() {
                    cat.munitions.push(Munition {
                        name: fields[0].clone(),
                        kind: fields.get(1).cloned().unwrap_or_default(),
                        description: fields.get(2).cloned().unwrap_or_default(),
                        mass: fields.get(3).cloned().unwrap_or_default(),
                    });
                }
            }
            Mode::Payloads(idx) => {
                if let Some(id) = fields.first().and_then(|s| s.parse::<i32>().ok()) {
                    let mut columns = Vec::new();
                    for (i, header) in payload_headers.iter().enumerate() {
                        let value = fields.get(i + 1).cloned().unwrap_or_default();
                        columns.push((header.clone(), value));
                    }
                    cat.aircraft[idx].payloads.push(PayloadOption { id, columns });
                }
            }
            Mode::Mods(idx) => {
                if let (Some(slot_n), Some(binary)) = (fields.first(), fields.get(1)) {
                    if let Ok(number) = slot_n.parse::<u32>() {
                        let description = fields.get(2).cloned().unwrap_or_default();
                        let ac = &mut cat.aircraft[idx];
                        if let Some(slot) = ac.mod_slots.iter_mut().find(|s| s.number == number) {
                            slot.options.push(ModOption {
                                binary_id: binary.clone(),
                                description,
                            });
                        } else {
                            ac.mod_slots.push(ModSlot {
                                number,
                                options: vec![ModOption {
                                    binary_id: binary.clone(),
                                    description,
                                }],
                            });
                        }
                    }
                }
            }
            Mode::None => {}
        }
    }
    coalesce_related_aircraft(&mut cat);
    cat
}

fn is_section_banner(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("payload") && !n.contains("id") || n == "soviet payloads"
}

/// `F-51` payload table and `F-51D` mod table are the same aircraft.
fn related_aircraft_names(a: &str, b: &str) -> bool {
    let ka = normalize_key(a);
    let kb = normalize_key(b);
    if ka.is_empty() || kb.is_empty() {
        return false;
    }
    ka == kb || ka.starts_with(&kb) || kb.starts_with(&ka)
}

fn merge_loadout(into: &mut AircraftLoadout, from: AircraftLoadout) {
    if normalize_key(&from.name).len() > normalize_key(&into.name).len() {
        into.name = from.name;
    }
    for p in from.payloads {
        if !into.payloads.iter().any(|e| e.id == p.id) {
            into.payloads.push(p);
        }
    }
    for slot in from.mod_slots {
        if let Some(existing) = into.mod_slots.iter_mut().find(|s| s.number == slot.number) {
            for opt in slot.options {
                if !existing
                    .options
                    .iter()
                    .any(|o| o.binary_id == opt.binary_id)
                {
                    existing.options.push(opt);
                }
            }
        } else {
            into.mod_slots.push(slot);
        }
    }
}

fn coalesce_related_aircraft(cat: &mut PayloadCatalog) {
    let mut i = 0;
    while i < cat.aircraft.len() {
        let mut j = i + 1;
        while j < cat.aircraft.len() {
            if related_aircraft_names(&cat.aircraft[i].name, &cat.aircraft[j].name) {
                let other = cat.aircraft.remove(j);
                merge_loadout(&mut cat.aircraft[i], other);
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

fn ensure_aircraft(cat: &mut PayloadCatalog, name: &str) -> usize {
    if let Some(i) = cat
        .aircraft
        .iter()
        .position(|a| related_aircraft_names(&a.name, name))
    {
        if normalize_key(name).len() > normalize_key(&cat.aircraft[i].name).len() {
            cat.aircraft[i].name = name.to_string();
        }
        i
    } else {
        cat.aircraft.push(AircraftLoadout {
            name: name.to_string(),
            payloads: Vec::new(),
            mod_slots: Vec::new(),
        });
        cat.aircraft.len() - 1
    }
}

#[derive(Clone, Copy)]
enum Mode {
    None,
    Munitions,
    Payloads(usize),
    Mods(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_western_and_soviet_payloads() {
        let cat = catalog();
        let f80 = cat.for_script("f80c10.txt").expect("F-80C-10");
        assert!(f80.payloads.len() >= 50);
        let p5 = f80.payload(5).expect("payload 5");
        let summary = p5.summary();
        assert!(summary.contains("Drop Tank 165"), "{summary}");
        assert!(summary.contains("AN-M64A1"), "{summary}");

        let mig = cat.for_script(r"LuaScripts\WorldObjects\Planes\mig15bis.txt").unwrap();
        assert_eq!(mig.payload(0).unwrap().summary(), "Clean");
        assert!(mig.payload(1).unwrap().summary().contains("250"));

        let f51 = cat.for_script("f51d.txt").expect("F-51 matches F-51D");
        assert!(f51.payloads.len() > 20);
        assert!(f51.has_mods(), "F-51D mods merge with F-51 payloads");
        assert!(f51.mod_slots.len() >= 5);
        assert!(f51
            .mod_slots
            .iter()
            .any(|s| s.options.iter().any(|o| o.description.contains("Gunsite")
                || o.description.contains("Gunsight"))));

        let b29 = cat.for_script("simpleb29.txt").expect("B-29 matches simpleb29");
        assert!(b29.payload(1).unwrap().summary().contains("AN-M64A1"));
    }

    #[test]
    fn unknown_aircraft_has_no_table() {
        assert!(catalog().for_script("t34-85.txt").is_none());
        assert!(catalog().for_script("la11.txt").is_none());
    }

    #[test]
    fn munition_description_from_payload_cell() {
        let cat = catalog();
        let mun = cat
            .munition_for_cell("2× AN-M64A1 500 lb HE")
            .expect("500 lb");
        assert!(mun.description.to_ascii_lowercase().contains("500"));
        let hvar = cat.munition_for_cell("4× HVAR 5\"").expect("HVAR");
        assert!(hvar.kind.to_ascii_lowercase().contains("rocket"));
        let fab = cat.munition_for_cell("2× FAB-100sc").expect("FAB-100");
        assert!(!fab.description.is_empty());
    }

    #[test]
    fn csv_quoted_comma_and_inch_mark() {
        let fields = parse_csv_line(
            r#""HVAR 5""",HE Rocket,"High Velocity Aircraft Rocket, High Explosive",mass"#,
        );
        assert_eq!(fields[0], r#"HVAR 5""#);
        assert_eq!(fields[1], "HE Rocket");
        assert!(fields[2].contains("High Explosive"));
    }

    #[test]
    fn mod_mask_binary_logic() {
        assert_eq!(encode_mod_mask(1), "1");
        assert_eq!(encode_mod_mask(0b11), "11");
        assert_eq!(encode_mod_mask(0b101), "101");
        assert_eq!(encode_mod_mask(0b1001), "1001");
        assert_eq!(encode_mod_mask(0b10001), "10001");
        let two_and_four = parse_mod_mask("101") | parse_mod_mask("10001");
        assert_eq!(encode_mod_mask(two_and_four), "10101");
        assert_eq!(parse_mod_mask("1"), 1);
    }

    #[test]
    fn c47_mods_are_one_exclusive_slot() {
        let ac = catalog().for_script("c47b.txt").expect("C-47B");
        assert_eq!(ac.mod_slots.len(), 1);
        assert!(ac.payloads.is_empty());
        let slot = &ac.mod_slots[0];
        assert!(slot.options.iter().any(|o| o.description == "Cargo"));
        assert!(slot.options.iter().any(|o| o.description == "Ambulance"));
        let cargo = slot.options.iter().find(|o| o.description == "Cargo").unwrap();
        let mask = select_exclusive(1, slot, cargo);
        assert_eq!(encode_mod_mask(mask), "11");
        assert_eq!(
            exclusive_selection(mask, slot).map(|o| o.description.as_str()),
            Some("Cargo")
        );
        assert_eq!(mods_preview("c47b.txt", "11"), "Cargo");
        assert_eq!(mods_preview("c47b.txt", "1"), "—");
    }

    #[test]
    fn yak9p_mods_combine_across_slots() {
        let ac = catalog().for_script("yak9p.txt").expect("Yak-9P");
        assert_eq!(ac.mod_slots.len(), 4);
        let mut mask = 1u64;
        for slot in &ac.mod_slots {
            if slot.number == 2 || slot.number == 4 {
                mask = set_toggle(mask, &slot.options[0], true);
            }
        }
        assert_eq!(encode_mod_mask(mask), "10101");
        let preview = mods_preview("yak9p.txt", "10101");
        assert!(preview.contains("Gunsight"), "{preview}");
        assert!(preview.contains("Aerobatic"), "{preview}");
        assert!(!preview.contains("Horizon"), "{preview}");
    }

    #[test]
    fn li2t_and_il10_are_listed() {
        assert!(catalog().for_script("li2t.txt").unwrap().has_mods());
        assert!(catalog().for_script("il10.txt").unwrap().has_payloads());
        assert!(catalog().for_script("tu2.txt").unwrap().has_payloads());
        assert!(catalog().for_script("f84e.txt").unwrap().payloads.len() > 40);
        assert!(catalog().for_script("f86a5.txt").unwrap().payloads.len() >= 12);
    }

    #[test]
    fn f51d_has_payloads_and_modifications() {
        let ac = catalog().for_script("f51d.txt").expect("F-51D");
        assert!(ac.has_payloads());
        assert!(ac.has_mods());
        assert!(ac.payload(11).unwrap().summary().contains("HVAR"));
        let preview = mods_preview("f51d.txt", "10101");
        assert!(preview.contains("Radar"), "{preview}");
        assert!(preview.contains("Fuel") || preview.contains("150"), "{preview}");
        assert_eq!(payload_preview("f51d.txt", 0), "Clean");
    }
}
