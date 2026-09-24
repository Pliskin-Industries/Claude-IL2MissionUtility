//! ui_tests.rs — end-to-end UI tests on a headless `egui::Context`
//!
//! A `Harness` runs real frames of `GroupGeneratorApp::ui`, reads the
//! AccessKit tree egui builds each frame to find widgets by label, and
//! clicks, drags and types through synthetic input events. File dialogs are
//! answered from a queue (`dialog::answer`), so Load… / Generate paths run
//! exactly as in the app, writing into a temp folder. Generation logic has
//! its own unit tests in each module; these check that the UI drives it.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use eframe::egui::accesskit::{self as ak, Role};
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

use super::*;

/// Test stand-in for `rfd::FileDialog`: every pick pops the next prepared
/// answer. A dialog with no prepared answer is a test bug and panics.
pub mod dialog {
    use super::*;

    thread_local! {
        static ANSWERS: RefCell<VecDeque<Vec<PathBuf>>> = const { RefCell::new(VecDeque::new()) };
    }

    /// Queue the paths the next dialog returns (one path for single picks).
    pub fn answer(paths: Vec<PathBuf>) {
        ANSWERS.with(|a| a.borrow_mut().push_back(paths));
    }

    pub fn pending() -> usize {
        ANSWERS.with(|a| a.borrow().len())
    }

    fn next() -> Vec<PathBuf> {
        ANSWERS.with(|a| a.borrow_mut().pop_front()).expect("a file dialog opened with no prepared answer")
    }

    pub struct FileDialog;

    impl FileDialog {
        pub fn new() -> Self {
            FileDialog
        }
        pub fn add_filter(self, _name: impl Into<String>, _extensions: &[impl ToString]) -> Self {
            self
        }
        pub fn set_file_name(self, _name: impl Into<String>) -> Self {
            self
        }
        pub fn pick_file(self) -> Option<PathBuf> {
            next().into_iter().next()
        }
        pub fn pick_files(self) -> Option<Vec<PathBuf>> {
            Some(next())
        }
        pub fn save_file(self) -> Option<PathBuf> {
            next().into_iter().next()
        }
        pub fn pick_folder(self) -> Option<PathBuf> {
            next().into_iter().next()
        }
    }
}

const SCREEN: Vec2 = Vec2::new(1400.0, 1000.0);
const CTRL: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

#[derive(Clone, Debug)]
struct Node {
    label: String,
    role: Role,
    rect: Rect,
    disabled: bool,
    toggled: Option<bool>,
}

struct Harness {
    ctx: egui::Context,
    app: GroupGeneratorApp,
    nodes: Vec<Node>,
    time: f64,
    mods: Modifiers,
    pointer: Pos2,
    dir: PathBuf,
}

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

impl Harness {
    /// A fresh app with every output folder and the height store in a temp dir.
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("il2_ui_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        theme::apply(&ctx);
        let mut app = GroupGeneratorApp::default();
        app.terrain_store_path = dir.join("store").join("korea_100m.hgt");
        app.harvest_db_dir = dir.join("airfields").display().to_string();
        app.harvest_missions_dir = dir.join("missions").display().to_string();
        app.mark_saved(AppMode::Template);
        app.mark_saved(AppMode::Map);
        let mut h = Harness { ctx, app, nodes: Vec::new(), time: 0.0, mods: Modifiers::NONE, pointer: Pos2::ZERO, dir };
        h.settle();
        h
    }

    fn step(&mut self, events: Vec<Event>) {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            time: Some(self.time),
            modifiers: self.mods,
            events,
            // A real GPU allows large textures; the high-resolution map needs one.
            max_texture_side: Some(16_384),
            ..Default::default()
        };
        self.time += 1.0 / 60.0;
        let app = &mut self.app;
        let out = self.ctx.run(raw, |ctx| app.ui(ctx));
        if let Some(update) = out.platform_output.accesskit_update {
            self.nodes = update.nodes.iter().filter_map(|(_, n)| to_node(n)).collect();
        }
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.step(Vec::new());
        }
    }

    fn all(&self, label: &str) -> Vec<Node> {
        self.nodes.iter().filter(|n| n.label == label).cloned().collect()
    }

    fn find(&self, label: &str) -> Node {
        let hits = self.all(label);
        assert!(!hits.is_empty(), "no widget labelled {label:?}. Labels: {:?}", self.labels());
        hits[0].clone()
    }

    fn find_prefix(&self, prefix: &str) -> Node {
        self.nodes
            .iter()
            .find(|n| n.label.starts_with(prefix))
            .cloned()
            .unwrap_or_else(|| panic!("no widget starting {prefix:?}. Labels: {:?}", self.labels()))
    }

    fn has(&self, label: &str) -> bool {
        self.nodes.iter().any(|n| n.label == label)
    }

    fn labels(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.label.clone()).filter(|l| !l.is_empty()).collect()
    }

    fn move_to(&mut self, pos: Pos2) {
        self.pointer = pos;
        self.step(vec![Event::PointerMoved(pos)]);
    }

    fn click_at(&mut self, pos: Pos2) {
        self.click_at_with(pos, PointerButton::Primary);
    }

    fn click_at_with(&mut self, pos: Pos2, button: PointerButton) {
        self.move_to(pos);
        let mods = self.mods;
        self.step(vec![Event::PointerButton { pos, button, pressed: true, modifiers: mods }]);
        self.step(vec![Event::PointerButton { pos, button, pressed: false, modifiers: mods }]);
        self.settle();
    }

    fn click(&mut self, label: &str) {
        let n = self.find(label);
        assert!(!n.disabled, "{label:?} is disabled");
        self.click_at(n.rect.center());
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        self.move_to(from);
        let mods = self.mods;
        self.step(vec![Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: mods }]);
        for k in 1..=8 {
            let p = from + (to - from) * (k as f32 / 8.0);
            self.move_to(p);
        }
        self.step(vec![Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: mods }]);
        self.settle();
    }

    fn key_with(&mut self, key: Key, modifiers: Modifiers) {
        let before = self.mods;
        self.mods = modifiers;
        self.step(vec![Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]);
        self.step(vec![Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers }]);
        self.mods = before;
        self.settle();
    }

    fn key(&mut self, key: Key) {
        self.key_with(key, Modifiers::NONE);
    }

    fn tab(&mut self, label: &str) {
        self.click(label);
    }

    /// Step until the map textures are loaded (they decode on a thread).
    fn wait_for_map(&mut self) {
        for _ in 0..2000 {
            if self.app.map_lo_tex.is_some() {
                self.settle();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            self.step(Vec::new());
        }
        panic!("map texture never loaded");
    }

    fn status(&self) -> String {
        self.app.status_text()
    }

    fn out(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

fn to_node(n: &ak::Node) -> Option<Node> {
    let r = n.bounds()?;
    Some(Node {
        // Buttons carry their text as the label; plain labels and links as the value.
        label: n.label().or_else(|| n.value()).unwrap_or_default().to_string(),
        role: n.role(),
        rect: Rect::from_min_max(Pos2::new(r.x0 as f32, r.y0 as f32), Pos2::new(r.x1 as f32, r.y1 as f32)),
        disabled: n.is_disabled(),
        toggled: n.toggled().map(|t| t == ak::Toggled::True),
    })
}

fn assert_group_file(path: &Path) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(text.len() > 100, "{} is nearly empty", path.display());
    crate::parser::parse_group_file(&text).unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()));
}

// ── Shell ─────────────────────────────────────────────────────────────────

#[test]
fn rail_and_ctrl_numbers_switch_every_tab() {
    let mut h = Harness::new("rail");
    for (mode, label) in MODES {
        h.tab(label);
        assert!(h.app.mode == mode, "rail {label}");
    }
    for (i, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6].into_iter().enumerate() {
        h.key_with(key, CTRL);
        assert!(h.app.mode == MODES[i].0, "Ctrl {}", i + 1);
    }
}

#[test]
fn f1_and_rail_help_open_the_help_for_the_tab() {
    let mut h = Harness::new("help");
    h.tab("Exclusive Activation");
    h.key(Key::F1);
    assert!(h.app.help_open);
    assert!(h.app.help_topic == HelpTopic::Exclusive);
    h.app.help_open = false;
    h.tab("Map");
    h.click("Help");
    assert!(h.app.help_open && h.app.help_topic == HelpTopic::Front);
}

#[test]
fn every_tab_has_a_primary_button_with_its_shortcut() {
    let mut h = Harness::new("primary");
    for (mode, label) in MODES {
        h.tab(label);
        let primary = if mode == AppMode::Map { "Generate Base Map" } else { "Generate File" };
        let n = h.find(primary);
        assert!(n.rect.height() >= 28.0, "{label}: primary button {} px tall", n.rect.height());
    }
}

// ── Template ──────────────────────────────────────────────────────────────

/// "+ Add" sits at the right end of each model row.
fn add_model(h: &mut Harness, model: &str) {
    let row = h.find(&format!("Model {model}"));
    h.click_at(Pos2::new(row.rect.right() - 20.0, row.rect.center().y));
}

#[test]
fn template_add_select_remove_undo_reset_and_generate() {
    let mut h = Harness::new("template");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    add_model(&mut h, "F-51D");
    add_model(&mut h, "Il-10");
    assert_eq!(h.app.tpl_seats.len(), 3);
    assert_eq!(h.app.tpl_seats[2].unit.label(), "Il-10");

    // Select card 1: the right panel shows the unit.
    h.click("Unit card 1");
    assert!(matches!(h.app.tpl_select, Some(TplSelect::Seat(0))));
    assert!(h.has("SELECTED · UNIT 1"), "{:?}", h.labels());

    // + Order ▾ → Goto WP adds an order to the selected unit.
    let orders = h.app.tpl_seats[0].orders.len();
    h.click("+ Order ▾");
    h.click("Goto WP");
    assert_eq!(h.app.tpl_seats[0].orders.len(), orders + 1);

    // Remove card 2 with its ×, then Ctrl Z brings it back identical.
    let before = h.app.tpl_fingerprint();
    let card = h.find("Unit card 2").rect;
    let x = h.all("×").into_iter().find(|n| card.contains(n.rect.center())).expect("× on card 2");
    h.click_at(x.rect.center());
    assert_eq!(h.app.tpl_seats.len(), 2);
    assert!(h.status().is_empty() || !h.status().contains("Undone"));
    assert!(h.has("Undo"), "status bar offers Undo");
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 3);
    assert_eq!(h.app.tpl_fingerprint(), before);

    // Reset asks first; Esc cancels; confirming clears; Ctrl Z restores.
    h.click("Reset");
    assert!(h.app.confirm == Some(Confirm::ResetTemplate));
    h.key(Key::Escape);
    assert!(h.app.confirm.is_none());
    assert_eq!(h.app.tpl_seats.len(), 3);
    h.click("Reset");
    let dialog_reset = h.all("Reset").into_iter().max_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).unwrap();
    h.click_at(dialog_reset.rect.center());
    assert!(h.app.tpl_seats.is_empty());
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 3);

    // Ctrl G writes the file the Generate button writes.
    let out = h.out("tpl.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
    assert!(!h.app.is_dirty(AppMode::Template), "Generate marks the template saved");
    assert_eq!(dialog::pending(), 0);
}

#[test]
fn template_cards_reorder_by_drag() {
    let mut h = Harness::new("drag");
    h.tab("Template");
    for m in ["F-51D", "Il-10", "B-29 (simple)"] {
        add_model(&mut h, m);
    }
    let c3 = h.find("Unit card 3").rect;
    let c1 = h.find("Unit card 1").rect;
    h.drag(c3.center(), Pos2::new(c1.center().x, c1.top() + 4.0));
    let order: Vec<&str> = h.app.tpl_seats.iter().map(|s| s.unit.label()).collect();
    assert_eq!(order, ["B-29 (simple)", "F-51D", "Il-10"]);
}

#[test]
fn template_load_over_edits_asks_first() {
    let mut h = Harness::new("load");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click("Load…");
    assert!(h.app.confirm == Some(Confirm::LoadTemplate), "unsaved edits ask before Load");
    let file = repo("TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group");
    dialog::answer(vec![file]);
    let confirm = h.all("Load…").into_iter().max_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).unwrap();
    h.click_at(confirm.rect.center());
    assert_eq!(h.app.tpl_seats.len(), 4, "{}", h.status());
    assert!(!h.app.is_dirty(AppMode::Template));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 1, "Ctrl Z undoes the load");
}

// ── Army Generator ────────────────────────────────────────────────────────

/// A Template Builder vehicle group: the input the Army Generator expects.
fn write_ground_template(path: &Path, name: &str) {
    let unit = crate::template::bundled_catalog()
        .into_iter()
        .find(|u| u.kind == CatalogKind::Vehicle)
        .expect("a catalog vehicle");
    let mut seats = Vec::new();
    for _ in 0..3 {
        append_seat(&mut seats, unit.clone(), 4);
    }
    let opts = TemplateOptions {
        name: name.into(),
        seats,
        zone_in: 10_000.0,
        zone_out: 19_000.0,
        ..TemplateOptions::default()
    };
    let root = crate::template::generate_template(&opts).expect("generate ground template");
    std::fs::write(path, serialize_group(&root)).unwrap();
}

#[test]
fn army_add_retype_remove_undo_and_generate() {
    let mut h = Harness::new("army");
    h.tab("Army Generator");
    let (a, b) = (h.out("armor_a.Group"), h.out("armor_b.Group"));
    write_ground_template(&a, "Armor A");
    write_ground_template(&b, "Armor B");
    dialog::answer(vec![a, b]);
    h.click("Add templates…");
    assert_eq!(h.app.recon_slots.len(), 2, "{}", h.status());
    assert!(h.has("COPY MIX"), "center shows the copy mix");

    // Type buttons on card 1 (left panel; the right panel has the import picker).
    let supply = h
        .all("Supply")
        .into_iter()
        .filter(|n| n.role == Role::Button)
        .min_by(|a, b| a.rect.left().total_cmp(&b.rect.left()).then(a.rect.top().total_cmp(&b.rect.top())))
        .expect("Supply type button");
    h.click_at(supply.rect.center());
    assert!(h.app.recon_slots[0].kind == UnitKind::Supply);

    let remove = h.all("Remove")[0].clone();
    h.click_at(remove.rect.center());
    assert_eq!(h.app.recon_slots.len(), 1);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.recon_slots.len(), 2);

    let out = h.out("army.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
}

#[test]
fn army_rework_detects_copies() {
    let mut h = Harness::new("rework");
    h.tab("Army Generator");
    h.click("Rework existing");
    assert!(h.app.recon_submode == ReconSubmode::Rework);
    dialog::answer(vec![repo("TemplateExamples/Random_Ground_Units_22of72.Group")]);
    h.click("Add packs…");
    assert!(!h.app.recon_rework.is_empty(), "{}", h.status());
    let detected: usize = h.app.recon_rework.iter().filter_map(|s| s.detected).sum();
    assert_eq!(detected, 72);
    let out = h.out("rework.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
}

// ── Fighter Pack ──────────────────────────────────────────────────────────

#[test]
fn fighter_types_reset_confirm_and_generate() {
    let mut h = Harness::new("fighter");
    h.tab("Fighter Pack");
    let yak = AIRCRAFT_TYPES.iter().position(|a| a.label == "Yak-9P").unwrap();
    assert!(!h.app.type_enabled[yak]);
    h.click("Yak-9P");
    assert!(h.app.type_enabled[yak]);
    assert!(h.has("3 of Group 1 flights") || h.has("GROUP 1 · FLIGHTS"), "preview renders");

    h.click("Reset");
    assert!(h.app.confirm == Some(Confirm::ResetFighter));
    let dialog_reset = h.all("Reset").into_iter().max_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).unwrap();
    h.click_at(dialog_reset.rect.center());
    assert!(!h.app.type_enabled[yak], "Reset restores the default types");

    let out = h.out("fighter.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
}

// ── Exclusive Activation ──────────────────────────────────────────────────

#[test]
fn exclusive_plans_select_remove_undo_and_generate() {
    let mut h = Harness::new("exclusive");
    h.tab("Exclusive Activation");
    dialog::answer(vec![repo("TemplateExamples/Exclusive_Activation_6plan.Group")]);
    h.click("Add templates…");
    assert_eq!(h.app.bomber_slots.len(), 6, "{}", h.status());
    h.click("Plan card 3");
    assert_eq!(h.app.bomber_selected, Some(2));
    h.click("Sequence 5");
    assert_eq!(h.app.bomber_selected, Some(4));
    h.click("Remove");
    assert_eq!(h.app.bomber_slots.len(), 5);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.bomber_slots.len(), 6);
    let out = h.out("exclusive.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
}

// ── Airfield ──────────────────────────────────────────────────────────────

#[test]
fn airfield_load_side_and_generate() {
    let mut h = Harness::new("airfield");
    h.tab("Airfield");
    assert!(h.find("Generate File").disabled, "Generate waits for a file");
    dialog::answer(vec![repo("TemplateExamples/Airfield_Mess.Group")]);
    h.click("Load airfield…");
    assert!(h.app.airfield_info.is_some(), "{}", h.status());
    assert!(h.has("KEPT"));
    h.click("DPRK [1]");
    assert!(!h.app.airfield_western);
    assert!(h.has("RELINKED TO DPRK [1]"));
    let out = h.out("airfield.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
}

#[test]
fn airfield_harvest_file_and_watch() {
    let mut h = Harness::new("harvest");
    h.tab("Airfield");
    // Harvest a file… (the K13 package stands in for a _gen.mission).
    dialog::answer(vec![repo("TemplateExamples/K13 AFB_mp.Group")]);
    h.click("Harvest a file…");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    assert!(db.join("catalog.Group").is_file(), "{}", h.status());
    assert!(h.has("DATABASE HARVEST"), "the log shows in the center");
    let logged = h.app.harvest_log.len();

    // Watch: a new _gen.mission in the Missions folder is harvested by itself.
    let missions = PathBuf::from(&h.app.harvest_missions_dir);
    std::fs::create_dir_all(&missions).unwrap();
    h.click("Watch for new airfields");
    assert!(h.app.harvest_watcher.is_some(), "{}", h.status());
    std::fs::copy(repo("TemplateExamples/K13 AFB_mp.Group"), missions.join("_gen.mission")).unwrap();
    for _ in 0..400 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step(Vec::new());
        if h.app.harvest_log.len() > logged {
            break;
        }
    }
    assert!(h.app.harvest_log.len() > logged, "watcher harvested the new file: {}", h.status());
    h.click("Watch for new airfields");
    assert!(h.app.harvest_watcher.is_none());
}

// ── Map ───────────────────────────────────────────────────────────────────

#[test]
fn map_tools_keys_escape_one_shot_objectives_and_tab_reset() {
    let mut h = Harness::new("maptools");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    for (i, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6].into_iter().enumerate() {
        h.key(key);
        assert!(h.app.map_drawing_mode == MAP_TOOLS[i].0, "key {}", i + 1);
    }
    // Esc drops a half-drawn salient and returns to Select.
    h.key(Key::Num3);
    h.click_at(map.center());
    h.click_at(map.center() + Vec2::new(30.0, 10.0));
    assert!(!h.app.current_salient.is_empty(), "salient in progress");
    assert!(h.status().is_empty() || h.has("SALIENT"), "banner shows");
    h.key(Key::Escape);
    assert!(h.app.current_salient.is_empty());
    assert!(h.app.map_drawing_mode == MapDrawingMode::None);

    // An objective tool places one, then returns to Select; Shift keeps it.
    h.key(Key::Num5);
    h.click_at(map.center());
    assert_eq!(h.app.east_objectives.len(), 1);
    assert!(h.app.map_drawing_mode == MapDrawingMode::None);
    h.key(Key::Num6);
    h.mods = Modifiers::SHIFT;
    h.click_at(map.center() + Vec2::new(40.0, 0.0));
    h.click_at(map.center() + Vec2::new(60.0, 0.0));
    h.mods = Modifiers::NONE;
    assert_eq!(h.app.nato_objectives.len(), 2);
    assert!(h.app.map_drawing_mode == MapDrawingMode::PlaceNatoObjective);

    // Leaving the tab puts the tool down.
    h.tab("Template");
    h.tab("Map");
    assert!(h.app.map_drawing_mode == MapDrawingMode::None);
}

#[test]
fn map_arrow_undo_redo_and_clear_undo() {
    let mut h = Harness::new("mapundo");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    h.key(Key::Num4);
    h.drag(map.center(), map.center() + Vec2::new(120.0, -60.0));
    assert_eq!(h.app.attack_arrows.len(), 1, "drag draws an attack arrow");
    h.key_with(Key::Z, CTRL);
    assert!(h.app.attack_arrows.is_empty());
    h.key_with(Key::Y, CTRL);
    assert_eq!(h.app.attack_arrows.len(), 1);

    // Clear objectives from the Forces dock, then Ctrl Z restores them.
    h.key(Key::Num5);
    h.click_at(map.center());
    h.click("Forces");
    let clear = h.all("Clear").into_iter().find(|n| !n.disabled && n.rect.top() > h.find("OBJECTIVES").rect.top()).expect("objectives Clear");
    h.click_at(clear.rect.center());
    assert!(h.app.east_objectives.is_empty());
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.east_objectives.len(), 1);
}

#[test]
fn map_generate_and_load_base_map_round_trip() {
    let mut h = Harness::new("basemap");
    h.tab("Map");
    h.wait_for_map();
    let out = h.out("base.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
    assert!(!h.app.is_dirty(AppMode::Map));
    // Nothing changed since: Load does not ask.
    dialog::answer(vec![out]);
    h.click("Load base map…");
    assert!(h.app.confirm.is_none(), "{}", h.status());
    assert_eq!(dialog::pending(), 0);
}

#[test]
fn map_terrain_import_layers_and_export() {
    let mut h = Harness::new("terrain");
    h.tab("Map");
    h.wait_for_map();
    h.click("Terrain");
    assert!(h.has_prefix("0 of "), "empty store: {:?}", h.labels());

    // A snapped probe file: nine land nodes of one tile at 100–108 m.
    let nodes: Vec<(usize, usize)> = heightprobe::tile_probe_nodes(10, 10).into_iter().take(9).collect();
    let mut group = heightprobe::probe_group("snapped", &nodes).unwrap();
    for (k, child) in group.children.iter_mut().enumerate() {
        child.set_property("YPos", format!("{:.3}", 100.0 + k as f64));
    }
    let snapped = h.out("snapped.Group");
    std::fs::write(&snapped, serialize_group(&group)).unwrap();
    dialog::answer(vec![snapped]);
    h.click("Import snapped…");
    assert_eq!(h.app.terrain_store.as_ref().unwrap().measured_nodes(), 9, "{}", h.status());
    assert!(h.app.terrain_store_path.is_file(), "the store is saved");
    let (i, j) = nodes[4];
    let y = h.app.terrain_store.as_ref().unwrap().node(i, j).unwrap();
    assert!((y - 104.0).abs() < 0.05);

    // An unsnapped file (all probes at 0 m) is refused.
    let raw = h.out("unsnapped.Group");
    std::fs::write(&raw, serialize_group(&heightprobe::tile_probe_group(11, 4).unwrap().unwrap())).unwrap();
    dialog::answer(vec![raw]);
    h.click("Import snapped…");
    assert!(h.status().contains("not merged"), "{}", h.status());

    h.click("Relief");
    assert!(h.app.terrain_relief.is_some(), "relief texture built");
    h.click("Coverage");
    assert!(h.app.terrain_show_coverage);
    assert_eq!(h.find("Coverage").toggled, Some(true), "the checkbox reports checked");

    // Export the tiles under a small AO in the middle of tile 10_10.
    let (i_lo, i_hi, j_lo, j_hi) = crate::terrain::tile_node_range(10, 10);
    let (x, z) = ((i_lo + i_hi) as f64 * 50.0, (j_lo + j_hi) as f64 * 50.0);
    h.app.front_aabb = WorldAabb::from_corners(x - 500.0, z - 500.0, x + 500.0, z + 500.0);
    h.settle();
    let tiles = h.out("tiles");
    std::fs::create_dir_all(&tiles).unwrap();
    dialog::answer(vec![tiles.clone()]);
    let button = h.find_prefix("Export AO tiles (");
    h.click_at(button.rect.center());
    let written: Vec<_> = std::fs::read_dir(&tiles).unwrap().collect();
    assert_eq!(written.len(), 1, "{}", h.status());
    assert!(tiles.join(format!("{}.Group", heightprobe::tile_name(10, 10))).is_file());
}

#[test]
fn map_dock_tabs_all_render() {
    let mut h = Harness::new("dock");
    h.tab("Map");
    for (tab, marker) in [("Forces", "FIGHTERS"), ("References 0", "Add reference groups…"), ("Terrain", "HEIGHT STORE"), ("Period", "DRAWN MARKS")] {
        h.click(tab);
        assert!(h.has(marker), "{tab}: {:?}", h.labels());
    }
}

// ── Sizes (README §6.6) ───────────────────────────────────────────────────

#[test]
fn interactive_widgets_are_at_least_28_px_tall() {
    let mut h = Harness::new("sizes");
    let mut small = Vec::new();
    let mut checked = 0;
    for (_, label) in MODES {
        h.tab(label);
        for n in &h.nodes {
            let interactive = matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton);
            checked += usize::from(interactive);
            if interactive && !n.disabled && n.rect.height() < 27.5 && n.rect.width() > 0.0 {
                small.push(format!("{label}: {:?} {:?} {}px", n.label, n.role, n.rect.height()));
            }
        }
    }
    assert!(checked > 150, "only {checked} interactive widgets seen: the role filter is wrong");
    small.sort();
    small.dedup();
    assert!(small.is_empty(), "targets under 28 px:\n{}", small.join("\n"));
}

impl Harness {
    fn has_prefix(&self, prefix: &str) -> bool {
        self.nodes.iter().any(|n| n.label.starts_with(prefix))
    }
}

#[test]
fn template_tree_chip_selects_and_moves() {
    let mut h = Harness::new("tree");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click("+ Order ▾");
    h.click("Goto WP");
    h.click("+ Order ▾");
    h.click("AttackArea");
    let kinds = |h: &Harness| h.app.tpl_seats[0].orders.iter().map(|o| o.kind).collect::<Vec<_>>();
    let before = kinds(&h);
    // Select the Goto WP chip ("<n> Goto WP"), then move it right with its arrow.
    let gi = before.iter().position(|k| *k == OrderKind::GotoWaypoint).expect("a Goto WP order");
    assert!(gi + 1 < before.len(), "Goto WP has an order after it");
    let chip = h.find(&format!("{} Goto WP", gi + 1));
    h.click_at(chip.rect.center());
    assert!(matches!(h.app.tpl_select, Some(TplSelect::Order { seat: 0, order }) if order == gi), "{:?}", h.app.tpl_select.is_some());
    assert!(h.has(&format!("SELECTED · UNIT 1 · ORDER {}", gi + 1)));
    h.click("Move right");
    let after = kinds(&h);
    assert_eq!(after[gi], before[gi + 1]);
    assert_eq!(after[gi + 1], before[gi]);
}

#[test]
fn map_places_fighters_from_the_forces_dock() {
    let mut h = Harness::new("placefighters");
    h.tab("Map");
    h.wait_for_map();
    h.click("Forces");
    let place = h.all("Place DPRK")[0].clone();
    h.click_at(place.rect.center());
    let n = h.app.map_fighters.as_ref().map_or(0, |l| l.spots.len());
    assert!(n > 0, "no fighters placed: {}", h.status());
    assert!(h.has_prefix("DPRK: "), "count line shows: {:?}", h.labels());
    // Clear (the one in the Fighters section), then undo.
    let fighters_top = h.find("FIGHTERS").rect.top();
    let clear = h
        .all("Clear")
        .into_iter()
        .filter(|n| !n.disabled && n.rect.top() > fighters_top)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("fighters Clear");
    h.click_at(clear.rect.center());
    assert!(h.app.map_fighters.is_none());
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.map_fighters.as_ref().map_or(0, |l| l.spots.len()), n);
}

#[test]
fn map_load_over_unsaved_changes_asks_first() {
    let mut h = Harness::new("mapdirty");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    h.key(Key::Num5);
    h.click_at(map.center());
    assert!(h.app.is_dirty(AppMode::Map));
    h.click("Load base map…");
    assert!(h.app.confirm == Some(Confirm::LoadBaseMap));
    h.key(Key::Escape);
    assert!(h.app.confirm.is_none());
    assert_eq!(h.app.east_objectives.len(), 1, "cancel keeps the map");
}

#[test]
fn map_terrain_readout_under_the_pointer() {
    let mut h = Harness::new("readout");
    h.tab("Map");
    h.wait_for_map();
    h.click("Terrain");
    h.click("Coverage");
    let map = h.find("Korea map").rect;
    h.move_to(map.center());
    h.settle();
    assert_eq!(h.app.terrain_readout().as_deref(), Some("Ground not measured"));
    // Measure the ground under the pointer, then the readout shows it.
    let (x, z) = h.app.map_hover_xz.expect("pointer over the map");
    let store = h.app.terrain_store.as_mut().expect("store loaded");
    let (i, j) = ((x / 100.0).floor() as usize, (z / 100.0).floor() as usize);
    for (a, b) in [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)] {
        store.set_node(a, b, 321.0);
    }
    h.settle();
    assert_eq!(h.app.terrain_readout().as_deref(), Some("Ground 321 m · 100 m grid"));
}
