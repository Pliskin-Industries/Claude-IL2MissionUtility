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

use super::airfield::country_name;
use super::builder::{FIELD_LABEL_W, skill_name};
use super::map::{fighter_svg_north, rasterize_svg};
use super::*;
use crate::heightprobe;
use crate::template::{OrderKind, TemplateOptions, append_seat};

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
        ANSWERS
            .with(|a| a.borrow_mut().pop_front())
            .expect("a file dialog opened with no prepared answer")
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

const SCREEN: Vec2 = Vec2::new(1400.0, 1280.0);
const CTRL: Modifiers = Modifiers {
    alt: false,
    ctrl: true,
    shift: false,
    mac_cmd: false,
    command: true,
};

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
    screen: Vec2,
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
        app.terrain_use_builtin = false;
        app.harvest_db_dir = dir.join("airfields").display().to_string();
        app.harvest_missions_dir = dir.join("missions").display().to_string();
        app.map_template_dir = dir.join("template").display().to_string();
        app.mark_saved(AppMode::Template);
        app.mark_saved(AppMode::Map);
        let mut h = Harness { ctx, app, nodes: Vec::new(), time: 0.0, mods: Modifiers::NONE, pointer: Pos2::ZERO, screen: SCREEN, dir };
        h.settle();
        h
    }

    fn step(&mut self, events: Vec<Event>) {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
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
            self.nodes = update
                .nodes
                .iter()
                .filter_map(|(_, n)| to_node(n))
                .collect();
        }
    }

    fn settle(&mut self) {
        for _ in 0..4 {
            self.step(Vec::new());
        }
    }

    fn all(&self, label: &str) -> Vec<Node> {
        self.nodes
            .iter()
            .filter(|n| n.label == label)
            .cloned()
            .collect()
    }

    fn find(&self, label: &str) -> Node {
        let hits = self.all(label);
        assert!(
            !hits.is_empty(),
            "no widget labelled {label:?}. Labels: {:?}",
            self.labels()
        );
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
        self.nodes
            .iter()
            .map(|n| n.label.clone())
            .filter(|l| !l.is_empty())
            .collect()
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
        self.step(vec![Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: mods,
        }]);
        self.step(vec![Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: mods,
        }]);
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
        self.step(vec![Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: mods,
        }]);
        for k in 1..=8 {
            let p = from + (to - from) * (k as f32 / 8.0);
            self.move_to(p);
        }
        self.step(vec![Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: mods,
        }]);
        self.settle();
    }

    fn key_with(&mut self, key: Key, modifiers: Modifiers) {
        let before = self.mods;
        self.mods = modifiers;
        self.step(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.step(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }]);
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
        label: n
            .label()
            .or_else(|| n.value())
            .unwrap_or_default()
            .to_string(),
        role: n.role(),
        rect: Rect::from_min_max(
            Pos2::new(r.x0 as f32, r.y0 as f32),
            Pos2::new(r.x1 as f32, r.y1 as f32),
        ),
        disabled: n.is_disabled(),
        toggled: n.toggled().map(|t| t == ak::Toggled::True),
    })
}

fn assert_group_file(path: &Path) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(text.len() > 100, "{} is nearly empty", path.display());
    crate::parser::parse_group_file(&text)
        .unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()));
}

// ── Shell ─────────────────────────────────────────────────────────────────

#[test]
fn rail_and_ctrl_numbers_switch_every_tab() {
    let mut h = Harness::new("rail");
    for (mode, label) in MODES {
        h.tab(label);
        assert!(h.app.mode == mode, "rail {label}");
    }
    for (i, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7].into_iter().enumerate() {
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
    assert_eq!(MODES.len(), 7);
    for (mode, label) in MODES {
        h.tab(label);
        let primary = if mode == AppMode::Map {
            "Generate Base Map"
        } else {
            "Generate File"
        };
        let n = h.find(primary);
        assert!(
            n.rect.height() >= 28.0,
            "{label}: primary button {} px tall",
            n.rect.height()
        );
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

    // Remove unit 2 from the inspector, then Ctrl Z brings it back identical.
    let before = h.app.tpl_fingerprint();
    h.click("Unit card 2");
    h.click("Remove unit");
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
    let dialog_reset = h
        .all("Reset")
        .into_iter()
        .max_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .unwrap();
    h.click_at(dialog_reset.rect.center());
    assert!(h.app.tpl_seats.is_empty());
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 3);

    // Ctrl G writes the file the Generate button writes.
    let out = h.out("tpl.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
    assert!(
        !h.app.is_dirty(AppMode::Template),
        "Generate marks the template saved"
    );
    assert_eq!(dialog::pending(), 0);
}

#[test]
fn template_mission_menu_adds_a_cap() {
    let mut h = Harness::new("mission");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click("Unit card 1");
    h.click("+ Mission ▾");
    h.click("CAP");
    assert!(
        h.app.tpl_seats[0]
            .orders
            .iter()
            .any(|o| o.kind == crate::template::OrderKind::GotoWaypoint && o.loop_to == Some(1)),
        "CAP closes the diamond"
    );
    assert_eq!(h.app.tpl_seats[0].events.len(), 4);
    assert!(h.app.tpl_seats[0].events.iter().any(|e| {
        e.kind == crate::template::EntityEvent::OnKilled
            && e.then == crate::template::EventThen::AllDestroyed
    }));
    assert!(h.status().contains("CAP"));
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
    assert!(
        h.app.confirm == Some(Confirm::LoadTemplate),
        "unsaved edits ask before Load"
    );
    let file = repo("TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group");
    dialog::answer(vec![file]);
    let confirm = h
        .all("Load…")
        .into_iter()
        .max_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .unwrap();
    h.click_at(confirm.rect.center());
    assert_eq!(h.app.tpl_seats.len(), 4, "{}", h.status());
    assert!(!h.app.is_dirty(AppMode::Template));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 1, "Ctrl Z undoes the load");
}

#[test]
fn template_preview_shows_the_highlighted_model_before_add() {
    let mut h = Harness::new("tplpreview");
    h.tab("Template");
    assert!(h.app.tpl_seats.is_empty());
    let pick = h.app.displayed_catalog()[h.app.tpl_add_pick]
        .label()
        .to_string();
    assert!(
        h.has(&pick),
        "the highlighted model is named in the preview"
    );
    let specs = h.find_prefix("Type: ");
    let formation = h.find("Formation view");
    let models = h.find("MODELS");
    assert!(
        specs.rect.top() < models.rect.top(),
        "preview sits above Models"
    );
    assert!(
        specs.rect.right() < formation.rect.left(),
        "preview stays in the model panel"
    );
    assert!(
        formation.rect.width() > 400.0,
        "the formation view keeps the center"
    );
    assert!(
        h.has("Placement & Checkzones"),
        "place and spawn stay reachable before a unit is added"
    );
    assert!(h.has("Activate or Spawn"));
    let script = h.app.displayed_catalog()[h.app.tpl_add_pick].script.clone();
    let plain = super::builder::plain_class_line(crate::model_spec::class_for(&script));
    assert!(h.has(plain), "a plain description of the highlighted model");

    add_model(&mut h, "Il-10");
    let formation = h.find("Formation view");
    let kicker = h.find("SELECTED · UNIT 1");
    assert!(
        (kicker.rect.left() - (formation.rect.left() + 14.0)).abs() < 2.0,
        "unit options use the formation's 14 px left margin"
    );
    let about = h.find("A ground-attack aircraft. It strikes targets on the ground.");
    let models = h.find("MODELS");
    assert!(
        about.rect.top() < models.rect.top(),
        "the description stays above Models"
    );
    assert!(
        about.rect.bottom() < h.find("Unit card 1").rect.top(),
        "the order of battle does not repeat it"
    );
    assert!(h.has("Change Model"));
    assert!(
        h.has("Placement & Checkzones"),
        "selecting a unit does not hide place and spawn"
    );
    assert!(h.has("Activate or Spawn"));
}

#[test]
fn template_empty_view_names_the_first_step_and_adds_the_pick() {
    let mut h = Harness::new("tplempty");
    h.tab("Template");
    assert!(
        h.has("Pick a model on the left, then + Add to begin."),
        "{:?}",
        h.labels()
    );
    let pick = h.app.displayed_catalog()[h.app.tpl_add_pick]
        .label()
        .to_string();
    h.click(&format!("+ Add {pick}"));
    assert_eq!(h.app.tpl_seats.len(), 1);
    assert_eq!(h.app.tpl_seats[0].unit.label(), pick);
    assert!(!h.has("Pick a model on the left, then + Add to begin."));

    // A kind with no models offers to show the list instead.
    h.app.tpl_seats.clear();
    h.app
        .tpl_catalog
        .retain(|u| u.kind != CatalogKind::UserAdded);
    h.click("User Added");
    h.click("Show model list");
    assert!(
        !h.app.tpl_show_models,
        "the left panel consumed the request"
    );
    assert!(h.app.tpl_seats.is_empty());
}

#[test]
fn template_kind_cells_and_both_filters() {
    let mut h = Harness::new("tplkinds");
    h.tab("Template");
    // Two rows of equal cells: four, then three.
    let row1: Vec<Node> = ["Planes", "Vehicles", "Infantry", "Trains"]
        .iter()
        .map(|l| h.find(l))
        .collect();
    let row2: Vec<Node> = ["Ships", "Fixed Units", "User Added"]
        .iter()
        .map(|l| h.find(l))
        .collect();
    for row in [&row1, &row2] {
        for n in row.iter() {
            assert!(
                (n.rect.width() - row[0].rect.width()).abs() < 1.0,
                "{} is {} px",
                n.label,
                n.rect.width()
            );
            assert!((n.rect.top() - row[0].rect.top()).abs() < 0.5 && n.rect.height() >= 28.0);
        }
    }
    assert!(row2[0].rect.top() > row1[0].rect.top());
    h.click("Vehicles");
    assert!(h.app.tpl_kind == CatalogKind::Vehicle);
    h.click("Planes");

    // Planes have both a class and a country filter; the list obeys both.
    let countries = h.app.countries_for_kind(CatalogKind::Plane);
    assert!(countries.len() > 1 && h.app.classes_for_kind(CatalogKind::Plane).len() > 1);
    let panel_right = h.find("Planes").rect.left() + 270.0;
    let mut combos: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| n.role == Role::ComboBox && n.rect.left() < panel_right)
        .cloned()
        .collect();
    combos.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    assert_eq!(combos.len(), 2, "class and country combos");
    h.click_at(combos[1].rect.center());
    let country = countries[countries.len() - 1];
    h.click(&country_short(country));
    assert_eq!(h.app.tpl_country, Some(country));
    let shown = h.app.displayed_catalog();
    assert!(!shown.is_empty() && shown.iter().all(|u| u.country() == country));
    for u in &shown {
        assert!(h.has(&format!("Model {}", u.label())));
    }
    let hidden = h
        .app
        .tpl_catalog
        .iter()
        .find(|u| u.kind == CatalogKind::Plane && u.country() != country)
        .unwrap();
    assert!(
        !h.has(&format!("Model {}", hidden.label())),
        "{} is filtered out",
        hidden.label()
    );
}

#[test]
fn template_card_meta_tree_hint_and_selection_grid() {
    let mut h = Harness::new("tplmeta");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    let seat = &h.app.tpl_seats[0];
    let country = country_short(seat.country);
    let country = country
        .split_once(' ')
        .map_or(country.as_str(), |(_, n)| n.trim())
        .to_string();
    let k = seat.orders.len();
    let meta = format!(
        "{country} · {} · {k} {}",
        skill_name(seat.skill),
        if k == 1 { "order" } else { "orders" }
    );
    assert!(h.has(&meta), "{meta:?} in {:?}", h.labels());
    assert!(h.has("UNIT → OnSpawned → orders. ‹ › or drag a chip left and right."));
    assert!(h.has("Add to Template"));

    // Nothing selected: the inspector is the template settings, including Zone In.
    h.app.tpl_select = None;
    h.settle();
    assert!(h.has("Placement & Checkzones"), "{:?}", h.labels());
    assert!(
        h.has(&format!("{:.1} km", h.app.tpl_zone_in / 1000.0)),
        "{:?}",
        h.labels()
    );
    assert!(h.app.tpl_zone_in > 1000.0);
    assert!(!h.has("Change Model"));

    // Seat fields: labels in a 96 px column, controls to the right of it.
    h.click("Unit card 1");
    assert!(h.has("Change Model"));
    assert!(
        h.has("Placement & Checkzones"),
        "place and spawn stay open while a unit is selected"
    );
    assert!(h.has("Activate or Spawn"));
    let role = h.find("Role").rect;
    let country_label = h.find("Country").rect;
    assert!((role.left() - country_label.left()).abs() < 0.5);
    let role_combo = h
        .nodes
        .iter()
        .find(|n| n.role == Role::ComboBox && (n.rect.center().y - role.center().y).abs() < 8.0)
        .expect("Role combo on the Role row")
        .clone();
    assert!(
        (role_combo.rect.left() - role.left() - FIELD_LABEL_W - 8.0).abs() < 1.0,
        "control column starts after 96 px"
    );
}

#[test]
fn template_remove_order_selects_its_unit() {
    let mut h = Harness::new("tplrmorder");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click("+ Order ▾");
    h.click("Goto WP");
    h.click("+ Order ▾");
    h.click("AttackArea");
    let before = h.app.tpl_seats[0].orders.len();
    let gi = h.app.tpl_seats[0]
        .orders
        .iter()
        .position(|o| o.kind == OrderKind::GotoWaypoint)
        .unwrap();
    assert!(gi + 1 < before, "an order follows Goto WP");
    let wp = h.app.tpl_seats[0].orders[gi].waypoint.max(1);
    h.click_at(h.find(&format!("{} Goto WP {wp}", gi + 1)).rect.center());
    h.click("Remove order");
    assert_eq!(h.app.tpl_seats[0].orders.len(), before - 1);
    assert!(
        matches!(h.app.tpl_select, Some(TplSelect::Seat(0))),
        "the unit is selected, not a neighbouring order"
    );
    assert!(h.has("SELECTED · UNIT 1"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats[0].orders.len(), before);
}

#[test]
fn template_copy_attributes_needs_a_unit_and_undoes() {
    let mut h = Harness::new("tplcopy");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    add_model(&mut h, "Il-10");
    h.app.tpl_select = None;
    h.settle();
    assert!(
        !h.has("Copy attributes to all"),
        "copy belongs to the selected unit"
    );
    assert!(h.has("Placement & Checkzones"));

    h.app.tpl_seats[0].skill = 4;
    h.app.tpl_seats[1].skill = 1;
    h.click("Unit card 1");
    h.click("Copy attributes to all");
    assert_eq!(h.app.tpl_seats[1].skill, 4);
    assert_eq!(h.app.undo_label(), Some("Copied attributes from F-51D"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats[1].skill, 1);
}

#[test]
fn template_unit_menu_duplicates_and_deletes() {
    let mut h = Harness::new("tplmenu");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click_at_with(
        h.find("Unit card 1").rect.center(),
        PointerButton::Secondary,
    );
    h.click("Duplicate");
    assert_eq!(h.app.tpl_seats.len(), 2);
    assert_eq!(h.app.tpl_seats[1].unit.label(), "F-51D");
    h.click_at_with(
        h.find("Unit card 2").rect.center(),
        PointerButton::Secondary,
    );
    h.click("Delete");
    assert_eq!(h.app.tpl_seats.len(), 1);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.tpl_seats.len(), 2);
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
        .min_by(|a, b| {
            a.rect
                .left()
                .total_cmp(&b.rect.left())
                .then(a.rect.top().total_cmp(&b.rect.top()))
        })
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
    dialog::answer(vec![repo(
        "TemplateExamples/Random_Ground_Units_22of72.Group",
    )]);
    h.click("Add packs…");
    assert!(!h.app.recon_rework.is_empty(), "{}", h.status());
    let detected: usize = h.app.recon_rework.iter().filter_map(|s| s.detected).sum();
    assert_eq!(detected, 72);
    assert!(h.has("COPY MIX") && h.has("On map"));
    // The same randomizer label in both submodes.
    h.click("Spawn all copies (no randomizer)");
    assert!(h.app.recon_strip_randomizer);
    h.click("Spawn all copies (no randomizer)");
    let out = h.out("rework.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_group_file(&out);
}

#[test]
fn army_cards_mix_block_import_row_and_settings() {
    let mut h = Harness::new("armylayout");
    h.tab("Army Generator");
    let (a, b) = (h.out("armor_a.Group"), h.out("armor_b.Group"));
    write_ground_template(&a, "Armor A");
    write_ground_template(&b, "Armor B");
    dialog::answer(vec![a, b]);
    h.click("Add templates…");

    // Type buttons are 36 × 28; the import picker is one row of six.
    let kinds = ["Ship", "Armor", "Supply", "Artillery", "Infantry", "Train"];
    for label in kinds {
        let buttons: Vec<Node> = h
            .all(label)
            .into_iter()
            .filter(|n| n.role == Role::Button)
            .collect();
        assert_eq!(buttons.len(), 3, "{label}: two cards and the import picker");
        for n in &buttons {
            assert!(
                (n.rect.width() - 36.0).abs() < 0.5 && (n.rect.height() - 28.0).abs() < 0.5,
                "{label} is {:?}",
                n.rect.size()
            );
        }
    }
    let picker = |h: &Harness, label: &str| {
        h.all(label)
            .into_iter()
            .filter(|n| n.role == Role::Button)
            .max_by(|a, b| a.rect.left().total_cmp(&b.rect.left()))
            .unwrap()
    };
    let top = picker(&h, "Ship").rect.top();
    assert!(
        kinds
            .iter()
            .all(|k| (picker(&h, k).rect.top() - top).abs() < 0.5),
        "one row"
    );
    h.click_at(picker(&h, "Train").rect.center());
    assert!(h.app.recon_import_kind == UnitKind::Train);

    // Card meta line: file and counts, zero counts left out.
    let info = &h.app.recon_slots[0].info;
    assert_eq!(info.ship_count + info.train_count + info.block_count, 0);
    assert!(
        h.has(&format!("armor_a.Group · {} vehicles", info.vehicle_count)),
        "{:?}",
        h.labels()
    );
    assert!(h.has("Zone In checkzones"));
    assert!(
        !h.labels()
            .iter()
            .any(|l| l.contains("confirm the group is valid"))
    );

    // COPY MIX is its own block below the parking grid.
    let grid = h.find_prefix("PARKING GRID").rect;
    let mix = h.find("COPY MIX").rect;
    assert!(
        mix.top() > grid.bottom() + 40.0,
        "COPY MIX sits at the bottom"
    );

    // Activate ratio shows a percent value.
    assert!(h.has("Activate ratio"));
    assert!(
        h.has(&format!("{}%", h.app.recon_percent)),
        "{:?}",
        h.labels()
    );

    // Timing stays visible with spawn-all on; its sliders are disabled.
    h.click("Timing");
    h.click("Spawn all copies (no randomizer)");
    assert!(h.app.recon_strip_randomizer);
    assert!(h.has(&format!(
        "Start {} s · {} ms apart",
        h.app.recon_start_delay_s, h.app.recon_group_delay_ms
    )));
    assert!(h.has("Timing applies only with the randomizer."));
    let timing = h.find("Timing").rect;
    let sliders: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| {
            n.role == Role::Slider
                && n.rect.top() > timing.top()
                && n.rect.left() >= timing.left() - 20.0
        })
        .cloned()
        .collect();
    assert_eq!(sliders.len(), 2, "start delay and group delay");
    assert!(sliders.iter().all(|n| n.disabled));
}

#[test]
fn recon_counts_line_omits_zero_counts() {
    let info = |vehicle_count, ship_count, train_count, block_count| UnitPlanInfo {
        name: String::new(),
        vehicle_count,
        ship_count,
        train_count,
        block_count,
        checkzones: Vec::new(),
        suggested_triggers: Vec::new(),
        restore_starts: Vec::new(),
        weapon_range_m: None,
        route: None,
        wp_ahead: Vec::new(),
    };
    assert_eq!(recon_counts_line(&info(6, 0, 0, 0)), "6 vehicles");
    assert_eq!(
        recon_counts_line(&info(3, 0, 1, 2)),
        "2 vehicles, 1 train, 2 blocks"
    );
    assert_eq!(recon_counts_line(&info(1, 0, 1, 0)), "1 train");
    assert_eq!(recon_counts_line(&info(2, 2, 0, 1)), "2 ships, 1 block");
    assert_eq!(recon_counts_line(&info(0, 0, 0, 0)), "no units");
}

// ── Fighter Pack ──────────────────────────────────────────────────────────

#[test]
fn fighter_types_reset_confirm_and_generate() {
    let mut h = Harness::new("fighter");
    h.tab("Fighter Pack");
    let yak = AIRCRAFT_TYPES
        .iter()
        .position(|a| a.label == "Yak-9P")
        .unwrap();
    assert!(!h.app.type_enabled[yak]);
    h.click("Yak-9P");
    assert!(h.app.type_enabled[yak]);
    assert!(
        h.has("3 of Group 1 flights") || h.has("GROUP 1 · FLIGHTS"),
        "preview renders"
    );

    h.click("Reset");
    assert!(h.app.confirm == Some(Confirm::ResetFighter));
    let dialog_reset = h
        .all("Reset")
        .into_iter()
        .max_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .unwrap();
    h.click_at(dialog_reset.rect.center());
    assert!(!h.app.type_enabled[yak], "Reset restores the default types");

    let out = h.out("fighter.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
}

#[test]
fn fighter_preview_sentence_table_and_empty_state() {
    let mut h = Harness::new("fighter_preview");
    h.tab("Fighter Pack");
    h.app.linked_groups = 3;
    h.app.flight_count = 3;
    h.app.max_in_flight = 4;
    h.settle();
    assert!(
        h.has("Group logic is built in. 3 linked groups, chained through NodeGates, parked on a 10 km grid from 40000, 40000."),
        "{:?}",
        h.labels()
    );
    // Flights 4/3/2: colour + number, roles counted from the preview seats.
    for name in ["Red 11", "Blue 21", "Yellow 31"] {
        assert!(h.has(name), "flight {name}: {:?}", h.labels());
    }
    assert!(h.has("2 AttackArea · 2 Cover"));
    assert!(h.has("2 AttackArea · 1 Cover"));
    assert!(h.has("AttackArea + Cover"));
    // The 4-ship shows its low and high elements, the 2-ship one altitude.
    let flights = crate::flights::preview_flights(&h.app.fighter_flight_config());
    let (low, high) = GroupGeneratorApp::flight_element_altitudes(&flights[0].seats);
    let high = high.expect("a complete 4-ship has a high pair");
    assert!(h.has(&format!("{} / {} m", group_digits(low), group_digits(high))));
    assert_eq!(
        GroupGeneratorApp::flight_element_altitudes(&flights[2].seats).1,
        None
    );
    assert!(h.has("Group 3 · 9 aircraft"));

    h.app.linked_groups = 1;
    h.app.custom_path = Some(PathBuf::from("My_Pack.Group"));
    h.settle();
    assert!(
        h.has("Group logic from My_Pack.Group. One group, parked at 40000, 40000."),
        "{:?}",
        h.labels()
    );
    h.app.custom_path = None;

    // No types: an empty state instead of the fallback type, and Generate is off.
    h.app.type_enabled.iter_mut().for_each(|e| *e = false);
    h.settle();
    assert!(h.has("Select at least one aircraft type to preview the pack."));
    assert!(!h.has("GROUP 1 · FLIGHTS"));
    assert!(!h.has("Red 11"));
    assert!(h.find("Generate File").disabled);
}

#[test]
fn fighter_flight_names_stay_unique_up_to_ten_flights() {
    let names: Vec<String> = (0..10)
        .map(|f| crate::aircraft::plane_display_name(f, 0))
        .collect();
    let unique: std::collections::HashSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "{names:?}");
    assert_eq!(names[0], "Red 11");
    assert_eq!(names[6], "Red 71", "the colour list wraps after six");
}

#[test]
fn fighter_role_and_altitude_text() {
    assert_eq!(
        GroupGeneratorApp::flight_role_text(&[(1.0, true)]),
        "AttackArea"
    );
    assert_eq!(
        GroupGeneratorApp::flight_role_text(&[(1.0, true), (1.0, false)]),
        "AttackArea + Cover"
    );
    assert_eq!(
        GroupGeneratorApp::flight_role_text(&[
            (1.0, true),
            (1.0, false),
            (3.0, true),
            (3.0, false)
        ]),
        "2 AttackArea · 2 Cover"
    );
    assert_eq!(
        GroupGeneratorApp::altitude_text(1800.0, Some(3800.0)),
        "1 800 / 3 800 m"
    );
    assert_eq!(GroupGeneratorApp::altitude_text(5200.0, None), "5 200 m");
    assert_eq!(group_digits(227_880.4), "227 880");
    assert_eq!(group_digits(950.0), "950");
    assert_eq!(group_digits(-1234.0), "-1 234");
}

#[test]
fn fighter_nodegate_links_never_end_a_row() {
    for groups in 1..=10 {
        let mut w = 130.0;
        while w < 1400.0 {
            let rows = GroupGeneratorApp::pack_card_rows(groups, w);
            let mut next = 0;
            for (r, row) in rows.iter().enumerate() {
                assert_eq!(row.start, next, "rows are contiguous");
                assert!(!row.is_empty());
                next = row.end;
                // A row is its cards plus the link in front of each card after group 1,
                // so every link sits directly before a card on the same row.
                let n = row.len() as f32;
                let links = if r == 0 { n - 1.0 } else { n };
                let row_w = n * PACK_CARD_W + links * PACK_LINK_W;
                assert!(
                    row_w <= w || row.len() == 1,
                    "{groups} groups at {w} px: row {r} is {row_w} px"
                );
            }
            assert_eq!(next, groups);
            w += 37.0;
        }
    }
    // Ten groups render as cards, each inside the window.
    let mut h = Harness::new("fighter_cards");
    h.tab("Fighter Pack");
    h.app.linked_groups = 10;
    h.settle();
    for g in 1..=10 {
        let card = h.find_prefix(&format!("Group {g} · "));
        assert!(
            card.rect.right() <= SCREEN.x,
            "Group {g} card off screen: {:?}",
            card.rect
        );
    }
}

#[test]
fn fighter_altitude_strip_skips_crowded_labels_but_every_tick_hovers() {
    // Skipping: a label that would touch the last drawn one is left out.
    let shown = strip_labels_shown(&[0.0, 20.0, 45.0, 100.0], &[40.0, 40.0, 40.0, 40.0], 6.0);
    assert_eq!(shown, vec![true, false, false, true]);
    let shown = strip_labels_shown(&[100.0, 0.0], &[40.0, 40.0], 6.0);
    assert_eq!(
        shown,
        vec![true, true],
        "order follows position, not flight number"
    );

    let mut h = Harness::new("fighter_strip");
    h.tab("Fighter Pack");
    h.app.flight_count = 10;
    h.settle();
    let ticks: Vec<Node> = h
        .nodes
        .iter()
        .filter(|n| n.label.starts_with("Altitude tick "))
        .cloned()
        .collect();
    assert_eq!(ticks.len(), 10, "one tick per flight: {:?}", h.labels());
    // Hovering a tick names its flight (whether or not its label was drawn).
    let tick = h.find("Altitude tick Green 11");
    h.move_to(tick.rect.center());
    for _ in 0..40 {
        h.step(Vec::new());
    }
    assert!(
        h.nodes.iter().any(|n| n.label.starts_with("Green 11 · ")),
        "hover names the flight: {:?}",
        h.labels()
    );
}

// ── Exclusive Activation ──────────────────────────────────────────────────

#[test]
fn exclusive_plans_select_remove_undo_and_generate() {
    let mut h = Harness::new("exclusive");
    h.tab("Exclusive Activation");
    dialog::answer(vec![repo(
        "TemplateExamples/Exclusive_Activation_6plan.Group",
    )]);
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

#[test]
fn exclusive_header_tags_kicker_and_idle_problem() {
    let mut h = Harness::new("exclusive_tags");
    h.tab("Exclusive Activation");
    assert!(
        !h.labels().iter().any(|l| l.starts_with("Editing")),
        "no pack loaded yet"
    );
    dialog::answer(vec![repo(
        "TemplateExamples/Exclusive_Activation_6plan.Group",
    )]);
    h.click("Add templates…");
    assert!(
        h.has("Editing Exclusive_Activation_6plan"),
        "{:?}",
        h.labels()
    );
    assert!(h.has("Plan 1"), "kicker in sentence case");
    assert!(!h.has("PLAN 1"));

    // Plan 2 loses its end timer, then its checkzones: tags and the idle status follow.
    h.app.bomber_slots[1].selected_completion = None;
    h.app.status = Status::Idle;
    h.settle();
    assert_eq!(
        plan_tags(&h.app.bomber_slots[1]),
        vec![("⚠ End timer", false)]
    );
    assert!(h.has("Plan 2 has no end timer"), "{:?}", h.labels());
    h.app.bomber_slots[1].selected_triggers.clear();
    h.settle();
    assert_eq!(
        plan_tags(&h.app.bomber_slots[1]),
        vec![("⚠ Checkzone", false), ("⚠ End timer", false)]
    );
    assert!(h.has("Plan 2 has no start checkzone"));
    assert!(h.find("Generate File").disabled);

    // A plan with a wiring warning is not Ready.
    let mut warned = h.app.bomber_slots[0].clone();
    let zone = warned.selected_triggers[0];
    warned
        .info
        .trigger_warnings
        .insert(zone, "zone is not Closer".into());
    assert_eq!(plan_tags(&warned), vec![("⚠ Check", false)]);

    // A long name truncates and leaves room for the tag.
    let long = "Very long plan name ".repeat(12);
    h.app.bomber_slots[0].info.name = long.clone();
    h.settle();
    let card = h.find("Plan card 1");
    let title = h.find(&format!("1 · {long}"));
    assert!(
        title.rect.right() + 30.0 < card.rect.right(),
        "title {:?} leaves no room in {:?}",
        title.rect,
        card.rect
    );
}

#[test]
fn exclusive_editing_clears_on_generate_and_when_emptied() {
    let mut h = Harness::new("exclusive_editing");
    h.tab("Exclusive Activation");
    dialog::answer(vec![repo(
        "TemplateExamples/Exclusive_Activation_6plan.Group",
    )]);
    h.click("Add templates…");
    assert!(h.has("Editing Exclusive_Activation_6plan"));
    let out = h.out("exclusive.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
    assert!(h.app.bomber_loaded_path.is_none());
    assert!(!h.has("Editing Exclusive_Activation_6plan"));

    dialog::answer(vec![repo(
        "TemplateExamples/Exclusive_Activation_6plan.Group",
    )]);
    h.click("Add templates…");
    h.app
        .bomber_slots
        .retain(|s| s.path != repo("TemplateExamples/Exclusive_Activation_6plan.Group"));
    h.settle();
    assert!(
        !h.labels().iter().any(|l| l.starts_with("Editing")),
        "no plans from the pack, no Editing"
    );
}

// ── Airfield ──────────────────────────────────────────────────────────────

#[test]
fn map_imports_landscape_and_harvested_airfields() {
    let mut h = Harness::new("provisions");
    let template = PathBuf::from(&h.app.map_template_dir);
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&template).unwrap();
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        template.join("landscape_Korea_Cities.Group"),
        tiny_named_group("CitiesProvision"),
    )
    .unwrap();
    std::fs::write(
        db.join("catalog.Group"),
        tiny_named_group("NotAnAirfieldCatalog"),
    )
    .unwrap();
    std::fs::write(db.join("Yonpo_502.Group"), tiny_named_group("YonpoField")).unwrap();
    h.app.custom_front_xz = vec![(100_000.0, 0.0), (100_000.0, 499_200.0)];

    h.tab("Map");
    let tab = h.find_prefix("References");
    h.click_at(tab.rect.center());
    assert!(
        h.has("landscape_Korea_Cities"),
        "landscape group is listed: {:?}",
        h.labels()
    );
    assert!(
        !h.has("Yonpo_502"),
        "a harvested airfield is not an automatic map group: {:?}",
        h.labels()
    );
    assert!(
        !h.has("NotAnAirfieldCatalog"),
        "the catalog is not an airfield"
    );
    assert!(h.has("TEMPLATE FOLDER"));
    assert!(
        h.has("1 airfield(s) from the Airfield tab are placed on this base map. 0 sit outside this area and are left out."),
        "{:?}",
        h.labels()
    );

    let out = h.out("provisioned.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate Base Map");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("CitiesProvision"), "{text}");
    assert_eq!(
        text.matches("YonpoField").count(),
        1,
        "the airfield list places Yonpo once"
    );
    assert!(!text.contains("NotAnAirfieldCatalog"), "{text}");
}

#[test]
fn map_export_places_only_airfields_kept_on_the_airfield_tab() {
    let mut h = Harness::new("airfield_export_owner");
    let template = PathBuf::from(&h.app.map_template_dir);
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&template).unwrap();
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        template.join("landscape_Korea_Cities.Group"),
        tiny_named_group("CitiesProvision"),
    )
    .unwrap();
    let suwon = tiny_airfield_at("Suwon", "150000.000", "150000.000", "502");
    std::fs::write(db.join("Suwon_502.Group"), &suwon).unwrap();
    let extra = crate::parser::parse_il2_document(&tiny_airfield_at(
        "ExtraField",
        "151000.000",
        "151000.000",
        "502",
    ))
    .unwrap();
    let copy = crate::parser::parse_il2_document(&suwon).unwrap();
    h.app.map_refs.push(crate::frontlines::MapRefGroup {
        path: PathBuf::from("loaded-suwon.Group"),
        entity: copy,
    });
    h.app.map_refs.push(crate::frontlines::MapRefGroup {
        path: PathBuf::from("loaded-extra.Group"),
        entity: extra,
    });
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];

    h.tab("Map");
    let out = h.out("once.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate Base Map");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("CitiesProvision"), "{text}");
    assert_eq!(text.matches("Name = \"Suwon\";").count(), 2, "{text}");
    assert!(
        !text.contains("ExtraField"),
        "an airfield that is only a map reference is left out"
    );

    h.tab("Airfield");
    let right = |node: &Node| node.rect.left() > SCREEN.x - 290.0;
    let removes: Vec<_> = h.all("Remove").into_iter().filter(&right).collect();
    let suwon_row = h
        .all("Suwon")
        .into_iter()
        .filter(&right)
        .min_by(|a, b| {
            let nearest = |row: &Node| {
                removes
                    .iter()
                    .map(|n| (n.rect.center().y - row.rect.center().y).abs())
                    .fold(f32::MAX, f32::min)
            };
            nearest(a).total_cmp(&nearest(b))
        })
        .expect("Suwon row");
    let remove = removes
        .iter()
        .min_by(|a, b| {
            (a.rect.center().y - suwon_row.rect.center().y)
                .abs()
                .total_cmp(&(b.rect.center().y - suwon_row.rect.center().y).abs())
        })
        .expect("Remove");
    h.click_at(remove.rect.center());
    h.tab("Map");
    let out = h.out("removed.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate Base Map");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("CitiesProvision"), "{text}");
    assert!(
        !text.contains("Suwon"),
        "removing the airfield on the Airfield tab leaves it out of the base map"
    );
    assert!(!text.contains("ExtraField"));
}

fn tiny_airfield_at(name: &str, x: &str, z: &str, country: &str) -> String {
    let mut root = crate::ast::Il2Entity::new("Group");
    root.set_name(name);
    root.index = Some(1);
    root.set_property("Index", "1");
    root.set_property("Desc", "\"\"");
    let mut field = crate::ast::Il2Entity::new("Airfield");
    field.set_name(name);
    field.index = Some(2);
    field.set_property("Index", "2");
    field.set_property("XPos", x);
    field.set_property("YPos", "0.000");
    field.set_property("ZPos", z);
    field.set_property("Country", country);
    root.children.push(field);
    crate::serialize::serialize_group(&root)
}

#[test]
fn airfield_lists_each_harvested_airbase() {
    let mut h = Harness::new("airbases");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        db.join("Suwon_601.Group"),
        tiny_named_group_at("Suwon", "140000.000", "150000.000"),
    )
    .unwrap();
    std::fs::write(
        db.join("Yonpo_502.Group"),
        tiny_named_group_at("Yonpo", "160000.000", "150000.000"),
    )
    .unwrap();
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];
    h.tab("Airfield");
    assert!(h.has("AIRBASES"));
    assert!(h.has("Suwon"), "{:?}", h.labels());
    assert!(h.has("Yonpo"));
    h.click("Add aircraft");
    h.settle();
    assert!(
        h.has("Suwon · 1"),
        "the selected airbase keeps the aircraft"
    );
    h.click("Yonpo");
    assert!(h.has("Yonpo"), "{:?}", h.labels());
    assert!(!h.app.field_spawn_nato, "Yonpo is a DPRK field");
    assert!(
        h.app.field_planes.is_empty(),
        "Yonpo starts with no aircraft"
    );
    let suwon = h
        .app
        .airbases
        .iter()
        .find(|s| s.info.name == "Suwon")
        .expect("Suwon slot");
    assert_eq!(suwon.planes.len(), 1);
    assert!(suwon.nato);
}

#[test]
fn airbases_in_the_map_area_mark_missing_data() {
    let mut h = Harness::new("airbase_area");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(db.join("Suwon_502.Group"), tiny_named_group("Suwon")).unwrap();
    std::fs::write(
        db.join("Kimpo_601.Group"),
        tiny_named_group_at("Kimpo", "142000.000", "150000.000"),
    )
    .unwrap();
    std::fs::write(
        db.join("Yonpo_502.Group"),
        tiny_named_group_at("Yonpo", "200000.000", "20000.000"),
    )
    .unwrap();
    let mut catalog = crate::ast::Il2Entity::new("Group");
    catalog.set_name("AirfieldDB");
    let mut missing = crate::ast::Il2Entity::new("AirfieldRecord");
    missing.set_name("Wonsan");
    missing.set_property("File", "\"Wonsan_502.Group\"");
    missing.set_property("XPos", "155000.000");
    missing.set_property("ZPos", "155000.000");
    missing.set_property("TaxiNodes", "0");
    catalog.children.push(missing);
    std::fs::write(
        db.join("catalog.Group"),
        crate::serialize::serialize_group(&catalog),
    )
    .unwrap();
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];
    h.tab("Airfield");
    assert!(h.has("IN THE AREA"), "{:?}", h.labels());
    assert!(!h.has("OUTSIDE THE AREA"), "{:?}", h.labels());
    let row = |label: &str| {
        let mut hits: Vec<_> = h
            .all(label)
            .into_iter()
            .filter(|node| node.rect.left() > SCREEN.x - 290.0)
            .collect();
        assert!(!hits.is_empty(), "no {label} in the airbases list");
        hits.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
        hits.remove(0)
    };
    let notice_under = |row: &Node| {
        h.all("Data required").iter().any(|node| {
            node.rect.left() > SCREEN.x - 290.0
                && node.rect.top() >= row.rect.bottom() - 2.0
                && node.rect.top() < row.rect.bottom() + 30.0
        })
    };
    let dprk = row("DPRK");
    let nato = row("NATO");
    let suwon = row("Suwon");
    let wonsan = row("Wonsan");
    let kimpo = row("Kimpo");
    assert!(dprk.rect.bottom() <= suwon.rect.top() + 1.0);
    assert!(suwon.rect.bottom() <= wonsan.rect.top() + 1.0);
    assert!(wonsan.rect.bottom() <= nato.rect.top() + 1.0);
    assert!(nato.rect.bottom() <= kimpo.rect.top() + 1.0);
    assert!(notice_under(&suwon), "Suwon has no runway group");
    assert!(notice_under(&wonsan), "Wonsan has no harvested file");
    assert!(notice_under(&kimpo), "Kimpo has no runway group");
    assert!(!h.has("Yonpo"), "Yonpo is outside the map area");
}

#[test]
fn preloaded_airbase_can_be_removed_restored_and_added() {
    let mut h = Harness::new("airbase_remove");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&db).unwrap();
    let suwon_path = db.join("Suwon_502.Group");
    std::fs::write(
        &suwon_path,
        tiny_named_group_at("Suwon", "150000.000", "150000.000"),
    )
    .unwrap();
    std::fs::write(
        db.join("Yonpo_601.Group"),
        tiny_named_group_at("Yonpo", "142000.000", "150000.000"),
    )
    .unwrap();
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];
    h.tab("Airfield");
    assert!(h.has("Restore preloaded"));
    assert!(h.has("Add airbase…"));
    let right = |node: &Node| node.rect.left() > SCREEN.x - 290.0;
    let removes: Vec<_> = h.all("Remove").into_iter().filter(right).collect();
    let suwon = h
        .all("Suwon")
        .into_iter()
        .filter(right)
        .min_by(|a, b| {
            let da = removes
                .iter()
                .map(|n| (n.rect.center().y - a.rect.center().y).abs())
                .fold(f32::MAX, f32::min);
            let db = removes
                .iter()
                .map(|n| (n.rect.center().y - b.rect.center().y).abs())
                .fold(f32::MAX, f32::min);
            da.total_cmp(&db)
        })
        .expect("Suwon row");
    let remove = removes
        .iter()
        .min_by(|a, b| {
            (a.rect.center().y - suwon.rect.center().y)
                .abs()
                .total_cmp(&(b.rect.center().y - suwon.rect.center().y).abs())
        })
        .cloned()
        .expect("Remove beside Suwon");
    assert!(
        (remove.rect.center().y - suwon.rect.center().y).abs() < 20.0,
        "remove {:?} suwon {:?}",
        remove.rect,
        suwon.rect
    );
    h.click_at(remove.rect.center());
    assert!(!h.has("Suwon"), "{:?}", h.labels());
    assert!(h.has("Yonpo"));
    assert!(
        suwon_path.is_file(),
        "remove leaves the harvest file on disk"
    );
    h.click("Restore preloaded");
    assert!(h.has("Suwon"), "{:?}", h.labels());
    let extra = std::env::temp_dir().join(format!("il2_add_airbase_{}.Group", std::process::id()));
    std::fs::write(
        &extra,
        tiny_named_group_at("Kimpo", "151000.000", "151000.000"),
    )
    .unwrap();
    dialog::answer(vec![extra.clone()]);
    h.click("Add airbase…");
    assert!(h.has("Kimpo"), "{:?}", h.labels());
    let _ = std::fs::remove_file(&extra);
}

#[test]
fn double_loaded_airbase_offers_a_removal() {
    let mut h = Harness::new("airbase_dup");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&db).unwrap();
    let write_field = |country: i32| {
        let mut root = crate::ast::Il2Entity::new("Group");
        root.set_name("K-23_Pyongyang_AF");
        root.index = Some(1);
        root.set_property("Index", "1");
        root.set_property("Desc", "\"\"");
        let mut field = crate::ast::Il2Entity::new("Airfield");
        field.set_name("K-23_Pyongyang_AF");
        field.index = Some(2);
        field.set_property("Index", "2");
        field.set_property("LinkTrId", "3");
        field.set_property("XPos", "150000.000");
        field.set_property("YPos", "0.000");
        field.set_property("ZPos", "150000.000");
        field.set_property("Country", country.to_string());
        root.children.push(field);
        let path = db.join(format!("K-23_Pyongyang_AF_{country}.Group"));
        std::fs::write(&path, crate::serialize::serialize_group(&root)).unwrap();
        path
    };
    let prc = write_field(503);
    let usa = write_field(601);
    let mut catalog = crate::ast::Il2Entity::new("Group");
    catalog.set_name("AirfieldDB");
    for (file, country) in [
        ("K-23_Pyongyang_AF_503.Group", 503),
        ("K-23_Pyongyang_AF_601.Group", 601),
    ] {
        let mut record = crate::ast::Il2Entity::new("AirfieldRecord");
        record.set_name("K-23_Pyongyang_AF");
        record.set_property("File", format!("\"{file}\""));
        record.set_property("Country", country.to_string());
        record.set_property("XPos", "150000.000");
        record.set_property("ZPos", "150000.000");
        record.set_property("TaxiNodes", "0");
        catalog.children.push(record);
    }
    std::fs::write(
        db.join("catalog.Group"),
        crate::serialize::serialize_group(&catalog),
    )
    .unwrap();
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];
    h.tab("Airfield");
    assert!(h.has("Airbase loaded twice"), "{:?}", h.labels());
    assert!(h.has("Remove PRC · K-23_Pyongyang_AF"), "{:?}", h.labels());
    assert!(h.has("Remove USA · K-23_Pyongyang_AF"), "{:?}", h.labels());
    h.click("Remove USA · K-23_Pyongyang_AF");
    assert!(!h.has("Airbase loaded twice"), "{:?}", h.labels());
    assert_eq!(h.app.airbases.len(), 1);
    assert_eq!(h.app.airbases[0].path, prc);
    assert!(
        usa.is_file(),
        "removing a copy leaves the harvest file on disk"
    );
}

#[test]
fn aircraft_copy_between_airfields() {
    let mut h = Harness::new("aircraft_copy");
    let db = PathBuf::from(&h.app.harvest_db_dir);
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        db.join("Suwon_502.Group"),
        tiny_named_group_at("Suwon", "150000.000", "150000.000"),
    )
    .unwrap();
    std::fs::write(
        db.join("Yonpo_601.Group"),
        tiny_named_group_at("Yonpo", "142000.000", "150000.000"),
    )
    .unwrap();
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.app.custom_front_xz = vec![(148_000.0, 0.0), (148_000.0, 499_200.0)];
    h.tab("Airfield");

    let right = |node: &Node| node.rect.left() > SCREEN.x - 290.0;
    let click_base = |h: &mut Harness, name: &str| {
        let removes: Vec<_> = h.all("Remove").into_iter().filter(&right).collect();
        let row = h
            .all(name)
            .into_iter()
            .filter(&right)
            .min_by(|a, b| {
                let da = removes
                    .iter()
                    .map(|n| (n.rect.center().y - a.rect.center().y).abs())
                    .fold(f32::MAX, f32::min);
                let db = removes
                    .iter()
                    .map(|n| (n.rect.center().y - b.rect.center().y).abs())
                    .fold(f32::MAX, f32::min);
                da.total_cmp(&db)
            })
            .unwrap_or_else(|| panic!("{name} row"));
        h.click_at(row.rect.center());
    };

    click_base(&mut h, "Suwon");
    assert!(h.find("Paste").disabled, "paste waits for a copy");
    let mut mustang = crate::airstart::default_plane("f51d");
    mustang.altitude_m = 0;
    let mut mig = crate::airstart::default_plane("mig15bis");
    mig.altitude_m = 0;
    h.app.field_planes = vec![mustang, mig];
    h.settle();
    assert_eq!(
        h.app
            .airbases
            .iter()
            .find(|s| s.info.name == "Suwon")
            .unwrap()
            .planes
            .len(),
        2
    );

    h.click("Copy all");
    assert_eq!(h.app.plane_clip.len(), 2);
    assert_eq!(h.app.plane_clip[0].type_id, "f51d");
    assert_eq!(h.app.plane_clip[1].type_id, "mig15bis");

    click_base(&mut h, "Yonpo");
    assert!(h.app.field_planes.is_empty());
    h.click("Paste");
    assert_eq!(h.app.field_planes.len(), 2);
    assert_eq!(h.app.field_planes[0].type_id, "f51d");
    assert_eq!(h.app.field_planes[1].type_id, "mig15bis");
    h.settle();
    let suwon = h
        .app
        .airbases
        .iter()
        .find(|s| s.info.name == "Suwon")
        .unwrap();
    assert_eq!(
        suwon.planes.len(),
        2,
        "the source airfield keeps its aircraft"
    );
    assert_eq!(
        h.app
            .airbases
            .iter()
            .find(|s| s.info.name == "Yonpo")
            .unwrap()
            .planes
            .len(),
        2
    );

    h.click("Add air start");
    h.click("Paste");
    assert_eq!(h.app.air_starts[0].planes.len(), 3);
    assert_eq!(h.app.air_starts[0].planes[1].type_id, "f51d");
    assert_eq!(h.app.air_starts[0].planes[2].type_id, "mig15bis");

    click_base(&mut h, "Suwon · 2");
    h.click("F-51D Mustang · Normal · Engine running · unlimited");
    h.click("Copy aircraft");
    assert_eq!(h.app.plane_clip.len(), 1);
    assert_eq!(h.app.plane_clip[0].type_id, "f51d");
    click_base(&mut h, "Yonpo · 2");
    h.click("Paste");
    assert_eq!(h.app.field_planes.len(), 3);
    assert_eq!(h.app.field_planes[2].type_id, "f51d");
    h.settle();
    assert_eq!(
        h.app
            .airbases
            .iter()
            .find(|s| s.info.name == "Suwon")
            .unwrap()
            .planes
            .len(),
        2
    );
    assert_eq!(
        h.app
            .airbases
            .iter()
            .find(|s| s.info.name == "Yonpo")
            .unwrap()
            .planes
            .len(),
        3
    );
}

fn tiny_named_group(name: &str) -> String {
    tiny_named_group_at(name, "150000.000", "150000.000")
}

fn tiny_named_group_at(name: &str, x: &str, z: &str) -> String {
    let mut root = crate::ast::Il2Entity::new("Group");
    root.set_name(name);
    root.index = Some(1);
    root.set_property("Index", "1");
    root.set_property("Desc", "\"\"");
    let mut block = crate::ast::Il2Entity::new("Block");
    block.set_name("Hall");
    block.index = Some(2);
    block.set_property("Index", "2");
    block.set_property("XPos", x);
    block.set_property("YPos", "0.000");
    block.set_property("ZPos", z);
    root.children.push(block);
    crate::serialize::serialize_group(&root)
}

#[test]
fn airfield_load_side_and_generate() {
    let mut h = Harness::new("airfield");
    h.tab("Airfield");
    assert!(
        h.find("Generate File").disabled,
        "Generate waits for a file"
    );
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
    std::fs::copy(
        repo("TemplateExamples/K13 AFB_mp.Group"),
        missions.join("_gen.mission"),
    )
    .unwrap();
    for _ in 0..400 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step(Vec::new());
        if h.app.harvest_log.len() > logged {
            break;
        }
    }
    assert!(
        h.app.harvest_log.len() > logged,
        "watcher harvested the new file: {}",
        h.status()
    );
    h.click("Watch for new airfields");
    assert!(h.app.harvest_watcher.is_none());
}

#[test]
fn airfield_panels_order_and_wording() {
    let mut h = Harness::new("airfield_panels");
    h.tab("Airfield");
    let get = h.find("GET THE FILE").rect.top();
    let side = h.find("FRIENDLY PLANE COALITION").rect.top();
    let harvest = h.find("HARVEST AUTOMATICALLY").rect.top();
    assert!(
        get < side && side < harvest,
        "Get the file, coalition, then the harvester"
    );
    assert!(
        h.has("Load a Freeflight airfield from _gen.mission, then generate the cleaned group.")
    );

    dialog::answer(vec![repo("TemplateExamples/Airfield_Mess.Group")]);
    h.click("Load airfield…");
    let info = h.app.airfield_info.clone().expect("airfield loaded");
    assert!(h.has("Strips the player and SP logic, then retargets the checkzones that were linked to the player."));
    // Removed: country name only.
    for p in &info.player_planes {
        assert!(h.has(&country_name(p.country)), "{:?}", h.labels());
    }
    assert!(
        !h.labels()
            .iter()
            .any(|l| COUNTRIES.iter().any(|(_, c)| l.contains(c))),
        "no raw country labels"
    );
    // Relinked: a short count beside the side.
    let n = info.unlink_zones.len();
    assert!(
        h.has(&format!("{n} checkzone{}", if n == 1 { "" } else { "s" })),
        "{:?}",
        h.labels()
    );
    // Right panel: sentence case layout, grouped origin.
    assert!(h.has(if info.in_group {
        "Inside a Group"
    } else {
        "Blocks at the root"
    }));
    if let Some((x, z)) = info.origin_xz {
        assert!(h.has(&format!("{}, {}", group_digits(x), group_digits(z))));
    }
    assert_eq!(country_name(601), "USA");
    assert_eq!(country_name(999), "country 999");
}

#[test]
fn air_start_bank_reaches_the_map() {
    let mut h = Harness::new("air_starts");
    h.tab("Airfield");
    assert!(h.has("AIR STARTS"));
    assert!(h.has("FIELD SPAWN"));
    assert_eq!(h.app.field_planes[0].type_id, "f51d");
    assert!(!h.app.field_planes[0].parking_snap);
    h.click("Add air start");
    assert!(h.has("Air start 1"));
    assert!(h.has("Add aircraft"));
    assert_eq!(h.app.air_starts[0].planes[0].type_id, "f51d");
    let name = h.find("Air start 1");
    let dprk = h
        .all("DPRK")
        .into_iter()
        .find(|n| n.rect.top() >= name.rect.top() - 4.0)
        .expect("air start coalition");
    h.click_at(dprk.rect.center());
    assert!(!h.app.air_starts[0].nato);
    assert!(
        h.app.field_spawn_nato,
        "the field spawn coalition stays NATO"
    );
    assert_eq!(h.app.air_starts[0].country(), 501);
    h.tab("Map");
    h.click("Forces");
    assert!(h.has("Air start 1 · DPRK"));
    assert!(h.has("Place"));
}

#[test]
fn every_placed_air_start_is_listed_on_the_right() {
    let mut h = Harness::new("air_start_rows");
    let north = crate::airstart::default_field("CAP North", false);
    let mut turned = north.clone();
    turned.heading_deg = 80.0;
    let south = crate::airstart::default_field("CAP South", true);
    h.app.air_starts = vec![
        north.clone(),
        south.clone(),
        crate::airstart::default_field("Reserve", true),
    ];
    h.app.placed_air_starts = vec![
        crate::airstart::PlacedAirStart {
            field: north,
            x: 150_000.0,
            z: 150_000.0,
        },
        crate::airstart::PlacedAirStart {
            field: turned,
            x: 151_000.0,
            z: 150_000.0,
        },
        crate::airstart::PlacedAirStart {
            field: south,
            x: 10_000.0,
            z: 10_000.0,
        },
    ];
    h.app.front_aabb =
        crate::mapclip::WorldAabb::from_corners(140_000.0, 140_000.0, 160_000.0, 160_000.0);
    h.tab("Airfield");
    assert!(h.has("CAP North · 0°"), "{:?}", h.labels());
    assert!(h.has("CAP North · 80°"), "{:?}", h.labels());
    assert!(h.has("CAP South · 0°"), "{:?}", h.labels());
    assert!(h.has("Reserve · NATO"), "{:?}", h.labels());
    let hint = h.find("DPRK is north of the front and NATO is south. Data required means that field has no runway group. Air starts are listed on the same side. Korea fields, under the list, are the sites in this area that still need a harvest.");
    let dprk = h
        .all("DPRK")
        .into_iter()
        .filter(|node| node.rect.left() > SCREEN.x - 290.0)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("DPRK heading");
    assert!(
        hint.rect.bottom() <= dprk.rect.top() + 1.0,
        "instructions stay above the list"
    );
    assert!(!h.has("Listed with the airbases. Select one to edit its aircraft."));
    assert!(!h.has("None on this side of the front."));
    assert!(!h.has("OUTSIDE THE AREA"));

    let north_row = h
        .all("CAP North · 0°")
        .into_iter()
        .find(|node| node.rect.left() > SCREEN.x - 290.0)
        .expect("north air start");
    h.click_at(north_row.rect.center());
    assert_eq!(h.app.air_start_placed, Some(0));
    assert_eq!(
        h.app.placed_air_starts[0].field.planes[0].type_id,
        "mig15bis"
    );
    h.app.placed_air_starts[0].field.planes[0].type_id = "la11".into();
    h.app.placed_air_starts[1].field.planes[0].type_id = "i16".into();
    h.settle();
    let turned = h
        .all("CAP North · 80°")
        .into_iter()
        .find(|node| node.rect.left() > SCREEN.x - 290.0)
        .expect("turned air start");
    h.click_at(turned.rect.center());
    assert_eq!(h.app.air_start_placed, Some(1));
    assert_eq!(h.app.placed_air_starts[1].field.planes[0].type_id, "i16");
    assert_eq!(
        h.app.placed_air_starts[0].field.planes[0].type_id, "la11",
        "the other air start keeps its own aircraft"
    );
}

#[test]
fn field_spawn_planes_are_not_the_air_start_list() {
    let mut h = Harness::new("field_spawn_lists");
    h.tab("Airfield");
    assert!(h.has("FIELD SPAWN"));
    assert!(h.has("AIR STARTS"));
    assert!(h.has("Export field spawn…"));
    h.app.field_planes[0].type_id = "mig15bis".into();
    h.app.field_planes[0].parking_snap = true;
    h.app.field_planes[0].fuel = 0.4;
    h.click("Add air start");
    assert_eq!(h.app.air_starts[0].planes[0].type_id, "f51d");
    assert!(!h.app.air_starts[0].planes[0].parking_snap);
    h.app.air_starts[0].planes[0].type_id = "la11".into();
    h.app.air_starts[0].planes[0].fuel = 0.2;
    assert_eq!(h.app.field_planes[0].type_id, "mig15bis");
    assert!(h.app.field_planes[0].parking_snap);
    assert!((h.app.field_planes[0].fuel - 0.4).abs() < 1e-9);
}

// ── Per-tab status and saved state ────────────────────────────────────────

#[test]
fn each_tab_keeps_its_own_status() {
    let mut h = Harness::new("tab_status");
    h.tab("Fighter Pack");
    h.app.status = Status::Info("Fighter message".into());
    h.tab("Airfield");
    assert_ne!(h.status(), "Fighter message", "a message stays on its tab");
    h.app.status = Status::Error("Airfield message".into());
    h.key_with(Key::Num3, CTRL);
    assert_eq!(h.status(), "Fighter message");
    h.tab("Airfield");
    assert_eq!(h.status(), "Airfield message");
}

#[test]
fn a_second_identical_generate_marks_the_tab_saved() {
    let mut h = Harness::new("saved");
    // The empty test store would warn about unmeasured ground, and a warning
    // stays on the tab. This check is about the plain "Wrote …" line.
    h.app.terrain_apply = false;
    h.tab("Template");
    add_model(&mut h, "F-51D");
    let out = h.out("tpl.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_group_file(&out);
    assert!(!h.app.is_dirty(AppMode::Template));
    let first = h.status();

    // An edit that does not change the Generate message.
    h.app.tpl_zone_in += 500.0;
    assert!(h.app.is_dirty(AppMode::Template));
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_eq!(h.status(), first, "same message as the first Generate");
    assert!(
        !h.app.is_dirty(AppMode::Template),
        "the second Generate counts as saved"
    );

    // An edit retires the "Wrote …" message (age_status); a cancelled dialog
    // then reports nothing: the status stays and the edits stay unsaved.
    h.app.tpl_zone_in += 500.0;
    h.settle();
    assert_eq!(h.status(), "", "the Generate message is gone after an edit");
    dialog::answer(Vec::new());
    h.click("Generate File");
    assert_eq!(h.status(), "");
    assert!(h.app.is_dirty(AppMode::Template));
}

/// Info messages give way to the idle hint once the tab is edited; warnings stay.
#[test]
fn status_messages_retire_after_the_next_edit() {
    let mut h = Harness::new("agestatus");
    h.tab("Fighter Pack");
    h.app.status = Status::Info("Fighter Pack reset to the default settings.".into());
    h.settle();
    assert!(
        !h.status().is_empty(),
        "a fresh message stays while nothing changes"
    );
    h.app.flight_count += 1;
    h.settle();
    assert_eq!(h.status(), "", "an edit retires it");
    h.app.status = Status::Warn {
        lead: "Placed with notes: ".into(),
        items: vec!["one".into()],
    };
    h.settle();
    h.app.flight_count -= 1;
    h.settle();
    assert!(
        h.status().starts_with("Placed with notes"),
        "warnings stay until replaced"
    );
}

/// Undoing a Remove on Exclusive Activation selects the plan that came back.
#[test]
fn exclusive_undo_reselects_the_removed_plan() {
    let mut h = Harness::new("exundo");
    h.tab("Exclusive Activation");
    dialog::answer(vec![repo(
        "TemplateExamples/Exclusive_Activation_6plan.Group",
    )]);
    h.key_with(Key::O, CTRL);
    h.settle();
    assert_eq!(h.app.bomber_slots.len(), 6);
    h.app.bomber_selected = Some(5);
    h.settle();
    h.click("Remove");
    assert_eq!(h.app.bomber_slots.len(), 5);
    assert_eq!(
        h.app.bomber_selected,
        Some(4),
        "the neighbour is selected after Remove"
    );
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.bomber_slots.len(), 6);
    assert_eq!(
        h.app.bomber_selected,
        Some(5),
        "Undo selects the restored plan"
    );
}

// ── Map ───────────────────────────────────────────────────────────────────

#[test]
fn map_tools_keys_escape_one_shot_objectives_and_tab_reset() {
    let mut h = Harness::new("maptools");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    for (i, key) in [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
    ]
    .into_iter()
    .enumerate()
    {
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
    let clear = h
        .all("Clear DPRK")
        .into_iter()
        .find(|n| !n.disabled && n.rect.top() > h.find("OBJECTIVES").rect.top())
        .expect("objectives Clear DPRK");
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
fn map_load_imports_air_starts_and_field_aircraft() {
    let mut north = crate::airstart::default_field("CAP North", false);
    north.heading_deg = 45.0;
    north.planes[0].number = 4;
    north.planes[0].altitude_m = 2200;
    let starts = crate::airstart::combined_group(
        &[crate::airstart::PlacedAirStart {
            field: north,
            x: 120_000.0,
            z: 240_000.0,
        }],
        &[15.0],
    );

    let mut spawn = crate::airstart::default_field("ImportField", true);
    spawn.planes[0] = crate::airstart::default_plane("f86a5");
    let mut field = crate::airstart::field_group(&spawn, 130_000.0, 250_000.0, 10.0);
    let air = field
        .children
        .iter_mut()
        .find(|child| child.block_type == "Airfield")
        .unwrap();
    air.set_property("Model", r#""graphics\airfields\fakefield_rnwspawn.mgm""#);
    air.set_property(
        "Script",
        r#""LuaScripts\WorldObjects\Airfields\fakefield_rnwspawn.txt""#,
    );
    let plane = air
        .children
        .iter_mut()
        .find(|child| child.block_type == "Planes")
        .unwrap()
        .children
        .get_mut(0)
        .unwrap();
    plane.set_property("StartType", "1");
    plane.set_property("SnapTo", "0");
    plane.set_property("Altitude", "0");
    field
        .children
        .push(crate::ast::Il2Entity::new("MCU_TR_TaxiGraph"));

    let mut refs = crate::ast::Il2Entity::new("Group");
    refs.set_name("Reference groups");
    refs.children.push(field);
    let mut root = crate::ast::Il2Entity::new("Group");
    root.set_name("Korea Base Map Custom");
    root.children.push(starts);
    root.children.push(refs);

    let mut h = Harness::new("import_air");
    let path = h.out("base.Group");
    std::fs::write(&path, crate::serialize::serialize_group(&root)).unwrap();
    h.tab("Map");
    dialog::answer(vec![path]);
    h.click("Load base map…");
    assert!(h.app.confirm.is_none(), "{}", h.status());
    assert!(h.status().contains("1 air start"), "{}", h.status());
    assert!(
        h.status().contains("1 airfield aircraft list"),
        "{}",
        h.status()
    );
    assert!(h.app.map_armies.is_empty(), "{}", h.status());
    assert_eq!(h.app.placed_air_starts.len(), 1);
    let placed = &h.app.placed_air_starts[0];
    assert_eq!(placed.field.name, "CAP North");
    assert!(!placed.field.nato);
    assert_eq!(placed.field.planes[0].type_id, "mig15bis");
    assert_eq!(placed.field.planes[0].number, 4);
    assert_eq!(placed.field.planes[0].altitude_m, 2200);
    assert!((placed.field.heading_deg - 45.0).abs() < 0.01);
    assert!((placed.x - 120_000.0).abs() < 0.01);
    let bank = h
        .app
        .air_starts
        .iter()
        .find(|field| field.name == "CAP North")
        .expect("air start bank");
    assert_eq!(bank.planes[0].type_id, "mig15bis");
    assert!(!bank.nato);
    let slot = h
        .app
        .airbases
        .iter()
        .find(|slot| slot.planes.iter().any(|plane| plane.type_id == "f86a5"))
        .expect("defined airfield");
    assert_eq!(slot.planes[0].type_id, "f86a5");
    assert!(!slot.planes[0].parking_snap);

    h.click("Forces");
    assert!(h.has("CAP North · DPRK"), "{:?}", h.labels());
    assert!(h.has("CAP North · 45°"), "{:?}", h.labels());
    h.tab("Airfield");
    assert!(h.has("CAP North · 45°"), "{:?}", h.labels());
}

#[test]
fn map_terrain_import_layers_and_export() {
    let mut h = Harness::new("terrain");
    h.tab("Map");
    h.wait_for_map();
    h.click("Terrain");
    assert!(h.has_prefix("0 of "), "empty store: {:?}", h.labels());

    // A snapped probe file: nine land nodes of one tile at 100–108 m.
    let nodes: Vec<(usize, usize)> = heightprobe::tile_probe_nodes(10, 10)
        .into_iter()
        .take(9)
        .collect();
    let mut group = heightprobe::probe_group("snapped", &nodes);
    for (k, child) in group.children.iter_mut().enumerate() {
        child.set_property("YPos", format!("{:.3}", 100.0 + k as f64));
    }
    let snapped = h.out("snapped.Group");
    std::fs::write(&snapped, serialize_group(&group)).unwrap();
    dialog::answer(vec![snapped]);
    h.click("Import snapped…");
    assert_eq!(
        h.app.terrain_store.as_ref().unwrap().measured_nodes(),
        9,
        "{}",
        h.status()
    );
    assert!(h.app.terrain_store_path.is_file(), "the store is saved");
    let (i, j) = nodes[4];
    let y = h.app.terrain_store.as_ref().unwrap().node(i, j).unwrap();
    assert!((y - 104.0).abs() < 0.05);

    // An unsnapped file (all probes at 0 m) is refused.
    let raw = h.out("unsnapped.Group");
    std::fs::write(
        &raw,
        serialize_group(&heightprobe::tile_probe_group(11, 4).expect("tile 11,4 has probes")),
    )
    .unwrap();
    dialog::answer(vec![raw]);
    h.click("Import snapped…");
    assert!(h.status().contains("not merged"), "{}", h.status());

    h.click("Relief");
    assert!(h.app.terrain_relief.is_some(), "relief texture built");
    h.click("Coverage");
    assert!(h.app.terrain_show_coverage);
    assert_eq!(
        h.find("Coverage").toggled,
        Some(true),
        "the checkbox reports checked"
    );

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
    assert!(
        tiles
            .join(format!("{}.Group", heightprobe::tile_name(10, 10)))
            .is_file()
    );
}

#[test]
fn map_dock_tabs_all_render() {
    let mut h = Harness::new("dock");
    h.tab("Map");
    for (tab, marker) in [
        ("Forces", "FIGHTERS"),
        ("References 0", "Add reference groups…"),
        ("Terrain", "HEIGHT STORE"),
        ("Period", "DRAWN MARKS"),
        ("Air", "SORTIES"),
    ] {
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
            let interactive = matches!(
                n.role,
                Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton
            );
            checked += usize::from(interactive);
            if interactive && !n.disabled && n.rect.height() < 27.5 && n.rect.width() > 0.0 {
                small.push(format!(
                    "{label}: {:?} {:?} {}px",
                    n.label,
                    n.role,
                    n.rect.height()
                ));
            }
        }
    }
    assert!(
        checked > 150,
        "only {checked} interactive widgets seen: the role filter is wrong"
    );
    small.sort();
    small.dedup();
    assert!(
        small.is_empty(),
        "targets under 28 px:\n{}",
        small.join("\n")
    );
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
    let kinds = |h: &Harness| {
        h.app.tpl_seats[0]
            .orders
            .iter()
            .map(|o| o.kind)
            .collect::<Vec<_>>()
    };
    let before = kinds(&h);
    // Select the Goto WP chip ("<n> Goto WP N"), then move it right with its arrow.
    let gi = before
        .iter()
        .position(|k| *k == OrderKind::GotoWaypoint)
        .expect("a Goto WP order");
    assert!(gi + 1 < before.len(), "Goto WP has an order after it");
    let wp = h.app.tpl_seats[0].orders[gi].waypoint.max(1);
    let chip = h.find(&format!("{} Goto WP {wp}", gi + 1));
    h.click_at(chip.rect.center());
    assert!(
        matches!(h.app.tpl_select, Some(TplSelect::Order { seat: 0, order }) if order == gi),
        "{:?}",
        h.app.tpl_select.is_some()
    );
    assert!(h.has(&format!("SELECTED · UNIT 1 · ORDER {}", gi + 1)));
    h.click("Move right");
    let after = kinds(&h);
    assert_eq!(after[gi], before[gi + 1]);
    assert_eq!(after[gi + 1], before[gi]);
}

#[test]
fn template_report_arrow_leaves_its_command() {
    let mut h = Harness::new("reportmove");
    h.tab("Template");
    add_model(&mut h, "F-51D");
    h.click("+ Order ▾");
    h.click("Take Off");
    h.click("+ Order ▾");
    h.click("Formation");
    h.click("+ Order ▾");
    h.click("OnTookOff");
    let kinds = |h: &Harness| {
        h.app.tpl_seats[0]
            .orders
            .iter()
            .map(|o| o.kind)
            .collect::<Vec<_>>()
    };
    let before = kinds(&h);
    let ri = before
        .iter()
        .position(|k| *k == OrderKind::OnTookOff)
        .expect("OnTookOff");
    let chip = h.find(&format!("{} OnTookOff", ri + 1));
    h.click_at(chip.rect.center());
    h.click("Move right");
    let after = kinds(&h);
    assert_ne!(after, before, "the report should leave the Take Off column");
    assert_eq!(after.last().copied(), Some(OrderKind::OnTookOff));
    let ri = after
        .iter()
        .position(|k| *k == OrderKind::OnTookOff)
        .unwrap();
    let chip = h.find(&format!("{} OnTookOff", ri + 1));
    let takeoff = after.iter().position(|k| *k == OrderKind::TakeOff).unwrap();
    let takeoff_chip = h.find(&format!("{} Take Off", takeoff + 1));
    h.press_and_move(chip.rect.center(), takeoff_chip.rect.center());
    h.release_at(takeoff_chip.rect.center());
    let dragged = kinds(&h);
    assert_eq!(dragged[0], OrderKind::OnTookOff);
    assert_eq!(dragged[1], OrderKind::TakeOff);
}

#[test]
fn map_places_fighters_from_the_forces_dock() {
    let mut h = Harness::new("placefighters");
    h.tab("Map");
    h.wait_for_map();
    h.click("Forces");
    // The Fighters section's Place DPRK (Units has one too, further down).
    let place = h
        .all("Place DPRK")
        .into_iter()
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("Place DPRK");
    h.click_at(place.rect.center());
    let n = h.app.map_fighters.as_ref().map_or(0, |l| l.spots.len());
    assert!(n > 0, "no fighters placed: {}", h.status());
    assert!(h.has_prefix("DPRK: "), "count line shows: {:?}", h.labels());
    // Clear DPRK (the one in the Fighters section), then undo.
    let fighters_top = h.find("FIGHTERS").rect.top();
    let clear = h
        .all("Clear DPRK")
        .into_iter()
        .filter(|n| !n.disabled && n.rect.top() > fighters_top)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("fighters Clear DPRK");
    h.click_at(clear.rect.center());
    assert!(h.app.map_fighters.is_none());
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.map_fighters.as_ref().map_or(0, |l| l.spots.len()), n);
}

#[test]
fn map_load_fighter_pack_uses_fighter_icons() {
    let mut h = Harness::new("loadfighters");
    h.tab("Map");
    h.wait_for_map();
    h.click("Forces");
    h.app.fighter_waves = 2;
    h.app.fighter_fill = false;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("TemplateExamples/Eastern_Fighters_Random_5pack_V6.Group");
    dialog::answer(vec![path]);
    let fighters_top = h.find("FIGHTERS").rect.top();
    let units = h.find("UNITS").rect.top();
    let load = h
        .all("Load DPRK…")
        .into_iter()
        .filter(|n| n.rect.top() > fighters_top && n.rect.top() < units)
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("fighters Load DPRK");
    h.click_at(load.rect.center());
    let layout = h.app.map_fighters.as_ref().expect(&h.status());
    assert!(layout.eastern);
    let w1 = layout.spots.iter().filter(|s| s.wave == 1).count();
    let w2 = layout.spots.iter().filter(|s| s.wave == 2).count();
    assert_eq!((w1, w2), (5, 5), "{}", h.status());
    assert_eq!(h.app.map_imported_fighters.len(), 2);
    assert!(h.has_prefix("DPRK: "), "count line shows: {:?}", h.labels());
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
    assert_eq!(
        h.app.terrain_readout().as_deref(),
        Some("Ground not measured")
    );
    // Measure the ground under the pointer, then the readout shows it.
    let (x, z) = h.app.map_hover_xz.expect("pointer over the map");
    let store = h.app.terrain_store.as_mut().expect("store loaded");
    let (i, j) = ((x / 100.0).floor() as usize, (z / 100.0).floor() as usize);
    for (a, b) in [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)] {
        store.set_node(a, b, 321.0);
    }
    h.settle();
    assert_eq!(
        h.app.terrain_readout().as_deref(),
        Some("Ground 321 m · 100 m grid")
    );
}

impl Harness {
    /// Press the primary button at `from`, move to `to` in steps, but keep it held.
    fn press_and_move(&mut self, from: Pos2, to: Pos2) {
        self.move_to(from);
        let mods = self.mods;
        self.step(vec![Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: mods,
        }]);
        for k in 1..=6 {
            self.move_to(from + (to - from) * (k as f32 / 6.0));
        }
    }

    fn release_at(&mut self, pos: Pos2) {
        let mods = self.mods;
        self.step(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: mods,
        }]);
        self.settle();
    }
}

// ── Side markers (README §8) ──────────────────────────────────────────────

/// Opaque pixels in row `y`.
fn row_fill(img: &egui::ColorImage, y: usize) -> usize {
    let w = img.size[0];
    (0..w).filter(|&x| img.pixels[y * w + x].a() > 128).count()
}

/// The row holding the most of the icon; for a level fighter that is the wing.
fn widest_row(img: &egui::ColorImage) -> usize {
    (0..img.size[1])
        .max_by_key(|&y| row_fill(img, y))
        .unwrap_or(0)
}

/// Share of the icon's opaque pixels in the left half.
fn left_share(img: &egui::ColorImage) -> f32 {
    let [w, h] = img.size;
    let (mut left, mut all) = (0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if img.pixels[y * w + x].a() > 128 {
                all += 1;
                left += usize::from(x < w / 2);
            }
        }
    }
    left as f32 / all.max(1) as f32
}

#[test]
fn side_marker_fighters_point_north_before_facing() {
    // `Side::facing_rad` turns DPRK by 180° and leaves NATO as drawn, which
    // is only right if both textures point north. The SVGs are drawn
    // diagonally (DPRK north-west, NATO north-east), so the rasterizer turns
    // them: afterwards the wing is forward of the middle and the plane is
    // left-right symmetric.
    for (eastern, name, bytes) in [
        (
            true,
            "EasternFighter",
            &include_bytes!("../assets/EasternFighter.svg")[..],
        ),
        (
            false,
            "NatoFighter",
            &include_bytes!("../assets/NatoFighter.svg")[..],
        ),
    ] {
        // Turned level, the wing spans a row far wider than anything in the
        // diagonal original.
        let raw = rasterize_svg(bytes, 64);
        let img = fighter_svg_north(eastern, 64);
        let (raw_w, img_w) = (
            row_fill(&raw, widest_row(&raw)),
            row_fill(&img, widest_row(&img)),
        );
        eprintln!("{name}: widest row raw {raw_w}, turned {img_w}");
        assert!(
            img_w as f32 >= raw_w as f32 * 1.15,
            "{name}: turning did not level the wing ({raw_w} → {img_w} px)"
        );
        let row = widest_row(&img);
        assert!(
            row < img.size[1] / 2,
            "{name}: widest row {row} of {} is not in the top half",
            img.size[1]
        );
        let share = left_share(&img);
        assert!(
            (share - 0.5).abs() < 0.03,
            "{name}: {share} of the icon is left of centre"
        );
    }
    assert_eq!(shell::Side::Nato.facing_rad(), 0.0);
    assert!((shell::Side::Dprk.facing_rad() - std::f32::consts::PI).abs() < 1e-6);
}

#[test]
fn map_tool_icons_are_painted_inside_their_box() {
    // Painted line icons (mockup 2f), not font glyphs: each has strokes that
    // stay inside the 24-unit viewBox the painter scales to 18 px.
    use shell::ToolIcon::*;
    for icon in [Select, Front, Salient, Arrow, Objective, Undo, Redo] {
        let paths = shell::tool_icon_paths(icon);
        assert!(!paths.is_empty(), "{icon:?} has no strokes");
        for (pts, _) in paths {
            assert!(
                pts.len() >= 2,
                "{icon:?} has a stroke with {} points",
                pts.len()
            );
            for p in pts {
                assert!(
                    (1.0..=23.0).contains(&p.x) && (1.0..=23.0).contains(&p.y),
                    "{icon:?} point {p:?} outside the box"
                );
            }
        }
    }
    let icons: Vec<_> = MAP_TOOLS.iter().map(|t| t.1).collect();
    assert_eq!(
        icons[4], icons[5],
        "both objective tools use the flag, tinted by side"
    );
}

#[test]
fn side_marker_textures_are_registered_at_startup() {
    let h = Harness::new("sidetex");
    let tex = shell::side_textures(&h.ctx).expect("side textures registered on the first frame");
    // Cropped to the plane (no ring), square, and large enough to stay crisp.
    for t in &tex {
        let [w, h] = t.size();
        assert_eq!(w, h, "side marker texture is square");
        assert!(w >= 48, "side marker texture is {w} px");
    }
}

// ── Map: front strokes, tool switching, line clears (README §6.2, §6.3) ──

#[test]
fn map_front_strokes_undo_redo_and_escape_mid_drag() {
    let mut h = Harness::new("frontstrokes");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    let at = |dx: f32| map.center() + Vec2::new(dx, 0.0);
    h.key(Key::Num2);
    // Two clicks: two strokes of one point each.
    h.click_at(at(-200.0));
    h.click_at(at(-170.0));
    assert_eq!(h.app.custom_front_xz.len(), 2);
    assert_eq!(h.app.undo_label().as_deref(), Some("Drew front line"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(
        h.app.custom_front_xz.len(),
        1,
        "Ctrl Z takes back the last click"
    );
    h.key_with(Key::Y, CTRL);
    assert_eq!(h.app.custom_front_xz.len(), 2, "Ctrl Y puts it back");

    // A drag is one stroke, however many points it adds.
    h.drag(at(-140.0), at(-20.0));
    let after_drag = h.app.custom_front_xz.len();
    assert!(after_drag > 3, "the drag added points: {after_drag}");
    h.key_with(Key::Z, CTRL);
    assert_eq!(
        h.app.custom_front_xz.len(),
        2,
        "one Ctrl Z drops the whole drag"
    );
    h.key_with(Key::Y, CTRL);
    assert_eq!(h.app.custom_front_xz.len(), after_drag);

    // Esc during a drag drops that drag's points and returns to Select; the
    // rest of the drag moves nothing (not the AO either).
    let ao = h.app.front_aabb;
    h.press_and_move(at(0.0), at(60.0));
    assert!(
        h.app.custom_front_xz.len() > after_drag,
        "points are being added"
    );
    h.key(Key::Escape);
    assert_eq!(
        h.app.custom_front_xz.len(),
        after_drag,
        "Esc dropped the drag in progress"
    );
    assert!(h.app.map_drawing_mode == MapDrawingMode::None);
    h.move_to(at(120.0));
    h.release_at(at(120.0));
    assert_eq!(h.app.custom_front_xz.len(), after_drag);
    assert!(
        h.app.front_aabb == ao,
        "the cancelled drag did not draw an AO"
    );

    // Esc with no drag keeps the finished front.
    h.key(Key::Num2);
    h.key(Key::Escape);
    assert_eq!(h.app.custom_front_xz.len(), after_drag);

    // The palette's Undo does what Ctrl Z does, and says what it undoes.
    let undo = h.find("Undo: Drew front line");
    h.click_at(undo.rect.center());
    assert_eq!(h.app.custom_front_xz.len(), 2);
}

#[test]
fn map_switching_tools_drops_a_half_drawn_mark() {
    let mut h = Harness::new("toolswitch");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    // Salient in progress, then key 4: dropped, like Esc.
    h.key(Key::Num3);
    h.click_at(map.center());
    h.click_at(map.center() + Vec2::new(30.0, 10.0));
    assert!(!h.app.current_salient.is_empty());
    h.key(Key::Num4);
    assert!(
        h.app.current_salient.is_empty(),
        "key 4 dropped the salient"
    );
    assert!(h.app.map_drawing_mode == MapDrawingMode::AttackArrow);
    // Arrow drag in progress, then key 3: the arrow is dropped and the rest
    // of that drag draws nothing with the new tool.
    h.press_and_move(map.center(), map.center() + Vec2::new(80.0, -40.0));
    assert!(h.app.attack_drag.is_some(), "arrow being dragged");
    h.key(Key::Num3);
    assert!(h.app.attack_drag.is_none(), "key 3 dropped the arrow");
    h.move_to(map.center() + Vec2::new(120.0, -40.0));
    h.release_at(map.center() + Vec2::new(120.0, -40.0));
    assert!(h.app.attack_arrows.is_empty());
    assert!(
        h.app.current_salient.is_empty(),
        "the old drag did not start a salient"
    );
    // Salient in progress, then the palette's Attack arrow tool: dropped too.
    h.click_at(map.center());
    h.click_at(map.center() + Vec2::new(30.0, 10.0));
    assert!(!h.app.current_salient.is_empty());
    h.click("Attack arrow");
    assert!(
        h.app.current_salient.is_empty(),
        "the palette dropped the salient"
    );
    assert!(h.app.map_drawing_mode == MapDrawingMode::AttackArrow);
}

#[test]
fn map_clear_lines_salients_and_arrows_undo() {
    let mut h = Harness::new("clearlines");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    h.key(Key::Num4);
    h.drag(map.center(), map.center() + Vec2::new(120.0, -60.0));
    h.drag(
        map.center() + Vec2::new(0.0, 40.0),
        map.center() + Vec2::new(120.0, -20.0),
    );
    assert_eq!(h.app.attack_arrows.len(), 2);
    h.key(Key::Num2);
    h.click_at(map.center() + Vec2::new(-200.0, 0.0));
    h.click_at(map.center() + Vec2::new(-150.0, 0.0));
    h.key(Key::Escape);
    h.click("Period");

    h.click("Clear arrows");
    assert!(h.app.attack_arrows.is_empty());
    assert_eq!(h.app.undo_label().as_deref(), Some("Cleared 2 arrows"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.attack_arrows.len(), 2, "Ctrl Z restores the arrows");
    assert_eq!(h.app.custom_front_xz.len(), 2, "and leaves the front alone");

    h.click("Clear lines");
    assert!(h.app.attack_arrows.is_empty() && h.app.custom_front_xz.is_empty());
    assert_eq!(
        h.app.undo_label().as_deref(),
        Some("Cleared the drawn front and 2 arrows")
    );
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.attack_arrows.len(), 2);
    assert_eq!(h.app.custom_front_xz.len(), 2);
    // The marks came back with them: Ctrl Z now undoes the last drawing.
    assert_eq!(h.app.undo_label().as_deref(), Some("Drew front line"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.custom_front_xz.len(), 1);
}

#[test]
fn map_undo_label_matches_what_ctrl_z_undoes() {
    let mut h = Harness::new("undolabel");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    h.key(Key::Num4);
    h.drag(map.center(), map.center() + Vec2::new(120.0, -60.0));
    h.key(Key::Num5);
    h.click_at(map.center() + Vec2::new(-80.0, 0.0));
    assert_eq!(
        h.app.map_tool_status(),
        None,
        "one-shot objective tool is back to Select"
    );
    // Clear the objective, then draw: the drawing is undone first, then the Clear.
    h.click("Forces");
    let top = h.find("OBJECTIVES").rect.top();
    let clear = h
        .all("Clear DPRK")
        .into_iter()
        .find(|n| !n.disabled && n.rect.top() > top)
        .expect("objectives Clear DPRK");
    h.click_at(clear.rect.center());
    assert_eq!(
        h.app.undo_label().as_deref(),
        Some("Cleared 1 DPRK objectives")
    );
    h.key(Key::Num4);
    h.drag(
        map.center() + Vec2::new(0.0, 40.0),
        map.center() + Vec2::new(120.0, -20.0),
    );
    assert_eq!(h.app.attack_arrows.len(), 2);
    assert_eq!(h.app.undo_label().as_deref(), Some("Drew an attack arrow"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.attack_arrows.len(), 1);
    assert_eq!(
        h.app.undo_label().as_deref(),
        Some("Cleared 1 DPRK objectives")
    );
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.east_objectives.len(), 1, "then the Clear");
    assert_eq!(
        h.app.attack_arrows.len(),
        1,
        "without touching the older arrow"
    );
    assert_eq!(h.app.undo_label().as_deref(), Some("Drew an attack arrow"));
}

#[test]
fn map_objective_tool_hint_is_short() {
    let mut h = Harness::new("objhint");
    h.tab("Map");
    h.key(Key::Num5);
    assert_eq!(
        h.app.map_tool_status().as_deref(),
        Some("Tool: DPRK objective · Click to place · Shift for more")
    );
    h.key(Key::Num6);
    assert_eq!(
        h.app.map_tool_status().as_deref(),
        Some("Tool: NATO objective · Click to place · Shift for more")
    );
}

#[test]
fn map_dock_tabs_fit_a_three_digit_reference_count() {
    let mut h = Harness::new("docktabs");
    h.tab("Map");
    h.app.map_refs = (0..123)
        .map(|i| crate::frontlines::MapRefGroup {
            path: PathBuf::from(format!("ref_{i}.Group")),
            entity: crate::ast::Il2Entity::new("Group"),
        })
        .collect();
    h.settle();
    let tabs: Vec<Node> = ["Period", "Forces", "References 123", "Terrain", "Air"]
        .iter()
        .map(|l| h.find(l))
        .collect();
    // One row of cells, left to right, filling the 288 px dock content.
    for pair in tabs.windows(2) {
        assert!(
            (pair[1].rect.top() - pair[0].rect.top()).abs() < 0.5,
            "{:?} is on another row",
            pair[1].label
        );
        assert!(
            (pair[1].rect.left() - pair[0].rect.right()).abs() < 0.5,
            "{:?} does not follow {:?}",
            pair[1].label,
            pair[0].label
        );
    }
    let span = tabs[4].rect.right() - tabs[0].rect.left();
    assert!(
        (287.0..=288.5).contains(&span),
        "tabs span {span} px, not the 288 px dock"
    );
    // Each label (bold, as when selected) with its count fits its cell with padding.
    for ((_, label, count), node) in MapDock::tabs(123).into_iter().zip(&tabs) {
        let need = shell::dock_tab_needed_width(&h.ctx, label, count);
        let cell = node.rect.width();
        assert!(
            need + 2.0 * shell::DOCK_TAB_PAD <= cell + 0.5,
            "{label} {count:?} needs {need} px of a {cell} px cell"
        );
    }
    h.click("References 123");
    assert!(h.app.map_dock == MapDock::References);
}

// ── Help (README §6.4) ────────────────────────────────────────────────────

#[test]
fn help_closes_on_escape_and_shortcuts_still_work() {
    let mut h = Harness::new("helpesc");
    h.key(Key::F1);
    assert!(h.app.help_open);
    h.key(Key::Escape);
    assert!(!h.app.help_open, "Esc closes Help");
    h.key_with(Key::Num6, CTRL);
    assert!(h.app.mode == AppMode::Map, "Ctrl 6 still switches tabs");
    h.key(Key::F1);
    assert!(h.app.help_open && h.app.help_topic == HelpTopic::Front);
}

/// Every non-ASCII character in the UI code and the manual (shown as Help)
/// must exist in a loaded font; a missing one draws as an empty box (✓ did).
#[test]
fn every_ui_glyph_is_in_a_loaded_font() {
    let h = Harness::new("glyphs");
    // The check itself works: U+2713 (check mark) is in none of the fonts.
    assert!(
        !h.ctx
            .fonts(|f| f.has_glyph(&egui::FontId::proportional(13.0), '\u{2713}'))
    );
    let mut missing = std::collections::BTreeSet::new();
    let sources = [
        include_str!("ui.rs"),
        include_str!("ui/air.rs"),
        include_str!("ui/map.rs"),
        include_str!("ui/replay.rs"),
        include_str!("airtask/mod.rs"),
        include_str!("airtask/library.rs"),
        include_str!("airtask/pool.rs"),
        include_str!("airtask/standalone.rs"),
        include_str!("airtask/timing.rs"),
        include_str!("airtask/clock.rs"),
        include_str!("in_theatre.rs"),
        // These newly covered backends have Korean parser/JSON test data,
        // not UI copy. Keep the original UI-source coverage unchanged.
        include_str!("trace.rs").split("#[cfg(test)]").next().unwrap(),
        include_str!("missionlog.rs").split("#[cfg(test)]").next().unwrap(),
        include_str!("shell.rs"),
        include_str!("help.rs"),
        include_str!("../USER_MANUAL.md"),
    ];
    for text in sources {
        for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
            for ch in line.chars().filter(|c| !c.is_ascii()) {
                let ok = h
                    .ctx
                    .fonts(|f| f.has_glyph(&egui::FontId::proportional(13.0), ch));
                if !ok {
                    missing.insert(ch);
                }
            }
        }
    }
    assert!(missing.is_empty(), "glyphs no loaded font has: {missing:?}");
}

/// No control spills out of a side panel (README §4 widths). An overflowing
/// row (the Airfield folder rows did) pushes egui into an unpainted gap.
#[test]
fn side_panel_controls_stay_inside_their_panel() {
    // (tab, left panel width, right panel width), after the 172 px rail.
    const PANELS: [(&str, f32, f32); 7] = [
        ("Template", 270.0, 304.0),
        ("Army Generator", 330.0, 300.0),
        ("Fighter Pack", 270.0, 304.0),
        ("Exclusive Activation", 290.0, 290.0),
        ("Airfield", 290.0, 270.0),
        ("Map", 56.0, 304.0),
        ("Air Tasking", 270.0, 304.0),
    ];
    let mut h = Harness::new("panelfit");
    let mut out = Vec::new();
    for (tab, left_w, right_w) in PANELS {
        h.tab(tab);
        if tab == "Air Tasking" { fill_air_pool(&mut h); }
        let left = (shell::RAIL_W, shell::RAIL_W + left_w);
        let right = (SCREEN.x - right_w, SCREEN.x);
        let mut check = |h: &Harness, state: &str| { for n in &h.nodes {
            let control = matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton | Role::TextInput);
            if !control || n.rect.width() <= 0.0 || n.rect.top() < shell::HEADER_H {
                continue;
            }
            let x = n.rect.left();
            let inside = |(a, b): (f32, f32)| x >= a - 0.5 && x < b;
            if inside(left) && n.rect.right() > left.1 + 0.5 {
                out.push(format!("{tab} {state} left: {:?} {:?} ends at {}", n.label, n.role, n.rect.right()));
            }
            if inside(right) && n.rect.right() > right.1 + 0.5 {
                out.push(format!("{tab} {state} right: {:?} {:?} ends at {}", n.label, n.role, n.rect.right()));
            }
        }};
        check(&h, "initial");
        if tab == "Air Tasking" {
            // The settings panel scrolls: check each part, the Player count section included.
            let mut count_radius = false;
            air_scroll(&mut h, SCREEN.x - 100.0, 10000.0);
            for _ in 0..10 {
                check(&h, "settings");
                count_radius |= h.all("Count radius").iter().any(|n| n.role == Role::Slider);
                air_scroll(&mut h, SCREEN.x - 100.0, -220.0);
            }
            assert!(count_radius, "the Player count section was not reached");
            visit_air_category_controls(&mut h, check);
        }
    }
    assert!(
        out.is_empty(),
        "controls overflow their panel:\n{}",
        out.join("\n")
    );
}

/// A button label never wraps onto a second line ("Clear arrows" did in a
/// narrow add_enabled_ui child). Wrapped text makes the button ~44 px tall.
#[test]
fn button_labels_stay_on_one_line() {
    let mut h = Harness::new("onelinebuttons");
    let mut tall = Vec::new();
    for (_, label) in MODES {
        h.tab(label);
        if label == "Air Tasking" { fill_air_pool(&mut h); }
        if label == "Map" {
            for dock in ["Period", "Forces", "Terrain"] {
                h.click_prefix_if_present(dock);
                collect_tall_buttons(&h, &format!("Map › {dock}"), &mut tall);
            }
            // "Air" is also the start of the rail's "Air Tasking": click it exactly.
            h.click("Air");
            fill_air_pool(&mut h);
            collect_tall_buttons(&h, "Map › Air", &mut tall);
        }
        collect_tall_buttons(&h, label, &mut tall);
        if label == "Air Tasking" {
            visit_air_category_controls(&mut h, |h, state| collect_tall_buttons(h, &format!("Air Tasking › {state}"), &mut tall));
        }
    }
    tall.sort();
    tall.dedup();
    assert!(
        tall.is_empty(),
        "buttons with wrapped labels:\n{}",
        tall.join("\n")
    );
}

fn collect_tall_buttons(h: &Harness, tab: &str, out: &mut Vec<String>) {
    if tab.starts_with("Air Tasking") {
        assert!(!h.app.air_plan.pool.is_empty() && h.app.air_selected.is_some());
    }
    for n in &h.nodes {
        // Tree chips are 36 px by design; the Map dock tabs (40 px, top row
        // of the dock) are skipped. A wrapped label makes a button 40 px.
        let dock_tab = n.rect.top() < 100.0
            && (n.label == "Air"
                || ["Period", "Forces", "References", "Terrain"]
                    .iter()
                    .any(|t| n.label.starts_with(t)));
        if n.role == Role::Button
            && !dock_tab
            && n.rect.height() > 36.5
            && !n.label.is_empty()
            && n.rect.width() < 200.0
        {
            out.push(format!(
                "{tab}: {:?} {}x{}",
                n.label,
                n.rect.width(),
                n.rect.height()
            ));
        }
    }
}

impl Harness {
    fn click_prefix_if_present(&mut self, prefix: &str) {
        if let Some(n) = self
            .nodes
            .iter()
            .find(|n| n.role == Role::Button && n.label.starts_with(prefix))
            .cloned()
        {
            self.click_at(n.rect.center());
            self.settle();
        }
    }
}

/// Terrain heights apply on export unless this session turns the switch off.
#[test]
fn terrain_heights_on_export_start_on() {
    let mut h = Harness::new("terrainon");
    assert!(h.app.terrain_apply, "Apply terrain heights starts on");
    h.tab("Map");
    h.wait_for_map();
    h.click_prefix_if_present("Terrain");
    let n = h.find("Apply terrain heights on export");
    assert_eq!(n.toggled, Some(true));
}

#[test]
fn map_lock_ao_blocks_empty_drag_and_reset() {
    let mut h = Harness::new("lockao");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    let full = h.app.front_aabb;
    h.drag(
        map.center() + Vec2::new(-80.0, -40.0),
        map.center() + Vec2::new(80.0, 40.0),
    );
    assert_ne!(h.app.front_aabb, full, "unlocked drag sets the AO");
    let locked_box = h.app.front_aabb;
    h.click("Lock AO");
    assert!(h.app.ao_locked);
    assert!(h.find("Reset AO").disabled, "Reset AO is off while locked");
    h.drag(
        map.center() + Vec2::new(-40.0, -20.0),
        map.center() + Vec2::new(40.0, 20.0),
    );
    assert_eq!(h.app.front_aabb, locked_box, "locked drag leaves the AO");
    h.click("Unlock AO");
    assert!(!h.app.ao_locked);
    h.click("Reset AO");
    assert_eq!(h.app.front_aabb, full);
}

#[test]
fn map_ao_resize_and_reset_are_undoable() {
    let mut h = Harness::new("aoresize");
    h.tab("Map");
    h.wait_for_map();
    let map = h.find("Korea map").rect;
    let full = h.app.front_aabb;
    h.drag(
        map.center() + Vec2::new(-80.0, -40.0),
        map.center() + Vec2::new(80.0, 40.0),
    );
    let resized = h.app.front_aabb;
    assert_ne!(resized, full, "unlocked drag sets the AO");
    assert_eq!(h.app.undo_label().as_deref(), Some("Resized AO"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.front_aabb, full, "Ctrl Z restores the AO");
    assert!(h.app.undo_label().is_none());

    h.drag(
        map.center() + Vec2::new(-80.0, -40.0),
        map.center() + Vec2::new(80.0, 40.0),
    );
    let resized = h.app.front_aabb;
    h.click("Reset AO");
    assert_eq!(h.app.front_aabb, full);
    assert_eq!(h.app.undo_label().as_deref(), Some("Reset AO"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.front_aabb, resized, "Ctrl Z restores the resized AO");
}

#[test]
#[ignore = "records P14 UI baseline; run only before Part B or with explicit approval"]
fn write_p14_baseline_ui() {
    let mut h = Harness::new("p14_baseline_ui_writer");
    h.app.east_objectives = vec![(180_000.0, 240_000.0), (190_000.0, 260_000.0)];
    h.app.nato_objectives = vec![(140_000.0, 220_000.0), (150_000.0, 250_000.0)];
    h.app.attack_arrows = vec![((140_000.0, 220_000.0), (130_000.0, 240_000.0))];
    h.app.front_t = 8.0;
    h.app.terrain_apply = false;
    let dir = crate::baseline_tests::baseline_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("base_map_ui.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert_eq!(dialog::pending(), 0);
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    assert_group_file(&out);
}

#[test]
fn map_generate_without_air_matches_baseline() {
    let mut h = Harness::new("p14_baseline_ui_check");
    h.app.east_objectives = vec![(180_000.0, 240_000.0), (190_000.0, 260_000.0)];
    h.app.nato_objectives = vec![(140_000.0, 220_000.0), (150_000.0, 250_000.0)];
    h.app.attack_arrows = vec![((140_000.0, 220_000.0), (130_000.0, 240_000.0))];
    h.app.front_t = 8.0;
    h.app.terrain_apply = false;
    let out = h.out("base_map_ui.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert_eq!(dialog::pending(), 0);
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    crate::baseline_tests::assert_fixture_matches(&out);
}

// ── Trace build and Replay (step 9T) ─────────────────────────────────────

/// The Template tab set to the inputs of the `template.Group` baseline.
fn baseline_template_inputs(h: &mut Harness) {
    let opts = crate::baseline_tests::template_options();
    h.app.set_mode(AppMode::Template);
    // Generate names the template after this file's stem.
    h.app.tpl_path = Some(h.dir.join(format!("{}.Group", opts.name)));
    h.app.tpl_loaded_path = None;
    h.app.tpl_seats = opts.seats;
    h.app.tpl_zone_in = opts.zone_in;
    h.app.tpl_zone_out = opts.zone_out;
    h.app.tpl_place_layout = opts.place_layout;
    h.app.tpl_per_group = opts.per_group;
    h.app.tpl_bring_up = opts.bring_up;
    h.app.tpl_spawn_reset = opts.allow_multiple_spawns;
    h.app.tpl_spawn_cooldown_min = opts.spawn_cooldown_min;
    h.app.tpl_wp_speed = opts.waypoint_speed;
    h.app.tpl_wp_altitude = opts.waypoint_altitude;
    h.app.tpl_wp_priority = opts.waypoint_priority;
    h.app.tpl_zone_coalition = opts.zone_coalition;
    h.app.tpl_zone_out_enabled = opts.zone_out_enabled;
    h.app.terrain_apply = false;
    h.settle();
}

fn count_blocks(root: &crate::ast::Il2Entity, block_type: &str) -> usize {
    let mut n = 0;
    root.for_each(&mut |e| n += usize::from(e.block_type == block_type));
    n
}

#[test]
fn trace_checkbox_off_output_matches_baseline() {
    let mut h = Harness::new("trace_checkbox_off");
    baseline_template_inputs(&mut h);
    // Off on a fresh start: nothing remembers it between runs.
    assert_eq!(h.find("Trace build").toggled, Some(false));
    let out = h.out("template.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_eq!(dialog::pending(), 0);
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    assert!(!h.status().contains("Trace build"), "{}", h.status());
    crate::baseline_tests::assert_fixture_matches(&out);
    assert!(!out.with_extension("trace.json").exists());
}

#[test]
fn trace_checkbox_on_writes_sidecar() {
    let mut h = Harness::new("trace_checkbox_on");
    baseline_template_inputs(&mut h);
    h.click("Trace build");
    assert_eq!(h.find("Trace build").toggled, Some(true));
    let out = h.out("template.Group");
    dialog::answer(vec![out.clone()]);
    h.click("Generate File");
    assert_eq!(dialog::pending(), 0);
    let map = crate::trace::read_trace_sidecar(&out).expect("sidecar next to the .Group");
    let n = map.entries.len();
    assert!(n > 0, "the template has decision points");
    assert!(h.status().contains(&format!("Trace build: {n} breadcrumbs")), "{}", h.status());
    let written = load_group(&out).unwrap();
    let baseline = load_group(&crate::baseline_tests::baseline_dir().join("template.Group")).unwrap();
    assert_eq!(
        count_blocks(&written, "MCU_TR_MissionObjective"),
        count_blocks(&baseline, "MCU_TR_MissionObjective") + n
    );
    // Each breadcrumb hangs on its source.
    for entry in &map.entries {
        let mut hooked = false;
        written.for_each(&mut |e| {
            hooked |= e.index == Some(entry.source_index)
                && e.targets.contains(&entry.breadcrumb_index);
        });
        assert!(hooked, "{}", entry.source_name);
    }
}

// ── Air Tasking ──────────────────────────────────────────────────────────

fn air_targets(h: &mut Harness) {
    h.app.custom_front_xz = vec![(200_000.0, 0.0), (200_000.0, 499_200.0)];
    h.app.east_objectives = vec![(190_000.0, 200_000.0), (180_000.0, 250_000.0)];
    h.app.nato_objectives = vec![(210_000.0, 200_000.0)];
    h.app.attack_arrows = vec![
        ((220_000.0, 300_000.0), (180_000.0, 300_000.0)),
        ((180_000.0, 350_000.0), (220_000.0, 350_000.0)),
    ];
    h.app.air_plan.seed = 7;
}

fn fill_air_pool(h: &mut Harness) {
    use crate::airtask::{self, PoolEntry};
    air_targets(h);
    let entries = h.app.with_air_context(|ctx| h.app.air_plan.library.iter().map(|sortie| {
        PoolEntry::new(sortie.id.clone(), airtask::target_choices(ctx, sortie.side)[0].target)
    }).collect::<Vec<_>>());
    h.app.air_plan.pool.clear();
    for entry in entries { airtask::add_entry(&mut h.app.air_plan, entry); }
    h.app.air_selected = Some(0);
    h.settle();
}

fn air_scroll(h: &mut Harness, x: f32, delta: f32) {
    h.move_to(Pos2::new(x, 600.0));
    h.step(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, delta),
        modifiers: Modifiers::NONE,
    }]);
    for _ in 0..20 { h.step(Vec::new()); }
}

fn air_open_section(h: &mut Harness, title: &str) {
    let header = air_settings_control(h, title, Role::Button);
    h.click_at(header.rect.center());
}

/// Scrolls the settings panel from the top until a control is on screen.
fn air_settings_control(h: &mut Harness, label: &str, role: Role) -> Node {
    air_scroll(h, SCREEN.x - 100.0, 10000.0);
    for _ in 0..12 {
        if let Some(node) = h.nodes.iter().find(|n| n.label == label && n.role == role
            && n.rect.center().y > shell::HEADER_H && n.rect.center().y < SCREEN.y - shell::STATUS_H).cloned() {
            return node;
        }
        air_scroll(h, SCREEN.x - 100.0, -240.0);
    }
    panic!("no {role:?} {label:?} in the settings panel. Labels: {:?}", h.labels());
}

fn air_type(h: &mut Harness, field: &str, text: &str) {
    h.click(field);
    h.key_with(Key::A, CTRL);
    h.step(vec![Event::Text(text.into())]);
    h.settle();
}

fn air_choose_category(h: &mut Harness, category: &str) {
    air_scroll(h, shell::RAIL_W + 130.0, -10000.0);
    h.click("Sortie category");
    // The Categories tab may also contain a plain label with this name.
    let option = h.all(category).into_iter().find(|n| n.role == Role::Button).unwrap();
    h.click_at(option.rect.center());
    assert_eq!(h.app.air_plan.library[h.app.air_library_selected.unwrap()].category, category);
}

fn air_filter(h: &mut Harness, category: &str) {
    h.click("Pool category filter");
    h.click(category);
}

/// Exercise the new controls in their normal, popup and inline-edit states.
fn visit_air_category_controls(h: &mut Harness, mut check: impl FnMut(&Harness, &str)) {
    h.click("Pool category filter");
    check(h, "Pool filter popup");
    h.click("Strike");
    check(h, "Filtered pool");
    air_filter(h, "All categories");
    air_scroll(h, shell::RAIL_W + 130.0, -10000.0);
    h.click("Sortie category");
    check(h, "Sortie category popup");
    h.key(Key::Escape);
    h.click("Categories");
    check(h, "Categories");
    h.click("Rename Strike");
    check(h, "Rename category");
    h.click("Cancel");
    h.click("Pool");
}

#[test]
fn air_tasking_category_combo_is_undoable() {
    let mut h = Harness::new("air_category_combo");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let before = h.app.air_plan.clone();
    air_choose_category(&mut h, "Interdiction");
    assert_eq!(h.app.air_plan.library[0].category, "Interdiction");
    assert!(h.has("Sortie 1 category: Interdiction"));
    assert!(h.labels().iter().any(|s| s.contains("NATO · Objective 1 · Interdiction")));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn air_tasking_category_counts_use_singular_and_plural() {
    let mut h = Harness::new("air_category_counts");
    h.tab("Air Tasking");
    h.click("Categories");
    for (name, label) in [("Air alert", "1 sortie"), ("Interdiction", "0 sorties"), ("Strike", "3 sorties")] {
        let row = h.find(&format!("Rename {name}")).rect;
        assert!(h.all(label).iter().any(|n| (n.rect.center().y - row.center().y).abs() < 0.5), "{name}: missing {label}");
    }
    assert!(!h.has("1 sorties"));
}

#[test]
fn air_tasking_category_rows_keep_fixed_columns_while_renaming() {
    let mut h = Harness::new("air_category_columns");
    let long_name = "Night interdiction and maritime patrol along the entire northern coastline";
    crate::airtask::add_category(&mut h.app.air_plan, long_name).unwrap();
    h.tab("Air Tasking");
    h.click("Categories");
    let names = h.app.air_plan.categories.clone();
    for screen in [Vec2::new(1280.0, 800.0), SCREEN] {
        h.screen = screen;
        h.settle();
        let rename = h.find("Rename Air alert").rect;
        let remove = h.find("Remove category Air alert").rect;
        let name_left = h.find("Air alert").rect.left();
        let count_right = h.find("0 sorties").rect.right();
        for name in &names {
            let rename_row = h.find(&format!("Rename {name}")).rect;
            let remove_row = h.find(&format!("Remove category {name}")).rect;
            assert!((rename_row.left() - rename.left()).abs() < 0.5, "Rename {name}: {rename_row:?}");
            assert!((remove_row.left() - remove.left()).abs() < 0.5, "Remove {name}: {remove_row:?}");
            assert!((h.find(name).rect.left() - name_left).abs() < 0.5, "name must start at the left column edge: {name}");
            let count = h.nodes.iter().find(|n| n.role == Role::Label && (n.label.ends_with(" sortie") || n.label.ends_with(" sorties"))
                && (n.rect.center().y - rename_row.center().y).abs() < 0.5).unwrap();
            assert!((count.rect.right() - count_right).abs() < 0.5, "count must be right aligned: {count:?}");
        }
        let name_column = h.find(long_name).rect;
        let mut input_edges: Option<(f32, f32)> = None;
        for name in &names {
            h.click(&format!("Rename {name}"));
            let input = h.find("Rename category").rect;
            assert!((input.left() - name_column.left()).abs() < 0.5 && input.right() >= name_column.right(), "rename input must fill the name column: {input:?} vs {name_column:?}");
            if let Some((left, right)) = input_edges {
                assert!((input.left() - left).abs() < 0.5 && (input.right() - right).abs() < 0.5, "rename input changed column edges: {input:?}");
            }
            input_edges = Some((input.left(), input.right()));
            assert!((h.find("0 sorties").rect.right() - count_right).abs() < 0.5);
            assert!((h.find("Save").rect.left() - rename.left()).abs() < 0.5);
            assert!((h.find("Cancel").rect.left() - remove.left()).abs() < 0.5);
            h.click("Cancel");
        }
    }
}

#[test]
fn air_tasking_category_names_are_left_aligned_and_truncated_with_hover_text() {
    fn text_shape<'a>(shape: &'a egui::Shape, name: &str, bounds: Rect) -> Option<&'a egui::epaint::TextShape> {
        match shape {
            egui::Shape::Text(text) if text.galley.text() == name && bounds.contains(text.pos) => Some(text),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| text_shape(shape, name, bounds)),
            _ => None,
        }
    }
    let mut h = Harness::new("air_category_name_layout");
    h.screen = Vec2::new(1280.0, 800.0);
    let long_name = "Night interdiction and maritime patrol along the entire northern coastline";
    crate::airtask::add_category(&mut h.app.air_plan, long_name).unwrap();
    h.tab("Air Tasking");
    h.click("Categories");
    let out = h.ctx.run(RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, h.screen)),
        time: Some(h.time),
        ..Default::default()
    }, |ctx| h.app.ui(ctx));
    h.time += 1.0 / 60.0;
    for name in ["Air alert", long_name] {
        let text = out.shapes.iter().find_map(|s| text_shape(&s.shape, name, h.find(name).rect)).unwrap();
        assert_eq!(text.galley.job.halign, egui::Align::LEFT);
        assert!((text.pos.x - h.find(name).rect.left()).abs() < 0.5);
        assert_eq!(text.galley.elided, name == long_name);
        assert_eq!(text.galley.rows.len(), 1);
    }
    assert_eq!(h.all(long_name).len(), 1);
    h.move_to(h.find(long_name).rect.center());
    for _ in 0..60 { h.step(Vec::new()); }
    assert!(h.all(long_name).len() >= 2, "hover must show the full category name");
}

#[test]
fn air_tasking_categories_add_rename_remove_and_undo() {
    let mut h = Harness::new("air_categories_edit");
    h.tab("Air Tasking");
    h.click("Categories");
    let before = h.app.air_plan.clone();
    air_type(&mut h, "New category", " Night raids ");
    h.click("Add category");
    assert_eq!(h.app.air_plan.categories.last().unwrap(), "Night raids");
    assert!(h.has("Rename Night raids") && h.has("0 sorties"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
    air_type(&mut h, "New category", "Night raids");
    h.click("Add category");
    air_choose_category(&mut h, "Night raids");
    let before = h.app.air_plan.clone();
    h.click("Rename Night raids");
    air_type(&mut h, "Rename category", "Maritime");
    h.click("Save");
    assert_eq!(h.app.air_plan.library[0].category, "Maritime");
    assert!(h.has("Rename Maritime"));
    assert!(!h.app.air_plan.categories.iter().any(|s| s == "Night raids"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
    h.click("Remove category Night raids");
    assert_eq!(h.app.air_plan.library[0].category, "Air alert");
    assert!(!h.app.air_plan.categories.iter().any(|s| s == "Night raids"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn air_tasking_category_errors_are_inline_and_cancel_keeps_plan() {
    let mut h = Harness::new("air_categories_errors");
    h.tab("Air Tasking");
    h.click("Categories");
    let before = h.app.air_plan.clone();
    h.click("Add category");
    assert!(h.labels().iter().any(|s| s.contains("Enter a category name.")));
    air_type(&mut h, "New category", " STRIKE ");
    h.click("Add category");
    assert!(h.labels().iter().any(|s| s.contains("A category named STRIKE already exists.")));
    h.click("Rename Air alert");
    air_type(&mut h, "Rename category", "Strike");
    h.click("Save");
    assert!(h.labels().iter().any(|s| s.contains("A category named Strike already exists.")));
    h.click("Cancel");
    assert!(!h.has("Save"));
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn air_tasking_pool_category_filter_hides_rows_and_limits_auto_fill() {
    let mut h = Harness::new("air_category_filter");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    air_filter(&mut h, "Strike");
    assert!(h.has("POOL · 3 OF 6"));
    for entry in [2, 5, 6] { assert!(h.has(&format!("Entry {entry} weight"))); }
    for entry in [1, 3, 4] { assert!(!h.has(&format!("Entry {entry} weight"))); }
    let before = h.app.air_plan.clone();
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan.pool.len(), 11);
    assert_eq!(h.status(), "Added 5 entries · Strike");
    assert_eq!(&h.app.air_plan.pool[..6], &before.pool);
    assert!(h.app.air_plan.pool[6..].iter().all(|e| h.app.air_plan.sortie(&e.sortie_id).unwrap().category == "Strike"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
    air_filter(&mut h, "All categories");
    assert!(h.has("POOL · 6"));
    for entry in 1..=6 { assert!(h.has(&format!("Entry {entry} weight"))); }
}

#[test]
fn air_tasking_auto_fill_uses_singular_for_one_entry() {
    let mut h = Harness::new("air_autofill_singular");
    h.tab("Air Tasking");
    air_targets(&mut h);
    h.app.attack_arrows.clear();
    h.settle();
    air_filter(&mut h, "Air alert");
    h.click("Auto-fill from objectives");
    assert_eq!(h.status(), "Added 1 entry · Air alert");
    h.app.air_plan.pool.clear();
    h.app.air_plan.library.truncate(1);
    air_filter(&mut h, "All categories");
    h.click("Auto-fill from objectives");
    assert_eq!(h.status(), "Added 1 entry");
}

#[test]
fn air_tasking_loads_category_label_from_user_template() {
    let mut h = Harness::new("air_file_category");
    h.tab("Air Tasking");
    air_targets(&mut h);
    let input = std::fs::read_to_string(repo("TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group")).unwrap();
    let mut root = crate::parser::parse_group_file(&input).unwrap();
    root.set_property("Desc", "\"Notes\nCategory: Interdiction\"".to_string());
    let path = h.out("Interdiction.Group");
    std::fs::write(&path, crate::serialize::serialize_group(&root)).unwrap();
    dialog::answer(vec![path]);
    h.key_with(Key::O, CTRL);
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    assert_eq!(h.app.air_plan.library[6].category, "Interdiction");
    assert!(h.has("Sortie 7 category: Interdiction"));
    assert!(h.has("Sortie category"));
    h.click("Add to pool");
    root.set_property("Desc", "\"Category: sTrIkE\"".to_string());
    let path = h.out("MixedCase.Group");
    std::fs::write(&path, crate::serialize::serialize_group(&root)).unwrap();
    dialog::answer(vec![path]);
    h.key_with(Key::O, CTRL);
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    assert_eq!(h.app.air_plan.library[7].category, "sTrIkE", "file label keeps its spelling");
    assert_eq!(h.app.air_plan.categories.iter().filter(|c| c.eq_ignore_ascii_case("Strike")).count(), 1);
    h.click("Add to pool");
    air_filter(&mut h, "Strike");
    assert!(!h.has("Entry 1 weight"));
    assert!(h.has("Entry 2 weight"));
    assert_eq!(dialog::pending(), 0);
}

#[test]
fn air_tasking_six_builtins_show_decision_0031_categories() {
    let mut h = Harness::new("air_builtin_categories");
    h.tab("Air Tasking");
    assert_eq!(h.app.air_plan.categories, ["Air alert", "Strike", "Close support", "Armed recon", "Interdiction"]);
    let expected = ["Air alert", "Strike", "Close support", "Armed recon", "Strike", "Strike"];
    let mut shown = std::collections::BTreeSet::new();
    for _ in 0..6 {
        shown.extend(h.labels());
        air_scroll(&mut h, shell::RAIL_W + 130.0, -220.0);
    }
    for (i, category) in expected.into_iter().enumerate() {
        assert_eq!(h.app.air_plan.library[i].category, category);
        assert!(shown.contains(&format!("Sortie {} category: {category}", i + 1)));
    }
}

#[test]
fn air_tasking_mode_popup_does_not_select_the_row_under_it() {
    let mut h = Harness::new("air_mode_selection");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let first = h.find("Pool entry 1").rect;
    h.click_at(Pos2::new(first.left() + 80.0, first.center().y));
    assert_eq!(h.app.air_selected, Some(0));
    h.click("Entry 1 mode");
    let zone = h.find("Zone").rect.center();
    let underlying = h.nodes.iter().find(|n| n.label.starts_with("Pool entry ") && n.rect.contains(zone)).unwrap();
    assert_ne!(underlying.label, "Pool entry 1", "regression click must overlap another row");
    h.click_at(zone);
    assert_eq!(h.app.air_plan.pool[0].mode, crate::airtask::TriggerMode::Zone);
    assert_eq!(h.app.air_selected, Some(0));
    let second = h.find("Pool entry 2").rect;
    h.click_at(Pos2::new(second.left() + 80.0, second.center().y));
    assert_eq!(h.app.air_selected, Some(1));
}

#[test]
fn air_tasking_unnoted_draft_edit_is_committed_and_undoable() {
    let mut h = Harness::new("air_unnoted_edit");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let before = h.app.air_plan.clone();
    let mut draft = before.clone();
    draft.pool[0].weight = 9;
    h.app.apply_air_edit(draft, None);
    h.settle();
    assert_eq!(h.app.air_plan.pool[0].weight, 9);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn ctrl_7_opens_air_tasking() {
    let mut h = Harness::new("air_ctrl7");
    h.key_with(Key::Num7, CTRL);
    assert!(h.app.mode == AppMode::AirTasking);
    assert!(h.has("AIR TASKING"));
    h.key(Key::F1);
    assert!(h.app.help_topic == HelpTopic::AirTasking);
}

#[test]
fn plain_7_on_map_does_not_panic() {
    let mut h = Harness::new("air_plain7");
    h.tab("Map");
    h.key(Key::Num4);
    h.key(Key::Num7);
    assert!(h.app.mode == AppMode::Map);
    assert!(h.app.map_drawing_mode == MapDrawingMode::AttackArrow);
}

#[test]
fn air_tasking_undo_restores_weight_change() {
    let mut h = Harness::new("air_weight_undo");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let before = h.app.air_plan.pool.clone();
    let weight = h.find("Entry 1 weight").rect.center();
    h.drag(weight, weight + Vec2::new(40.0, 0.0));
    assert_ne!(h.app.air_plan.pool[0].weight, before[0].weight);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan.pool, before, "one Undo restores the start of the drag");
}

/// Decision 0047 (step 12e): one N for both sides, from 1 to 3, with Undo.
#[test]
fn air_tasking_threshold_n_runs_from_1_to_3() {
    let mut h = Harness::new("air_players");
    h.tab("Air Tasking");
    let players = |h: &Harness| h.all("Players").into_iter().find(|n| n.role == Role::SpinButton).unwrap();
    let at = players(&h).rect.center();
    assert!(!players(&h).disabled);
    assert!(h.has("1 player · 12 km"));
    h.drag(at, at + Vec2::new(400.0, 0.0));
    assert_eq!(h.app.air_plan.threshold.players_n, 3, "stops at 3");
    assert!(h.has("3 players · 12 km"));
    assert!(!h.has("Two or more players comes with the multi-player phase."));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan.threshold.players_n, 1, "one Undo restores the start of the drag");
    h.drag(at, at - Vec2::new(400.0, 0.0));
    assert_eq!(h.app.air_plan.threshold.players_n, 1, "stops at 1");
}

/// Decision 0050 D (step 12e): the count radius slider sets the plan, with
/// Undo, and the section names each side's areas (one per target).
#[test]
fn air_tasking_count_radius_slider_sets_the_plan() {
    let mut h = Harness::new("air_count_radius");
    h.tab("Air Tasking");
    assert!(h.has("Areas · DPRK 0 · NATO 0"));
    fill_air_pool(&mut h);
    assert!(h.has("Areas · DPRK 1 · NATO 1"), "{:?}", h.labels());
    assert!(h.has("50 km · 2 areas"));
    let slider = air_settings_control(&mut h, "Count radius", Role::Slider);
    assert!(!slider.disabled);
    h.drag(slider.rect.center(), Pos2::new(slider.rect.left() - 50.0, slider.rect.center().y));
    assert_eq!(h.app.air_plan.threshold.count_radius_m, 10_000.0, "stops at 10 km");
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan.threshold.count_radius_m, 50_000.0);
    let slider = air_settings_control(&mut h, "Count radius", Role::Slider);
    h.drag(slider.rect.center(), Pos2::new(slider.rect.right() + 50.0, slider.rect.center().y));
    assert_eq!(h.app.air_plan.threshold.count_radius_m, 100_000.0, "stops at 100 km");
}

/// Decisions 0050 D and 0051: a Map air start inside a count circle that
/// watches its side keeps Generate off, with the error as a Check, as soon as
/// it is placed (the check's cache key holds the spawns).
#[test]
fn air_tasking_air_start_inside_a_count_circle_disables_generate() {
    let mut h = Harness::new("air_count_spawn");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    assert!(!h.find("Generate File").disabled);
    let area = h.app.with_air_context(|ctx| crate::airtask::preview(&h.app.air_plan, ctx))
        .rings.iter().find(|r| r.side == crate::airtask::AirSide::Dprk && !r.inner).unwrap().centre;
    h.app.placed_air_starts.push(crate::airstart::PlacedAirStart {
        field: crate::airstart::default_field("NATO start", true),
        x: area.0 + 20_000.0,
        z: area.1,
    });
    h.settle();
    assert!(h.find("Generate File").disabled);
    let problem = h.app.air_idle_problem().unwrap();
    assert!(problem.contains("\"NATO start\" is 20.0 km away, inside the 50.0 km player count circle"), "{problem}");
    // Step 12e: lowering the radius in the tab turns Generate back on.
    let slider = air_settings_control(&mut h, "Count radius", Role::Slider);
    h.drag(slider.rect.center(), Pos2::new(slider.rect.left() - 50.0, slider.rect.center().y));
    assert_eq!(h.app.air_plan.threshold.count_radius_m, 10_000.0);
    assert!(!h.find("Generate File").disabled, "{:?}", h.app.air_idle_problem());
}

#[test]
fn air_plan_first_entry_ticks_include_in_base_map() {
    let mut h = Harness::new("air_first");
    h.tab("Air Tasking");
    air_targets(&mut h);
    h.settle();
    assert!(!h.app.air_plan.include_in_base_map);
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    h.click("Add to pool");
    assert_eq!(h.app.air_plan.pool.len(), 1);
    assert!(h.app.air_plan.include_in_base_map);
    h.key_with(Key::Z, CTRL);
    assert!(h.app.air_plan.pool.is_empty());
    assert!(!h.app.air_plan.include_in_base_map);
}

#[test]
fn air_tasking_tab_shows_builtin_library() {
    let mut h = Harness::new("air_library");
    h.tab("Air Tasking");
    let names = h.app.air_plan.library.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    assert_eq!(names.len(), 6);
    let mut shown = std::collections::BTreeSet::new();
    for _ in 0..5 {
        shown.extend(h.labels());
        air_scroll(&mut h, shell::RAIL_W + 130.0, -220.0);
    }
    for name in names { assert!(shown.contains(&name), "missing {name}"); }
    assert!(shown.contains("✔ Valid · war span (unsourced)"));
}

#[test]
fn air_tasking_builtin_remove_disabled() {
    let mut h = Harness::new("air_builtin_remove");
    h.tab("Air Tasking");
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    assert!(h.find("Remove").disabled);
    assert!(h.has("Built-in sorties stay in the library."));
    assert_eq!(h.app.air_plan.library.len(), 6);
}

#[test]
fn air_tasking_shows_totals() {
    let mut h = Harness::new("air_totals");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    air_scroll(&mut h, SCREEN.x - 100.0, -10000.0);
    let totals = h.app.with_air_context(|ctx| crate::airtask::totals(&h.app.air_plan, ctx));
    for (label, values) in [("Copies", totals.copies), ("Aircraft", totals.aircraft), ("MCUs", totals.mcus), ("Check zones", totals.check_zones)] {
        for (column, value) in [values[0], values[1], values[0] + values[1]].into_iter().enumerate() {
            assert!(h.has(&format!("{label} {}: {value}", ["DPRK", "NATO", "All"][column])), "{:?}", h.labels());
        }
    }
}

#[test]
fn air_tasking_widgets_are_28_px_with_a_filled_pool() {
    let mut h = Harness::new("air_sizes");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    h.ctx.style_mut(|style| style.animation_time = 0.0);
    h.app.air_plan.strategies.budget = crate::airtask::BudgetImpl::SingleSlot;
    h.app.air_plan.threshold.dwell_s = Some(120.0);
    h.app.air_plan.threshold.hold_s = Some(240.0);
    h.settle();
    for title in ["Schedule · DPRK", "Schedule · NATO", "Zone triggers", "Hard stop"] { air_open_section(&mut h, title); }
    air_scroll(&mut h, SCREEN.x - 100.0, 10000.0);
    let mut small = Vec::new();
    let mut disabled = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut check = |h: &Harness, _state: &str| {
        for n in &h.nodes {
            if matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton | Role::TextInput) && n.rect.width() > 0.0 {
                seen.insert(n.label.clone());
                if n.disabled { disabled.insert(n.label.clone()); }
                if n.rect.height() < 27.5 { small.push(format!("{:?} {:?}: {} px", n.label, n.role, n.rect.height())); }
            }
        }
    };
    for _ in 0..10 {
        check(&h, "settings");
        air_scroll(&mut h, SCREEN.x - 100.0, -220.0);
    }
    visit_air_category_controls(&mut h, &mut check);
    for field in ["Entry 6 weight", "Entry 6 runs", "Dwell D", "Hold H", "First slot", "Period", "Jitter steps", "× step", "Quiet", "Arrive in M", "Chance", "Cooldown", "Spawn distance", "Margin", "Extension", "Include in base map"] {
        assert!(seen.contains(field), "not checked: {field}; {seen:?}");
    }
    assert!(seen.contains("Players") && !disabled.contains("Players"));
    assert!(seen.contains("Count radius"));
    assert!(disabled.contains("Remove"));
    for field in ["Pool", "Categories", "Pool category filter", "Sortie category", "Rename Strike", "Remove category Strike", "New category", "Add category", "Rename category", "Save", "Cancel", "Interdiction"] {
        assert!(seen.contains(field), "category control not checked: {field}; {seen:?}");
    }
    assert!(small.is_empty(), "targets under 28 px, including disabled controls: {small:?}");
}

#[test]
fn air_tasking_controls_fit_the_minimum_window() {
    let mut h = Harness::new("air_minimum");
    h.screen = Vec2::new(1280.0, 800.0);
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let left_end = shell::RAIL_W + 270.0;
    let right_start = h.screen.x - 304.0;
    let check = |h: &Harness, state: &str| { for n in &h.nodes {
        if !matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton | Role::TextInput) || n.rect.width() == 0.0 {
            continue;
        }
        assert!(n.rect.right() <= h.screen.x + 0.5, "{state} beyond window: {n:?}");
        if n.rect.top() >= shell::HEADER_H {
            let x = n.rect.left();
            if (shell::RAIL_W..left_end).contains(&x) { assert!(n.rect.right() <= left_end + 0.5, "beyond library: {n:?}"); }
            if (left_end..right_start).contains(&x) { assert!(n.rect.right() <= right_start + 0.5, "beyond centre: {n:?}"); }
        }
    }};
    check(&h, "Pool");
    assert!(h.has("Entry 6 weight") && h.has("Entry 6 runs"));
    visit_air_category_controls(&mut h, check);
    for label in ["Pool", "Categories"] {
        let n = h.all(label).into_iter().find(|n| n.role == Role::Button).unwrap();
        assert!((n.rect.height() - 28.0).abs() < 0.5, "{n:?}");
        let size = h.ctx.fonts(|f| f.layout_no_wrap(label.into(), egui::FontId::new(13.0, theme::bold_family()), c::TEXT).size());
        assert!(size.x <= n.rect.width() && size.y <= 28.0, "{label} does not fit on one line");
    }
}

#[test]
fn air_tasking_library_footer_is_pinned_inside_the_minimum_panel() {
    let mut h = Harness::new("air_pinned_library_footer");
    h.screen = Vec2::new(1280.0, 800.0);
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    assert_eq!(h.app.air_plan.library.len(), 6);
    assert!(!h.app.air_plan.pool.is_empty());
    let panel = egui::containers::panel::PanelState::load(&h.ctx, egui::Id::new("air_left")).unwrap().rect;
    assert!((panel.width() - 270.0).abs() < 0.5, "{panel:?}");
    let category = h.find("Sortie category").rect;
    let target = h.all("NATO objective 1").into_iter().find(|n| n.role == Role::ComboBox).unwrap().rect;
    let add = h.find("Add to pool");
    let selected = h.find_prefix("Selected: ").rect;
    let hint = h.find("Built-in sorties stay in the library.").rect;
    for rect in [selected, category, target, add.rect, h.find("Remove").rect, hint] {
        assert!(panel.contains_rect(rect), "footer must be fully visible before scrolling: {rect:?} outside {panel:?}");
    }
    assert!(!add.disabled);
    assert!((category.right() - target.right()).abs() < 0.5, "combos must fill the same footer width");
    assert!(panel.right() - category.right() < 10.0, "combo must reach the footer's right edge");
    assert!(panel.bottom() - hint.bottom() < 10.0, "footer must sit at the bottom of the panel");
    let first_card = h.find("Library sortie 1").rect;
    h.move_to(first_card.center());
    h.step(vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -10000.0),
        modifiers: Modifiers::NONE,
    }]);
    for _ in 0..20 { h.step(Vec::new()); }
    let last_card = h.find("Library sortie 6").rect;
    assert!(panel.contains_rect(last_card) && last_card.bottom() < selected.top(), "cards must scroll above the footer: {last_card:?}");
    assert_eq!(h.find("Add to pool").rect, add.rect, "scrolling cards must not move the footer");
    assert_eq!(h.find("Sortie category").rect, category);
    assert_eq!(h.find("Built-in sorties stay in the library.").rect, hint);
    let before = h.app.air_plan.pool.len();
    h.click("Add to pool");
    assert_eq!(h.app.air_plan.pool.len(), before + 1);
}

#[test]
fn air_tasking_add_to_pool_uses_the_picked_target() {
    use crate::airtask::{AirSide, TargetRef};
    let mut h = Harness::new("air_target_pick");
    h.tab("Air Tasking");
    air_targets(&mut h);
    h.app.nato_objectives.push((230_000.0, 260_000.0));
    h.settle();
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    h.click("NATO objective 1");
    h.click("NATO objective 2");
    h.click("Add to pool");
    assert_eq!(h.app.air_plan.pool[0].target, TargetRef::Objective { side: AirSide::Nato, xz: (230_000.0, 260_000.0) });
}

#[test]
fn air_tasking_auto_fill_adds_each_target_once() {
    use crate::airtask::{self, AirSide};
    let mut h = Harness::new("air_autofill");
    h.tab("Air Tasking");
    air_targets(&mut h);
    h.settle();
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan.pool.len(), 2 * 3 + 4 * 2);
    assert_eq!(h.status(), "Added 14 entries");
    h.app.with_air_context(|ctx| {
        for sortie in &h.app.air_plan.library {
            for choice in airtask::target_choices(ctx, sortie.side) {
                assert_eq!(h.app.air_plan.pool.iter().filter(|e| e.sortie_id == sortie.id
                    && airtask::resolve_target(&e.target, ctx) == Some(choice)).count(), 1);
            }
        }
        assert_eq!(airtask::target_choices(ctx, AirSide::Dprk).len(), 3);
    });
    let before = h.app.air_plan.clone();
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan, before);
    assert_eq!(h.status(), "Added 0 entries");
    h.key_with(Key::Z, CTRL);
    assert!(h.app.air_plan.pool.is_empty(), "a no-op auto-fill keeps the previous undo");
}

#[test]
fn air_tasking_switched_off_entry_stays_in_pool() {
    let mut h = Harness::new("air_off");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let before = h.app.air_plan.pool.clone();
    h.click("Entry 1 enabled");
    assert_eq!(h.app.air_plan.pool.len(), 6);
    assert!(!h.app.air_plan.pool[0].enabled);
    assert_eq!(h.app.air_plan.pool[0].target, before[0].target);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan.pool, before);
}

#[test]
fn air_tasking_remove_user_template_takes_its_entries_and_undo_restores_both() {
    let mut h = Harness::new("air_remove_user");
    h.tab("Air Tasking");
    air_targets(&mut h);
    dialog::answer(vec![repo("TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group")]);
    h.key_with(Key::O, CTRL);
    air_scroll(&mut h, shell::RAIL_W + 130.0, -10000.0);
    h.click("Add to pool");
    let before = h.app.air_plan.clone();
    h.click("Remove");
    assert_eq!(h.app.air_plan.library.len(), 6);
    assert!(h.app.air_plan.pool.is_empty());
    let label = h.app.air_undo.label().unwrap();
    assert!(label.contains(&before.library[6].name) && label.ends_with("and 1 entry"));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn air_tasking_reset_keeps_library_and_draws_a_new_seed() {
    let mut h = Harness::new("air_reset");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    dialog::answer(vec![repo("TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group")]);
    h.key_with(Key::O, CTRL);
    h.app.air_plan.budget.per_side = [3, 4];
    h.app.air_plan.schedule[0].period_min = 37.0;
    h.app.air_plan.threshold.dwell_s = Some(90.0);
    crate::airtask::add_category(&mut h.app.air_plan, "Unused").unwrap();
    crate::airtask::set_category(&mut h.app.air_plan, 6, "Night raids");
    let before = h.app.air_plan.clone();
    h.click("Reset");
    assert!(h.app.confirm == Some(Confirm::ResetAirPlan));
    assert_eq!(h.app.air_plan, before);
    h.key(Key::Escape);
    assert!(h.app.confirm.is_none());
    assert_eq!(h.app.air_plan, before);
    h.click("Reset");
    let button = h.all("Reset").into_iter().max_by(|a, b| a.rect.top().total_cmp(&b.rect.top())).unwrap();
    h.click_at(button.rect.center());
    assert_ne!(h.app.air_plan.seed, before.seed);
    assert_eq!(h.app.air_plan, crate::airtask::AirPlan {
        library: before.library.clone(), seed: h.app.air_plan.seed,
        categories: ["Air alert", "Strike", "Close support", "Armed recon", "Interdiction", "Night raids"].map(str::to_owned).to_vec(),
        ..Default::default()
    });
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

#[test]
fn air_tasking_ctrl_o_adds_a_user_template() {
    let mut h = Harness::new("air_ctrl_o");
    h.tab("Air Tasking");
    let path = repo("TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group");
    dialog::answer(vec![path.clone()]);
    h.key_with(Key::O, CTRL);
    assert_eq!(h.app.air_plan.library.len(), 7);
    assert_eq!(h.app.air_plan.library[6].source, crate::airtask::SortieSource::File(path.clone()));
    dialog::answer(vec![path]);
    h.key_with(Key::O, CTRL);
    assert_eq!(h.app.air_plan.library.len(), 7, "same file is not added twice");
    let missing = h.out("missing.Group");
    dialog::answer(vec![missing]);
    h.key_with(Key::O, CTRL);
    assert!(matches!(h.app.status, Status::Error(_)));
    assert_eq!(h.app.air_plan.library.len(), 7);
    assert_eq!(dialog::pending(), 0);
}

#[test]
fn air_tasking_header_shows_map_date_and_entry_count() {
    let mut h = Harness::new("air_header");
    h.app.front_t = crate::frontlines::TIMELINE.iter().position(|m| (m.year, m.month, m.day) == (1950, 9, 15)).unwrap() as f32;
    h.tab("Air Tasking");
    assert!(h.has("Map date 15 Sep 1950 · 0 entries"));
    fill_air_pool(&mut h);
    assert!(h.has("Map date 15 Sep 1950 · 6 entries"));
    h.app.air_plan.pool.truncate(1);
    h.settle();
    assert!(h.has("Map date 15 Sep 1950 · 1 entry"));
}

#[test]
fn air_tasking_selected_entry_shows_hard_stop() {
    use crate::airtask::TriggerMode;
    let mut h = Harness::new("air_hard_stop");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    h.app.air_selected = Some(1); // HVAR: 600 s on station, 300 s clock arrival.
    h.settle();
    assert!(h.has("35 min"));
    h.app.air_plan.pool[1].mode = TriggerMode::Zone;
    h.settle();
    assert!(h.has("32.7 min"));
    h.app.air_plan.pool[1].mode = TriggerMode::Both;
    h.settle();
    assert!(h.has("32.7 min"));
    assert!(!h.has("35 min"));
    h.app.air_plan.library[1].wp_speed_kmh = 0.0;
    h.settle();
    assert!(h.has("—"));
    h.app.air_selected = Some(3); // Armed recon: 1200 s on station.
    h.settle();
    assert!(h.has("45 min"));
}

#[test]
fn air_tasking_orphaned_entry_shows_orphaned_and_a_cross() {
    let mut h = Harness::new("air_orphan");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    h.app.nato_objectives.clear();
    h.settle();
    assert!(h.has("Orphaned"));
    assert!(h.all("✖").iter().any(|node| node.role == Role::Label));
}

#[test]
fn air_tasking_manual_timing_and_heading_changes_undo() {
    let mut h = Harness::new("air_settings_undo");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let before = h.app.air_plan.clone();
    h.click("Override");
    assert_eq!(h.app.air_plan.pool[0].heading_override, Some(0.0));
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
    h.click("Manual timing");
    let timing = crate::airtask::derived_timing(&before.threshold);
    assert_eq!(h.app.air_plan.threshold.dwell_s, Some(timing.dwell_s));
    assert_eq!(h.app.air_plan.threshold.hold_s, Some(timing.hold_s));
    h.click("Manual timing");
    assert_eq!(h.app.air_plan.threshold.dwell_s, None);
    assert_eq!(h.app.air_plan.threshold.hold_s, None);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan.threshold.dwell_s, Some(timing.dwell_s));
}

#[test]
fn air_tasking_standalone_matches_public_builder_and_keeps_seed() {
    let mut h = Harness::new("air_standalone");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let seed = h.app.air_plan.seed;
    let expected = h.app.with_air_context(|ctx| crate::airtask::build_air_group(&h.app.air_plan, ctx)).unwrap();
    let path = h.out("Air.Group");
    dialog::answer(vec![path.clone()]);
    h.key_with(Key::G, CTRL);
    let text = std::fs::read_to_string(&path).unwrap();
    let root = crate::parser::parse_group_file(&text).unwrap();
    assert_eq!(root.name(), Some("Air Tasking"));
    assert_eq!(root.index, Some(1));
    assert_eq!(root, expected);
    assert_eq!(h.app.air_plan.seed, seed);
    dialog::answer(vec![path.clone()]);
    h.key_with(Key::G, CTRL);
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
}

#[test]
fn air_tasking_generate_disabled_until_validate_passes() {
    use crate::airtask::{self, IssueSeverity};
    let mut h = Harness::new("air_validate");
    h.tab("Air Tasking");
    air_targets(&mut h);
    for scenario in ["empty", "orphaned", "budget zero"] {
        match scenario {
            "empty" => h.app.air_plan.pool.clear(),
            "orphaned" => { fill_air_pool(&mut h); h.app.east_objectives.clear(); h.app.nato_objectives.clear(); },
            _ => { fill_air_pool(&mut h); h.app.air_plan.budget.per_side[0] = 0; },
        }
        h.settle();
        let errors = h.app.with_air_context(|ctx| airtask::validate(&h.app.air_plan, ctx))
            .into_iter().filter(|issue| issue.severity == IssueSeverity::Error).collect::<Vec<_>>();
        assert!(!errors.is_empty(), "{scenario} must have a validation error");
        assert!(h.find("Generate File").disabled, "{scenario}");
        h.key_with(Key::G, CTRL); // No dialog answer: disabled Ctrl G must not open one.
        let chip = h.find_prefix("⚠ ");
        h.click_at(chip.rect.center());
        for issue in errors { assert!(h.has(&issue.text), "{scenario}: missing Check text {}", issue.text); }
        h.key(Key::Escape);
    }
}

#[test]
fn air_tasking_generate_file_writes_air_pack() {
    let mut h = Harness::new("air_file_step7");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let path = h.out("Air.Group");
    dialog::answer(vec![path.clone()]);
    h.key_with(Key::G, CTRL);
    let root = crate::parser::parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(root.name(), Some("Air Tasking"));
    assert_eq!(root.block_type, "Group");
    let mut groups = Vec::new();
    let mut indexes = Vec::new();
    root.for_each(&mut |node| {
        if node.block_type == "Group" { groups.push(node.name().unwrap_or("").to_owned()); }
        if let Some(id) = node.index { indexes.push(id); }
    });
    assert_eq!(groups.iter().filter(|name| name.as_str() == "Air Tasking").count(), 1);
    for side in ["DPRK", "NATO"] {
        assert!(groups.iter().any(|name| name.contains(side) && name.contains("Logic")), "missing {side} Logic: {groups:?}");
    }
    indexes.sort_unstable();
    assert_eq!(indexes, (1..=indexes.len() as i32).collect::<Vec<_>>());
    root.for_each(&mut |node| {
        for id in node.targets.iter().chain(&node.objects) { assert!(indexes.binary_search(id).is_ok(), "dangling link {id}"); }
        for key in ["LinkTrId", "MisObjID", "TarId", "CmdId"] {
            if let Some(id) = node.property(key).and_then(|text| text.parse::<i32>().ok()).filter(|id| *id > 0) {
                assert!(indexes.binary_search(&id).is_ok(), "dangling {key} {id}");
            }
        }
    });
}

#[test]
fn air_tasking_totals_above_limit_are_orange_and_do_not_block_generate() {
    let mut h = Harness::new("air_limits");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    h.app.air_plan.limits = crate::airtask::TotalsLimits { aircraft: 0, mcus: 0, check_zones: 0 };
    h.settle();
    assert!(!h.find("Generate File").disabled);
    let issues = h.app.with_air_context(|ctx| crate::airtask::validate(&h.app.air_plan, ctx));
    assert!(issues.iter().any(|i| i.severity == crate::airtask::IssueSeverity::Warning));
    air_scroll(&mut h, SCREEN.x - 100.0, -10000.0);
    let totals = h.app.with_air_context(|ctx| crate::airtask::totals(&h.app.air_plan, ctx));
    let out = h.ctx.run(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)), ..Default::default() }, |ctx| h.app.ui(ctx));
    fn orange_at(shape: &egui::Shape, text: &str, bounds: Rect) -> bool {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().any(|s| orange_at(s, text, bounds)),
            egui::Shape::Text(t) => t.galley.job.text == text && bounds.expand(1.0).contains(t.pos)
                && t.galley.job.sections.iter().any(|s| s.format.color == c::WARN_TEXT),
            _ => false,
        }
    }
    for (label, values) in [("Aircraft", totals.aircraft), ("MCUs", totals.mcus), ("Check zones", totals.check_zones)] {
        let all = values[0] + values[1];
        assert!(all > 0);
        let node = h.find(&format!("{label} All: {all}"));
        assert!(out.shapes.iter().any(|s| orange_at(&s.shape, &all.to_string(), node.rect)), "{label} total is not orange");
    }
}

// ── P14 step 9: Air Tasking on the Map tab ──────────────────────────────

/// Issues, totals and preview come from one `airtask::check`, cached per plan
/// and Map inputs: one full evaluation per plan change, none per frame.
#[test]
fn air_checks_build_once_per_plan_change() {
    let mut h = Harness::new("air_cache");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let evaluations = crate::airtask::evaluations();
    assert!(evaluations > 0, "the Air Tasking tab evaluated the plan");
    for _ in 0..10 {
        h.step(Vec::new());
    }
    h.tab("Map");
    h.tab("Air Tasking");
    assert_eq!(crate::airtask::evaluations(), evaluations, "an unchanged plan is not evaluated again");
    h.app.air_plan.pool[0].weight = 9;
    h.settle();
    assert_eq!(crate::airtask::evaluations(), evaluations + 1, "one change, one evaluation");
}

/// The settings panel reads a draft of the plan while the other panels read
/// the live plan: the cache keeps both, so going back and forth costs nothing.
#[test]
fn air_check_cache_keeps_the_live_plan_and_a_draft() {
    let mut h = Harness::new("air_cache_draft");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    let mut draft = h.app.air_plan.clone();
    draft.pool[0].weight = 9;
    let before = crate::airtask::evaluations();
    h.app.with_air_context(|ctx| {
        for _ in 0..3 {
            h.app.air_eval(&h.app.air_plan, ctx);
            h.app.air_eval(&draft, ctx);
        }
    });
    assert_eq!(crate::airtask::evaluations(), before + 1, "only the draft is new");
}

/// Map tab, Air dock open, `air_targets` on the map and an empty pool.
/// DPRK has 2 date-valid built-ins and 3 targets (2 objectives, 1 arrow);
/// NATO has 4 and 2 (1 objective, 1 arrow): 6 and 8 entries.
fn open_map_air_dock(h: &mut Harness) {
    h.tab("Map");
    air_targets(h);
    h.app.air_plan.pool.clear();
    h.settle();
    h.click("Air");
}

/// The dock's DPRK / NATO toggle; the map legend has labels with the same text.
fn click_side(h: &mut Harness, side: &str) {
    let toggle = h.all(side).into_iter().find(|n| n.role == Role::Button).unwrap();
    h.click_at(toggle.rect.center());
}

fn entries_at(h: &Harness, target: crate::airtask::TargetRef) -> usize {
    h.app.air_plan.pool.iter().filter(|e| e.target == target).count()
}

#[test]
fn air_dock_auto_fill_adds_entries_for_objectives() {
    use crate::airtask::{AirSide, TargetRef};
    let mut h = Harness::new("air_dock_objectives");
    open_map_air_dock(&mut h);
    h.click("Auto-fill from objectives");
    assert_eq!(h.status(), "Added 6 DPRK entries");
    assert_eq!(h.app.air_plan.pool.len(), 6);
    // Plan 6.2: each new entry has the default weight (5, D15) and mode Clock.
    for entry in &h.app.air_plan.pool {
        assert_eq!((entry.weight, entry.mode), (5, crate::airtask::TriggerMode::Clock), "{entry:?}");
        assert!(entry.enabled);
    }
    for xz in [(190_000.0, 200_000.0), (180_000.0, 250_000.0)] {
        assert_eq!(entries_at(&h, TargetRef::Objective { side: AirSide::Dprk, xz }), 2, "{xz:?}");
    }
    let nato = TargetRef::Objective { side: AirSide::Nato, xz: (210_000.0, 200_000.0) };
    assert_eq!(entries_at(&h, nato), 0, "the NATO toggle is off");
    assert!(h.has("Entry 6 weight") && h.has("Entry 6 mode") && h.has("Remove entry 6"));
    click_side(&mut h, "NATO");
    assert!(!h.has("Entry 1 weight"), "the list shows one side");
    h.click("Auto-fill from objectives");
    assert_eq!(entries_at(&h, nato), 4);
}

#[test]
fn air_dock_auto_fill_includes_arrows() {
    use crate::airtask::TargetRef;
    let mut h = Harness::new("air_dock_arrows");
    open_map_air_dock(&mut h);
    // An arrow belongs to the side its tail is on: DPRK north of x = 200 km.
    let dprk_arrow = TargetRef::Arrow { tail: (220_000.0, 300_000.0), tip: (180_000.0, 300_000.0) };
    let nato_arrow = TargetRef::Arrow { tail: (180_000.0, 350_000.0), tip: (220_000.0, 350_000.0) };
    h.click("Auto-fill from objectives");
    assert_eq!((entries_at(&h, dprk_arrow), entries_at(&h, nato_arrow)), (2, 0));
    click_side(&mut h, "NATO");
    h.click("Auto-fill from objectives");
    assert_eq!(h.status(), "Added 8 NATO entries");
    assert_eq!((entries_at(&h, dprk_arrow), entries_at(&h, nato_arrow)), (2, 4));
}

#[test]
fn air_dock_auto_fill_twice_is_idempotent() {
    let mut h = Harness::new("air_dock_twice");
    open_map_air_dock(&mut h);
    h.click("Auto-fill from objectives");
    let before = h.app.air_plan.clone();
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan, before);
    assert_eq!(h.status(), "Added 0 DPRK entries");
}

#[test]
fn air_dock_draws_preview() {
    let mut h = Harness::new("air_dock_draw");
    open_map_air_dock(&mut h);
    h.wait_for_map();
    h.click("Auto-fill from objectives");
    click_side(&mut h, "NATO");
    h.click("Auto-fill from objectives");
    let preview = h.app.with_air_context(|ctx| crate::airtask::preview(&h.app.air_plan, ctx));
    assert_eq!(h.app.air_dock.drawn, preview.counts());
    assert!(preview.counts().lines > 0 && preview.counts().rings > 0, "{:?}", preview.counts());
    assert!(preview.counts().count_circles > 0, "{:?}", preview.counts());
    h.click("Approach lines");
    assert_eq!(h.app.air_dock.drawn.lines, 0, "SHOW hides the approach lines");
    assert_eq!(h.app.air_dock.drawn.rings, preview.counts().rings);
    h.click("Count circles");
    assert_eq!(h.app.air_dock.drawn.count_circles, 0, "SHOW hides the count circles");
    assert_eq!(h.app.air_dock.drawn.rings, preview.counts().rings);
}

#[test]
fn map_undo_restores_air_pool_after_auto_fill() {
    let mut h = Harness::new("air_dock_undo_fill");
    open_map_air_dock(&mut h);
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan.pool.len(), 6);
    h.key_with(Key::Z, CTRL);
    assert!(h.app.air_plan.pool.is_empty(), "Map's Ctrl Z undoes the dock's Auto-fill");
}

#[test]
fn map_undo_restores_air_entry_after_remove() {
    let mut h = Harness::new("air_dock_undo_remove");
    open_map_air_dock(&mut h);
    h.click("Auto-fill from objectives");
    h.key_with(Key::Z, CTRL);
    h.click("Auto-fill from objectives");
    let before = h.app.air_plan.clone();
    h.click("Remove entry 2");
    assert_eq!(h.app.air_plan.pool.len(), 5);
    assert_eq!(h.app.air_plan.pool[1], before.pool[2]);
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.air_plan, before);
}

/// Only the dock's edits put the plan in a Map undo: an Air Tasking edit
/// after a Map Clear keeps that undo, and Ctrl Z keeps the edit.
#[test]
fn map_undo_of_a_clear_keeps_later_air_settings() {
    let mut h = Harness::new("air_dock_undo_clear");
    open_map_air_dock(&mut h);
    h.click("Forces");
    // With no fighters or units placed, only the objectives' Clear DPRK is enabled.
    let clear = h.all("Clear DPRK").into_iter().find(|n| !n.disabled).unwrap();
    h.click_at(clear.rect.center());
    assert!(h.app.east_objectives.is_empty());
    h.app.air_plan.budget.per_side[0] = 2;
    h.settle();
    let edited = h.app.air_plan.clone();
    assert!(h.app.map_undo.peek().is_some(), "the Clear undo does not hold the plan, so it stays");
    h.key_with(Key::Z, CTRL);
    assert_eq!(h.app.east_objectives.len(), 2);
    assert_eq!(h.app.air_plan, edited);
}

/// The inputs `map_generate_without_air_matches_baseline` uses.
fn baseline_map_inputs(h: &mut Harness) {
    h.app.east_objectives = vec![(180_000.0, 240_000.0), (190_000.0, 260_000.0)];
    h.app.nato_objectives = vec![(140_000.0, 220_000.0), (150_000.0, 250_000.0)];
    h.app.attack_arrows = vec![((140_000.0, 220_000.0), (130_000.0, 240_000.0))];
    h.app.front_t = 8.0;
    h.app.terrain_apply = false;
}

fn map_air_check(h: &Harness) -> Option<shell::Check> {
    h.app
        .readiness_checks()
        .into_iter()
        .find(|c| c.text == "Air Tasking plan is valid (see the Air Tasking tab)")
}

#[test]
fn map_readiness_includes_air_check_when_included() {
    let mut h = Harness::new("air_map_ready");
    h.tab("Map");
    assert!(map_air_check(&h).is_none(), "an empty pool adds no check");
    fill_air_pool(&mut h);
    assert!(h.app.air_plan.include_in_base_map);
    assert!(map_air_check(&h).is_some_and(|c| c.ok));
    assert!(!h.find("Generate Base Map").disabled);
    h.app.air_plan.budget.per_side[0] = 0;
    h.settle();
    assert!(map_air_check(&h).is_some_and(|c| !c.ok), "budget 0 blocks Map like build_air_packs does");
    assert!(h.find("Generate Base Map").disabled);
    h.app.air_plan.include_in_base_map = false;
    h.settle();
    assert!(map_air_check(&h).is_none(), "an unticked plan does not touch the base map");
    assert!(!h.find("Generate Base Map").disabled);
}

#[test]
fn map_readiness_orphaned_only_air_pool_warns_but_allows_generate() {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef};
    let mut h = Harness::new("air_map_orphaned");
    h.tab("Map");
    baseline_map_inputs(&mut h);
    let gone = TargetRef::Objective { side: AirSide::Dprk, xz: (10_000.0, 10_000.0) };
    let id = h.app.air_plan.library.iter().find(|s| s.side == AirSide::Dprk).unwrap().id.clone();
    airtask::add_entry(&mut h.app.air_plan, PoolEntry::new(id, gone));
    h.app.air_plan.include_in_base_map = true;
    h.settle();
    // The note sits under the dock tabs, so every dock tab shows it.
    for tab in ["Air", "Forces"] {
        h.click(tab);
        assert!(h.has("Air Tasking has no valid entries and will be left out"), "{tab}");
    }
    assert!(map_air_check(&h).is_some_and(|c| c.ok));
    assert!(!h.find("Generate Base Map").disabled);
    let out = h.out("base_map_ui.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    crate::baseline_tests::assert_fixture_matches(&out);
}

#[test]
fn map_generate_with_air_includes_air_tasking_group() {
    let mut h = Harness::new("air_map_generate");
    h.tab("Map");
    fill_air_pool(&mut h);
    h.app.terrain_apply = false;
    let out = h.out("base_map_air.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let root = crate::parser::parse_group_file(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let mut air = 0;
    root.for_each(&mut |node| {
        air += usize::from(node.block_type == "Group" && node.name() == Some("Air Tasking"));
    });
    assert_eq!(air, 1, "one Air Tasking group in the base map");
}

/// A reference group near the NATO objective: a truck on 601, an AI plane on
/// 501 and a co-op player plane on 601.
const REF_UNITS_0037: &str = r#"Group
{
  Name = "Ref units";
  Index = 1;
  Desc = "";
  Vehicle
  {
    Index = 2;
    Name = "Ref truck";
    LinkTrId = 0;
    XPos = 211000.000;
    YPos = 0.000;
    ZPos = 201000.000;
    XOri = 0.00;
    YOri = 0.00;
    ZOri = 0.00;
    Country = 601;
  }
  Plane
  {
    Index = 3;
    Name = "Ref AI";
    LinkTrId = 0;
    XPos = 212000.000;
    YPos = 1000.000;
    ZPos = 202000.000;
    XOri = 0.00;
    YOri = 0.00;
    ZOri = 0.00;
    Country = 501;
    CoopStart = 0;
  }
  Plane
  {
    Index = 4;
    Name = "Ref coop";
    LinkTrId = 0;
    XPos = 213000.000;
    YPos = 1000.000;
    ZPos = 203000.000;
    XOri = 0.00;
    YOri = 0.00;
    ZOri = 0.00;
    Country = 601;
    CoopStart = 1;
  }
}
"#;

/// A strike mission's group: one AI plane on 501.
const STRIKE_0037: &str = r#"Group
{
  Name = "Strike AI";
  Index = 1;
  Desc = "";
  Plane
  {
    Index = 2;
    Name = "Strike AI lead";
    LinkTrId = 0;
    XPos = 180000.000;
    YPos = 1500.000;
    ZPos = 210000.000;
    XOri = 0.00;
    YOri = 0.00;
    ZOri = 0.00;
    Country = 501;
    CoopStart = 0;
  }
}
"#;

/// (block, Country) of every node outside the Air Tasking group, in file order.
fn map_countries(node: &crate::ast::Il2Entity, out: &mut Vec<(String, String)>) {
    if node.block_type == "Group" && node.name() == Some("Air Tasking") {
        return;
    }
    if let Some(c) = node.property("Country") {
        let coop = node.property("CoopStart").is_some_and(|s| s != "0");
        let block = if coop { format!("{} coop", node.block_type) } else { node.block_type.clone() };
        out.push((block, c.to_string()));
    }
    for child in &node.children {
        map_countries(child, out);
    }
}

fn generate_map(h: &mut Harness, file: &str) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    let out = h.out(file);
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let root = crate::parser::parse_group_file(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let mut countries = Vec::new();
    map_countries(&root, &mut countries);
    (root, countries)
}

fn country_of<'a>(root: &'a crate::ast::Il2Entity, name: &str) -> Option<&'a str> {
    root.find_by_name(name).and_then(|n| n.property("Country"))
}

/// Forces › Fighters › Place NATO: the Map's own NATO fighter pack.
fn place_nato_fighters(h: &mut Harness) {
    h.click("Forces");
    // The Fighters section's Place NATO (Units has one too, further down).
    let place = h
        .all("Place NATO")
        .into_iter()
        .min_by(|a, b| a.rect.top().total_cmp(&b.rect.top()))
        .expect("Place NATO");
    h.click_at(place.rect.center());
    assert!(h.app.map_fighters.as_ref().is_some_and(|l| !l.eastern), "{}", h.status());
}

/// Map AI on both player countries, plus an Airfield, a co-op plane and a truck.
fn map_ai_inputs(h: &mut Harness) {
    h.tab("Map");
    h.wait_for_map();
    fill_air_pool(h);
    place_nato_fighters(h);
    h.app.placed_air_starts.push(crate::airstart::PlacedAirStart {
        field: crate::airstart::default_field("NATO start", true),
        x: 215_000.0,
        z: 205_000.0,
    });
    // The air start is 25.5 km from DPRK area 1: keep it outside the player
    // count circle (decision 0050 D; the radius control comes in step 12e).
    h.app.air_plan.threshold.count_radius_m = 15_000.0;
    h.app.map_refs = vec![crate::frontlines::MapRefGroup {
        path: PathBuf::from("Ref units.Group"),
        entity: crate::parser::parse_group_file(REF_UNITS_0037).unwrap(),
    }];
    h.app.map_missions = vec![super::map::PlacedMission {
        name: "Strike AI".into(),
        eastern: true,
        root: crate::parser::parse_group_file(STRIKE_0037).unwrap(),
        target_x: 210_000.0,
        target_z: 200_000.0,
        start_x: 180_000.0,
        start_z: 210_000.0,
        ip_x: 195_000.0,
        ip_z: 205_000.0,
        target_label: "Custom".into(),
        target_class: None,
        attacks: Vec::new(),
    }];
    h.app.terrain_apply = false;
}

/// Decision 0037 A: with Air Tasking in the base map, the Map's own AI aircraft
/// are written on 602 / 503. Player spawns and ground units keep their countries.
#[test]
fn map_generate_with_air_moves_map_ai_off_player_countries() {
    let mut h = Harness::new("air_map_0037");
    map_ai_inputs(&mut h);
    let pool = std::mem::take(&mut h.app.air_plan.pool);
    h.app.air_plan.include_in_base_map = false;
    let (plain_root, plain) = generate_map(&mut h, "base_map_0037_plain.Group");
    h.app.air_plan.pool = pool;
    h.app.air_plan.include_in_base_map = true;
    let (air_root, air) = generate_map(&mut h, "base_map_0037_air.Group");

    let has = |list: &[(String, String)], block: &str, country: &str| {
        list.iter().any(|(b, c)| b == block && c == country)
    };
    // Without Air Tasking the Map writes its fighters on the player country.
    assert!(has(&plain, "Plane", "601"), "{plain:?}");
    assert!(has(&plain, "Plane", "501"), "the reference group's AI plane");
    for block in ["Vehicle", "Airfield", "Plane coop"] {
        assert!(has(&plain, block, "601"), "{block} in {plain:?}");
    }
    // With it, every AI plane moves; nothing else changes.
    let expected: Vec<(String, String)> = plain
        .iter()
        .map(|(block, country)| {
            let moved = match (block.as_str(), country.as_str()) {
                ("Plane", "601") => "602",
                ("Plane", "501") => "503",
                _ => country.as_str(),
            };
            (block.clone(), moved.to_string())
        })
        .collect();
    assert_eq!(air, expected);
    // Each writer: the fighter pack, the reference group and the strike mission.
    assert_eq!(country_of(&plain_root, "Ref AI"), Some("501"));
    assert_eq!(country_of(&air_root, "Ref AI"), Some("503"));
    assert_eq!(country_of(&plain_root, "Strike AI lead"), Some("501"));
    assert_eq!(country_of(&air_root, "Strike AI lead"), Some("503"));
    assert_eq!(country_of(&air_root, "Ref coop"), Some("601"));
    // The whole file, Air Tasking included: no AI plane on a player country,
    // and the Air Tasking entry triggers still watch 601 and 501.
    let mut watched = std::collections::BTreeSet::new();
    air_root.for_each(&mut |node| {
        let country = node.property("Country").unwrap_or("");
        if node.block_type == "Plane" && node.property("CoopStart") == Some("0") {
            assert!(!["601", "501"].contains(&country), "{:?} on {country}", node.name());
        }
        if node.block_type == "MCU_TR_ComplexTrigger" {
            watched.insert(country.to_string());
        }
    });
    assert!(watched.contains("601") && watched.contains("501"), "{watched:?}");
}

fn generate_map_from_button(h: &mut Harness, file: &str) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    h.settle();
    let out = h.out(file);
    dialog::answer(vec![out.clone()]);
    h.click("Generate Base Map");
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let root = crate::parser::parse_group_file(&std::fs::read_to_string(out).unwrap()).unwrap();
    let mut countries = Vec::new();
    map_countries(&root, &mut countries);
    (root, countries)
}

/// Decision 0054: the companion base map moves Map AI even though the Air
/// Tasking group will be generated separately. Decision 0037's spawns stay.
#[test]
fn map_generate_0054_standalone_moves_map_ai_off_player_countries() {
    let mut h = Harness::new("air_map_standalone_0054");
    map_ai_inputs(&mut h);
    h.app.air_plan.include_in_base_map = false;
    let pool = std::mem::take(&mut h.app.air_plan.pool);
    let (_, plain) = generate_map_from_button(&mut h, "base_map_0054_plain.Group");
    h.app.air_plan.pool = pool;
    assert!(!h.app.air_plan.pool.is_empty());
    let (root, countries) = generate_map_from_button(&mut h, "base_map_0054_standalone.Group");

    assert!(root.find_by_name("Air Tasking").is_none());
    assert_eq!(country_of(&root, "Ref AI"), Some("503"));
    assert_eq!(country_of(&root, "Strike AI lead"), Some("503"));
    assert!(countries.contains(&("Plane".into(), "602".into())), "the NATO fighter pack");
    for block in ["Vehicle", "Airfield", "Plane coop"] {
        assert!(countries.contains(&(block.into(), "601".into())), "{block} stays on 601");
    }
    // Exactly the mapping in threshold::ai_country_for_player, and no other
    // country changes: Airfields, co-op planes and ground units stay as built.
    let expected: Vec<_> = plain
        .into_iter()
        .map(|(block, country)| {
            let country = match (block.as_str(), country.as_str()) {
                ("Plane", "501") => "503".into(),
                ("Plane", "601") => "602".into(),
                _ => country,
            };
            (block, country)
        })
        .collect();
    assert_eq!(countries, expected);
}

#[test]
fn map_generate_0054_empty_pool_keeps_map_ai_on_player_countries() {
    let mut h = Harness::new("air_map_empty_0054");
    map_ai_inputs(&mut h);
    h.app.air_plan.pool.clear();
    for include in [false, true] {
        h.app.air_plan.include_in_base_map = include;
        let (root, countries) = generate_map_from_button(&mut h, "base_map_0054_empty.Group");
        assert!(root.find_by_name("Air Tasking").is_none());
        assert_eq!(country_of(&root, "Ref AI"), Some("501"));
        assert_eq!(country_of(&root, "Strike AI lead"), Some("501"));
        assert_eq!(country_of(&root, "Ref coop"), Some("601"));
        for block in ["Plane", "Vehicle", "Airfield", "Plane coop"] {
            assert!(countries.contains(&(block.into(), "601".into())), "{block} stays on 601");
        }
        assert!(countries.iter().all(|(block, c)| block != "Plane" || c == "501" || c == "601"), "{countries:?}");
    }
}

/// Decision 0042 A: a DPRK air start placed on the Map is written on the
/// country the NATO Air Tasking areas watch for DPRK players, so a player who
/// spawns there can start a NATO sortie.
#[test]
fn dprk_air_start_is_on_the_country_air_tasking_watches() {
    let mut h = Harness::new("air_map_0042");
    h.tab("Map");
    h.wait_for_map();
    fill_air_pool(&mut h);
    h.app.placed_air_starts.push(crate::airstart::PlacedAirStart {
        field: crate::airstart::default_field("DPRK start", false),
        x: 230_000.0,
        z: 205_000.0,
    });
    // The air start is 20.6 km from NATO area 1: keep it outside the player
    // count circle (decision 0050 D; the radius control comes in step 12e).
    h.app.air_plan.threshold.count_radius_m = 15_000.0;
    h.app.terrain_apply = false;
    let (root, _) = generate_map(&mut h, "base_map_0042.Group");

    let field = root.find_by_name("DPRK start").expect("the air start group");
    let airfield = field
        .children
        .iter()
        .find(|c| c.block_type == "Airfield")
        .expect("its Airfield");
    let nato = root
        .find_by_name("Air Tasking NATO Logic")
        .expect("NATO Air Tasking");
    let mut watched = std::collections::BTreeSet::new();
    nato.for_each(&mut |node| {
        if node.block_type == "MCU_TR_ComplexTrigger" {
            watched.insert(node.property("Country").unwrap_or("").to_string());
        }
    });
    assert_eq!(watched.len(), 1, "{watched:?}");
    assert_eq!(airfield.property("Country"), watched.first().map(String::as_str));
}

/// `replay.Group` traced at A, B and C, with its sidecar, in the harness dir.
/// With the `chain` logs, A fires; B (3 s on, unconditional) and C never do.
fn replay_chain_group(h: &Harness) -> PathBuf {
    use crate::trace::{ObjectiveStyle, TraceCarrier, TraceMap, TraceSelect};
    let mut group = load_group(&repo("src/testdata/missionlog/replay.Group")).unwrap();
    let mut map = TraceMap::default();
    crate::trace::instrument(
        &mut group,
        &TraceSelect { indexes: vec![1, 3, 4], ..TraceSelect::default() },
        &mut 100,
        TraceCarrier::Objective(ObjectiveStyle::default()),
        &mut map,
    ).expect("trace indexes fit");
    let path = h.out("chain.Group");
    let text = serialize_group(&group);
    std::fs::write(&path, &text).unwrap();
    write_trace(&path, &text, Some(&map)).unwrap();
    path
}

fn a1_group() -> PathBuf {
    repo("handoff/p14-probes/acceptance-a1/P14_Acceptance_A1.Group")
}

/// Open Replay…, pick the .Group and the log folder, and run it.
fn run_replay(h: &mut Harness, group: &Path, logs: &Path) {
    h.click("Replay…");
    assert!(h.find("Run replay").disabled, "needs a .Group and a log file");
    dialog::answer(vec![group.to_path_buf()]);
    h.click("Pick .Group…");
    dialog::answer(vec![logs.to_path_buf()]);
    h.click("Log folder…");
    h.click("Run replay");
    assert_eq!(dialog::pending(), 0);
    assert!(h.app.replay.loaded.is_some(), "{:?}", h.app.replay.error);
}

const NO_SIDECAR: &str = "No .trace.json next to this .Group: the report has no breadcrumb sections.";

#[test]
fn review_replay_selects_one_session_newest_first() {
    let mut h = Harness::new("review_sessions");
    let group = replay_chain_group(&h);
    let logs = h.out("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let old = "2026-10-30_12-00-00";
    let new = "2026-11-01_09-00-00";
    for (session, n, lines) in [
        (old, 0, "T:503 AType:8 POS(5000,0,5000)\nT:653 AType:8 POS(5010,0,5000)"),
        (new, 0, "T:503 AType:8 POS(5000,0,5000)"),
        (new, 10, "T:1503 AType:21 USERID:observer"),
        (new, 2, "T:1003 AType:21 USERID:observer"),
    ] {
        std::fs::write(logs.join(format!("missionReport({session})[{n}].txt")), lines).unwrap();
    }
    run_replay(&mut h, &group, &logs);
    let replay = &h.app.replay.loaded.as_ref().unwrap().replay;
    assert_eq!(replay.dead_links.len(), 1, "the old session's B must not hide the new session's dead link");
    assert_eq!(replay.dead_links[0].to, 101, "B is missing in the new session");
    let names: Vec<_> = replay.files.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, [0, 2, 10].map(|n| format!("missionReport({new})[{n}].txt")));
    assert!(h.find("Log session").rect.height() >= 27.5);
    h.click("Log session");
    h.click(old);
    assert!(h.app.replay.loaded.is_none(), "changing sessions clears the old report");
    h.click("Run replay");
    assert!(h.app.replay.loaded.as_ref().unwrap().replay.dead_links.is_empty());
}

#[test]
fn review_replay_rescans_after_picking_an_empty_folder() {
    let mut h = Harness::new("review_rescan");
    let group = replay_chain_group(&h);
    let logs = h.out("logs");
    std::fs::create_dir_all(&logs).unwrap();
    h.click("Replay…");
    dialog::answer(vec![group]);
    h.click("Pick .Group…");
    dialog::answer(vec![logs.clone()]);
    h.click("Log folder…");
    std::fs::write(logs.join("missionReport(rescan)[0].txt"), "T:503 AType:8 POS(5000,0,5000)").unwrap();
    assert!(!h.find("Run replay").disabled, "Run can rescan an initially empty folder");
    h.click("Run replay");
    assert!(!h.has("Log session"), "one session needs no picker");
    std::fs::write(logs.join("missionReport(rescan)[1].txt"), "T:653 AType:8 POS(5010,0,5000)\nT:1503 AType:21 USERID:observer").unwrap();
    h.click("Run replay");
    let replay = &h.app.replay.loaded.as_ref().unwrap().replay;
    assert_eq!(replay.files.len(), 2);
    assert_eq!(replay.firings[1].times_s.len(), 1, "Run reads the new [1] file");
}

#[test]
fn review_replay_marks_discarded_lines_incomplete() {
    let mut h = Harness::new("review_malformed");
    let group = replay_chain_group(&h);
    let logs = h.out("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("missionReport(bad)[0].txt"), "T:503 AType:8 POS(5000,0,5000)\nbroken header\nT:no AType:8\nT:1503 AType:21 USERID:observer").unwrap();
    run_replay(&mut h, &group, &logs);
    assert!(h.labels().iter().any(|s| s.contains("incomplete") && s.contains("2 malformed lines")), "{:?}", h.labels());
    for line in [2, 3] {
        assert!(h.labels().iter().any(|s| s.contains(&format!("missionReport(bad)[0].txt:{line}"))), "{:?}", h.labels());
    }
    assert!(h.status().contains("incomplete"));
}

#[test]
fn review_replay_refuses_logs_without_valid_records() {
    let mut h = Harness::new("review_empty_records");
    let group = replay_chain_group(&h);
    let logs = h.out("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let file = logs.join("missionReport(empty)[0].txt");
    h.click("Replay…");
    dialog::answer(vec![group]);
    h.click("Pick .Group…");
    dialog::answer(vec![logs]);
    h.click("Log folder…");
    for contents in ["", "malformed\nT:bad AType:8"] {
        std::fs::write(&file, contents).unwrap();
        h.click("Run replay");
        assert!(h.app.replay.loaded.is_none(), "no success report without records");
        assert!(!h.app.replay.layer);
        assert!(matches!(h.app.status, Status::Error(_)), "{}", h.status());
        assert!(h.app.replay.error.as_ref().is_some_and(|e| e.contains("no valid records")));
    }
}

#[test]
fn review_replay_empty_trace_has_no_breadcrumb_coverage() {
    let mut h = Harness::new("review_empty_trace");
    let group = h.out("empty.Group");
    std::fs::copy(repo("src/testdata/missionlog/replay.Group"), &group).unwrap();
    crate::trace::write_trace_sidecar(&group, &crate::trace::TraceMap::default()).unwrap();
    run_replay(&mut h, &group, &repo("src/testdata/missionlog/chain"));
    assert!(h.labels().iter().any(|s| s.contains("no breadcrumb coverage")), "{:?}", h.labels());
}

#[test]
fn review_trace_overflow_shows_error_and_writes_nothing() {
    let mut h = Harness::new("review_overflow_ui");
    h.tab("Airfield");
    let root = crate::parser::parse_group_file(&format!(
        "Group {{ MCU_TR_MissionBegin {{ Index = {}; Targets = []; }} }}", i32::MAX
    )).unwrap();
    h.app.airfield_root = Some(root);
    h.app.trace_build = true;
    h.app.export_airfield();
    h.settle();
    assert!(matches!(h.app.status, Status::Error(_)) && h.status().contains("i32 index range"), "{}", h.status());
    assert!(std::fs::read_dir(&h.dir).unwrap().next().is_none());
    assert_eq!(dialog::pending(), 0, "refused before opening the save dialog");
}

#[test]
fn review_replay_refuses_a_changed_timer() {
    let mut h = Harness::new("review_changed_timer");
    let group = replay_chain_group(&h);
    run_replay(&mut h, &group, &repo("src/testdata/missionlog/chain"));
    let text = std::fs::read_to_string(&group).unwrap();
    assert!(text.contains("Time = 2;"));
    std::fs::write(&group, text.replacen("Time = 2;", "Time = 3;", 1)).unwrap();
    h.click("Run replay");
    assert!(h.app.replay.loaded.is_none());
    assert!(h.app.replay.error.as_ref().is_some_and(|e| e.contains("the .Group changed since its trace was written; regenerate the trace build")));
    assert!(matches!(h.app.status, Status::Error(_)));
}

#[test]
fn review_replay_legacy_trace_shows_cannot_verify() {
    let mut h = Harness::new("review_legacy_trace");
    run_replay(&mut h, &a1_group(), &repo("src/testdata/missionlog/real_46"));
    assert!(h.labels().iter().any(|s| s.contains("cannot verify")), "{:?}", h.labels());
}

#[test]
fn review_trace_save_refuses_non_group_paths_before_writing() {
    for mode in ["Template", "Map", "Air Tasking", "Fighter Pack"] {
        let mut h = Harness::new(&format!("review_save_{mode}"));
        match mode {
            "Template" => baseline_template_inputs(&mut h),
            "Map" => baseline_map_inputs(&mut h),
            "Air Tasking" => { h.tab(mode); fill_air_pool(&mut h); }
            _ => h.tab(mode),
        }
        h.app.trace_build = true;
        for name in ["mission", "mission.txt", "mission.trace.json"] {
            let out = h.out(name);
            std::fs::write(&out, b"keep this mission").unwrap();
            let before = std::fs::read_dir(&h.dir).unwrap().count();
            dialog::answer(vec![out.clone()]);
            h.key_with(Key::G, CTRL);
            assert_eq!(dialog::pending(), 0);
            assert!(matches!(h.app.status, Status::Error(_)) && h.status().contains(".Group"), "{mode}: {}", h.status());
            assert_eq!(std::fs::read(&out).unwrap(), b"keep this mission", "{mode}: {name}");
            assert!(!out.with_extension("eng").exists());
            assert_eq!(std::fs::read_dir(&h.dir).unwrap().count(), before, "no sidecars written");
        }
        let out = h.out("accepted.gRoUp");
        dialog::answer(vec![out.clone()]);
        h.key_with(Key::G, CTRL);
        assert!(matches!(h.app.status, Status::Info(_)), "{mode}: {}", h.status());
        assert!(out.is_file() && out.with_extension("trace.json").is_file());
    }
}

#[test]
fn review_trace_bands_are_disjoint() {
    let mut h = Harness::new("review_trace_bands");
    h.tab("Map");
    fill_air_pool(&mut h);
    h.app.terrain_apply = false;
    h.app.trace_build = true;
    let map_path = h.out("map.Group");
    dialog::answer(vec![map_path.clone()]);
    h.app.generate_front_file();
    let map = crate::trace::read_trace_sidecar(&map_path).unwrap();
    h.tab("Air Tasking");
    let air_path = h.out("air.Group");
    dialog::answer(vec![air_path.clone()]);
    h.key_with(Key::G, CTRL);
    let air = crate::trace::read_trace_sidecar(&air_path).unwrap();
    assert_eq!(map.band, Some(crate::trace::TraceBand::Map));
    assert_eq!(air.band, Some(crate::trace::TraceBand::AirTasking));
    for (path, trace) in [(&map_path, &map), (&air_path, &air)] {
        assert_eq!(trace.group_fingerprint, Some(crate::trace::fingerprint(&std::fs::read_to_string(path).unwrap())));
    }
    assert!(!map.entries.is_empty() && !air.entries.is_empty());
    assert!(map.entries.iter().all(|m| air.entries.iter().all(|a| m.pos != a.pos)), "Map and standalone Air breadcrumbs collide");
    baseline_template_inputs(&mut h);
    let group_path = h.out("group.Group");
    dialog::answer(vec![group_path.clone()]);
    h.key_with(Key::G, CTRL);
    let group = crate::trace::read_trace_sidecar(&group_path).unwrap();
    assert_eq!(group.band, Some(crate::trace::TraceBand::Group));
    assert!(!group.entries.is_empty());
    assert!(group.entries.iter().all(|g| map.entries.iter().chain(&air.entries).all(|e| g.pos != e.pos)));
}

#[test]
fn replay_panel_shows_dead_links_and_unobserved_successors() {
    let mut h = Harness::new("replay_chain");
    let group = replay_chain_group(&h);
    run_replay(&mut h, &group, &repo("src/testdata/missionlog/chain"));
    let a = "Replay synthetic / A (MCU_Timer #1)";
    let b = "Replay synthetic / B (MCU_Timer #3)";
    // Downstream cells name the MCU without its group path.
    let c = "C (MCU_Timer #4)";
    // Dead links is the first section: A -> B, fired at 10 s, 3 s delay.
    assert!(h.has("Dead links 1") && h.has(a) && h.has(b), "{:?}", h.labels());
    h.click("Unobserved 1");
    assert!(h.has(crate::missionlog::UNOBSERVED_NOTE), "the observations note shows");
    assert!(h.has(b) && h.has(a) && h.has(c), "successor, upstream and downstream: {:?}", h.labels());
    assert!(!h.has(NO_SIDECAR));
}

#[test]
fn replay_panel_without_sidecar_has_no_breadcrumb_sections() {
    let mut h = Harness::new("replay_no_sidecar");
    let group = h.out("plain.Group");
    std::fs::copy(repo("src/testdata/missionlog/replay.Group"), &group).unwrap();
    run_replay(&mut h, &group, &repo("src/testdata/missionlog/synthetic"));
    assert!(h.has(NO_SIDECAR));
    for section in ["Dead links", "Unobserved", "Never fired"] {
        assert!(!h.labels().iter().any(|l| l.starts_with(section)), "{section} shown");
    }
    // Never spawned is the first section without a trace.
    assert!(h.has("F-80C") && h.has("Never spawned 4"), "{:?}", h.labels());
    h.click("Timeline 10");
    assert!(h.has("Spawn: F-80C / f80c10 (id 101, country 601, parent 0)"), "{:?}", h.labels());
}

/// The A1 acceptance flight (real logs) through the panel: the counts on the
/// section tabs equal the report the `replay` command line writes.
#[test]
fn replay_panel_reads_the_a1_logs() {
    let mut h = Harness::new("replay_a1");
    let logs = repo("src/testdata/missionlog/real_46");
    run_replay(&mut h, &a1_group(), &logs);
    let parsed = crate::missionlog::parse_log_dir(&logs).unwrap();
    let group = load_group(&a1_group()).unwrap();
    let trace = crate::trace::read_trace_sidecar(&a1_group()).unwrap();
    let replay = crate::missionlog::replay(&parsed, &group, Some(&trace));
    let rows = crate::missionlog::report_rows(&replay);
    assert!(rows.dead_links.is_empty(), "A1 had no dead links");
    for label in [
        "Dead links 0".to_string(),
        format!("Unobserved {}", rows.unobserved.len()),
        format!("Never fired {}", rows.never_fired_total),
        format!("Never spawned {}", rows.never_spawned.len()),
        format!("Timeline {}", replay.timeline.len()),
    ] {
        assert!(h.has(&label), "{label}: {:?}", h.labels());
    }
    h.click(&format!("Unobserved {}", rows.unobserved.len()));
    assert!(h.has(&rows.unobserved[0].successor), "{}", rows.unobserved[0].successor);
}

#[test]
fn replay_panel_filters_the_timeline_by_group() {
    let mut h = Harness::new("replay_filter");
    let group = h.out("plain.Group");
    std::fs::copy(repo("src/testdata/missionlog/replay.Group"), &group).unwrap();
    let logs = h.out("logs");
    std::fs::create_dir_all(&logs).unwrap();
    let lines = [
        "T:100 AType:12 ID:500 TYPE:GAZ-AA COUNTRY:601 NAME:Only truck PID:-1 POS(110000.0,10.0,210000.0)",
        "T:110 AType:12 ID:501 TYPE:Ship COUNTRY:601 NAME:Only ship PID:-1 POS(120000.0,0.0,220000.0)",
    ];
    std::fs::write(logs.join("missionReport(filter)[0].txt"), lines.join("\n")).unwrap();
    run_replay(&mut h, &group, &logs);
    h.click("Timeline 2");
    let truck = "Spawn: Only truck / GAZ-AA (id 500, country 601, parent -1)";
    let ship = "Spawn: Only ship / Ship (id 501, country 601, parent -1)";
    assert!(h.has(truck) && h.has(ship), "{:?}", h.labels());
    h.app.replay.group_filter = Some(vec!["Replay synthetic".into(), "Copy 1".into()]);
    h.settle();
    assert!(h.has(truck) && !h.has(ship), "the other copy's ship is filtered out");
}

#[test]
fn replay_panel_widgets_are_28_px() {
    let mut h = Harness::new("replay_sizes");
    h.click("Replay…");
    let mut small = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let check = |h: &Harness, small: &mut Vec<String>, seen: &mut std::collections::BTreeSet<String>| {
        for n in &h.nodes {
            let control = matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton);
            if !control || n.rect.width() <= 0.0 || n.rect.top() < shell::HEADER_H {
                continue;
            }
            seen.insert(n.label.clone());
            if n.rect.height() < 27.5 {
                small.push(format!("{:?} {:?}: {} px", n.label, n.role, n.rect.height()));
            }
        }
    };
    // Run replay is disabled before anything is picked; it is checked too.
    check(&h, &mut small, &mut seen);
    dialog::answer(vec![a1_group()]);
    h.click("Pick .Group…");
    dialog::answer(vec![repo("src/testdata/missionlog/real_46")]);
    h.click("Log folder…");
    h.click("Run replay");
    for prefix in ["Dead links", "Unobserved", "Never fired", "Fired", "Never spawned", "Timeline"] {
        let tab = h.find_prefix(prefix);
        h.click_at(tab.rect.center());
        h.settle();
        check(&h, &mut small, &mut seen);
    }
    for label in ["Pick .Group…", "Trace file…", "Log folder…", "Run replay", "Close", "Timeline group"] {
        assert!(seen.contains(label), "not checked: {label}; {seen:?}");
    }
    small.sort();
    small.dedup();
    assert!(small.is_empty(), "Replay panel targets under 28 px: {small:?}");
}

/// A .trace.json left beside a file regenerated without trace no longer
/// matches it: Run replay refuses it and says why.
#[test]
fn replay_refuses_a_sidecar_from_another_build() {
    let mut h = Harness::new("replay_stale_sidecar");
    let group = replay_chain_group(&h);
    std::fs::copy(repo("src/testdata/missionlog/replay.Group"), &group).unwrap();
    h.click("Replay…");
    dialog::answer(vec![group.clone()]);
    h.click("Pick .Group…");
    dialog::answer(vec![repo("src/testdata/missionlog/chain")]);
    h.click("Log folder…");
    h.click("Run replay");
    assert!(h.app.replay.loaded.is_none());
    let err = h.app.replay.error.clone().expect("an error shows");
    assert!(err.contains("chain.trace.json") && err.contains("from another build"), "{err}");
    h.settle();
    assert!(h.has(&err));
}

/// A new pick makes the shown report stale: it and the Map layer go.
#[test]
fn replay_new_pick_clears_the_report_and_map_layer() {
    let mut h = Harness::new("replay_new_pick");
    let group = replay_chain_group(&h);
    run_replay(&mut h, &group, &repo("src/testdata/missionlog/chain"));
    assert!(h.app.replay.layer);
    dialog::answer(vec![a1_group()]);
    h.click("Pick .Group…");
    assert!(h.app.replay.loaded.is_none() && !h.app.replay.layer);
    assert!(!h.labels().iter().any(|l| l.starts_with("Dead links")));
}

/// Drawn inside the app, the Replay window takes Esc: it closes, and an
/// active Map tool stays active.
#[test]
fn esc_closes_an_embedded_replay_and_nothing_else() {
    let mut h = Harness::new("replay_esc");
    h.tab("Map");
    h.app.pick_map_tool(MapDrawingMode::PlaceEastObjective);
    h.click("Replay…");
    assert!(h.app.replay.open && h.app.replay.embedded);
    h.key(Key::Escape);
    h.settle();
    assert!(!h.app.replay.open);
    assert!(h.app.map_drawing_mode == MapDrawingMode::PlaceEastObjective);
}

/// Long MCU paths wrap inside their column: no table text runs past the
/// window's right edge (the live run of 2026-10-28 found the upstream and
/// downstream columns pushed out of view).
#[test]
fn replay_tables_stay_inside_the_window() {
    let mut h = Harness::new("replay_table_width");
    run_replay(&mut h, &a1_group(), &repo("src/testdata/missionlog/real_46"));
    let right = h.find("Close").rect.right();
    for prefix in ["Unobserved", "Never spawned", "Timeline"] {
        let tab = h.find_prefix(prefix);
        h.click_at(tab.rect.center());
        h.settle();
        let tabs_bottom = tab.rect.bottom();
        let outside: Vec<_> = h
            .nodes
            .iter()
            // Table text only: the app's own panels sit behind the embedded window.
            .filter(|n| {
                ["Korea Base Map", "Breadcrumb:", "Spawn:", "Kill:", "Upstream fired", "Downstream", "Group"]
                    .iter()
                    .any(|p| n.label.starts_with(p))
            })
            .filter(|n| n.rect.top() > tabs_bottom && n.rect.width() > 0.0 && n.rect.right() > right + 1.0)
            .map(|n| format!("{:?} ends at {}", n.label, n.rect.right()))
            .collect();
        assert!(outside.is_empty(), "{prefix}: past the window edge {right}: {outside:?}");
        // No cell taller than a few lines: a row must fit the window.
        let tall: Vec<_> = h
            .nodes
            .iter()
            .filter(|n| n.label.starts_with("Korea Base Map") && n.rect.top() > tabs_bottom)
            .filter(|n| n.rect.height() > 160.0)
            .map(|n| format!("{} px: {:.60}", n.rect.height(), n.label))
            .collect();
        assert!(tall.is_empty(), "{prefix}: cells too tall: {tall:?}");
        assert!(h.labels().iter().any(|l| l.starts_with("Korea Base Map")), "{prefix}: table text checked");
        if prefix == "Unobserved" {
            assert!(h.has("Upstream fired"), "the column header is in view");
        }
    }
}

/// At the app's minimum window, a loaded replay keeps the legend usable:
/// "All layers" and the Replay toggle stay clear of the zoom chip (the live
/// run of 2026-10-28 found them pushed under it).
#[test]
fn map_legend_fits_with_a_replay_at_minimum_width() {
    let mut h = Harness::new("replay_legend_width");
    h.screen = Vec2::new(1280.0, 800.0);
    h.tab("Map");
    h.wait_for_map();
    run_replay(&mut h, &a1_group(), &repo("src/testdata/missionlog/real_46"));
    h.click("Close");
    h.settle();
    let zoom_left = h.find("−").rect.left();
    for label in ["All layers ▾", "Replay"] {
        let n = h.find(label);
        assert!(n.rect.right() < zoom_left, "{label} ends at {} under the zoom chip at {zoom_left}", n.rect.right());
    }
}

#[test]
fn map_replay_layer_draws_spawns() {
    let mut h = Harness::new("replay_map_layer");
    h.tab("Map");
    // The legend and the layer draw once the map texture has loaded.
    h.wait_for_map();
    assert!(!h.has("Replay"), "no Replay layer until a replay is loaded");
    assert_eq!(h.app.replay.drawn, [0, 0]);
    run_replay(&mut h, &a1_group(), &repo("src/testdata/missionlog/real_46"));
    h.click("Close");
    h.settle();
    let markers = &h.app.replay.loaded.as_ref().unwrap().replay.markers;
    let spawns = markers.iter().filter(|m| m.kind == crate::missionlog::MarkerKind::Spawn).count();
    let kills = markers.len() - spawns;
    assert!(spawns > 0 && kills > 0, "A1 has spawns and kills");
    assert_eq!(h.find("Replay").toggled, Some(true), "on once loaded");
    assert_eq!(h.app.replay.drawn, [spawns, kills]);
    h.click("Replay");
    h.settle();
    assert_eq!(h.app.replay.drawn, [0, 0], "the layer turns off");
}

/// The Air Tasking tab's Generate File: P14 names get breadcrumbs too.
#[test]
fn air_tasking_trace_build_includes_role_names() {
    let mut h = Harness::new("air_tab_trace");
    h.tab("Air Tasking");
    fill_air_pool(&mut h);
    h.app.trace_build = true;
    let out = h.out("air_trace.Group");
    dialog::answer(vec![out.clone()]);
    h.key_with(Key::G, CTRL);
    assert_eq!(dialog::pending(), 0);
    let map = crate::trace::read_trace_sidecar(&out).unwrap();
    assert!(h.status().contains(&format!("Trace build: {} breadcrumbs.", map.entries.len())), "{}", h.status());
    let starts = map.entries.iter().filter(|e| e.source_name.ends_with(" START")).count();
    assert!(starts > 0, "sortie START timers traced");
}

/// The .Group was written though its language files failed: the trace
/// sidecar is still written, and the status says both.
#[test]
fn trace_sidecar_is_written_when_language_files_fail() {
    let mut h = Harness::new("trace_lang_fail");
    h.tab("Map");
    fill_air_pool(&mut h);
    h.app.terrain_apply = false;
    h.app.trace_build = true;
    let out = h.out("lang_fail.Group");
    std::fs::create_dir_all(out.with_extension("eng")).unwrap();
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Error(_)), "{}", h.status());
    assert!(h.status().contains("language files failed") && h.status().contains("Trace build:"), "{}", h.status());
    assert!(crate::trace::read_trace_sidecar(&out).is_ok());
}

/// On the Map tab a trace build follows decision points across the file and
/// the P14 names: the sorties' START and DONE HUB timers get breadcrumbs.
#[test]
fn map_trace_build_includes_air_tasking_roles() {
    let mut h = Harness::new("air_map_trace");
    h.tab("Map");
    fill_air_pool(&mut h);
    h.app.terrain_apply = false;
    h.app.trace_build = true;
    let out = h.out("base_map_air_trace.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let map = crate::trace::read_trace_sidecar(&out).unwrap();
    assert!(h.status().ends_with(&format!("Trace build: {} breadcrumbs.", map.entries.len())));
    let roles = |suffix: &str| {
        map.entries
            .iter()
            .filter(|e| e.source_type == "MCU_Timer" && e.source_name.ends_with(suffix))
            .count()
    };
    assert!(roles(" START") > 0 && roles(" DONE HUB") > 0, "P14 names traced");
    assert!(map.entries.iter().any(|e| e.source_type == "MCU_CheckZone"));
}

#[test]
fn map_generate_with_include_unticked_matches_baseline() {
    use crate::airtask::{self, PoolEntry};
    let mut h = Harness::new("air_map_unticked");
    h.tab("Map");
    baseline_map_inputs(&mut h);
    let entries = h.app.with_air_context(|ctx| {
        h.app.air_plan.library.iter().map(|sortie| {
            PoolEntry::new(sortie.id.clone(), airtask::target_choices(ctx, sortie.side)[0].target)
        }).collect::<Vec<_>>()
    });
    for entry in entries {
        airtask::add_entry(&mut h.app.air_plan, entry);
    }
    h.app.air_plan.include_in_base_map = false;
    h.settle();
    let out = h.out("base_map_ui.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    crate::baseline_tests::assert_fixture_matches(&out);
}

/// The Map fingerprints cover the air plan: a later edit drops the dock's
/// undo instead of letting Ctrl Z throw it away, and marks Map unsaved.
#[test]
fn map_undo_is_dropped_by_a_later_air_edit() {
    let mut h = Harness::new("air_dock_settle");
    open_map_air_dock(&mut h);
    h.app.mark_saved(AppMode::Map);
    h.click("Auto-fill from objectives");
    assert!(h.app.map_undo.peek().is_some());
    assert!(h.app.is_dirty(AppMode::Map), "an air edit is an unsaved Map change");
    h.app.air_plan.pool[0].weight = 9;
    h.settle();
    assert!(h.app.map_undo.peek().is_none(), "the weight edit drops the Auto-fill undo");
}

const AIR_LOAD_WARNING: &str = "The loaded map had Air Tasking. It is not loaded back: Generate writes only the current plan, and its pool is now empty.";

#[test]
fn load_base_map_with_air_tasking_is_not_an_army() {
    let mut h = Harness::new("air_map_load_found");
    h.tab("Map");
    fill_air_pool(&mut h);
    h.app.terrain_apply = false;
    let out = h.out("base_map_air.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let entries = h.app.air_plan.pool.len();
    dialog::answer(vec![out]);
    h.app.load_base_map();
    assert!(h.app.map_armies.is_empty(), "the Air Tasking group is not loaded as an army");
    assert!(h.app.air_plan.pool.is_empty());
    match &h.app.status {
        Status::Warn { lead, items } => {
            assert!(lead.contains(&format!("Cleared {entries} Air Tasking entries")), "{lead}");
            assert_eq!(items, &[AIR_LOAD_WARNING.to_string()]);
        }
        _ => panic!("expected the orange Air Tasking warning: {}", h.status()),
    }
}

#[test]
fn load_base_map_keeps_air_library_and_settings() {
    let mut h = Harness::new("air_map_load_keeps");
    h.tab("Map");
    baseline_map_inputs(&mut h);
    let out = h.out("base_map_plain.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    fill_air_pool(&mut h);
    h.app.air_plan.budget.per_side = [2, 4];
    h.app.air_plan.categories.push("Night".into());
    let mut expected = h.app.air_plan.clone();
    expected.pool.clear();
    dialog::answer(vec![out]);
    h.app.load_base_map();
    assert_eq!(h.app.air_plan, expected, "library, categories, settings and seed stay");
    assert!(matches!(h.app.status, Status::Info(_)), "no Air Tasking in the file: {}", h.status());
    assert!(h.status().contains("Cleared 6 Air Tasking entries"), "{}", h.status());
}

/// Every Air dock control is a 28 px target inside the 304 px Map dock,
/// with both sides' lists filled; disabled controls count too.
#[test]
fn map_air_dock_widgets_are_28_px_with_entries() {
    let mut h = Harness::new("air_dock_sizes");
    open_map_air_dock(&mut h);
    h.click("Auto-fill from objectives");
    click_side(&mut h, "NATO");
    h.click("Auto-fill from objectives");
    let dock = (SCREEN.x - 304.0, SCREEN.x);
    let mut small = Vec::new();
    let mut outside = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for side in ["NATO", "DPRK"] {
        air_scroll(&mut h, SCREEN.x - 150.0, 10000.0);
        click_side(&mut h, side);
        for _ in 0..8 {
            for n in &h.nodes {
                let control = matches!(n.role, Role::Button | Role::CheckBox | Role::ComboBox | Role::Slider | Role::SpinButton);
                if !control || n.rect.width() <= 0.0 || n.rect.left() < dock.0 - 0.5 || n.rect.top() < shell::HEADER_H {
                    continue;
                }
                seen.insert(n.label.clone());
                if n.rect.height() < 27.5 {
                    small.push(format!("{side} {:?} {:?}: {} px", n.label, n.role, n.rect.height()));
                }
                if n.rect.right() > dock.1 + 0.5 {
                    outside.push(format!("{side} {:?} ends at {}", n.label, n.rect.right()));
                }
            }
            air_scroll(&mut h, SCREEN.x - 150.0, -240.0);
        }
    }
    for label in ["DPRK", "NATO", "Auto-fill from objectives", "Entry 1 weight", "Entry 1 mode", "Remove entry 1",
        "Entry 14 weight", "Entry 14 mode", "Remove entry 14", "Approach lines", "Threshold rings", "Count circles", "Spawn points", "RTB points"] {
        assert!(seen.contains(label), "not checked: {label}; {seen:?}");
    }
    small.sort();
    small.dedup();
    outside.sort();
    outside.dedup();
    assert!(small.is_empty(), "Air dock targets under 28 px: {small:?}");
    assert!(outside.is_empty(), "Air dock controls overflow the dock: {outside:?}");
}

/// The build re-reads user templates from disk, so the cached checks follow
/// the file: a template deleted after it was added is an error at once.
#[test]
fn air_checks_follow_a_user_template_file() {
    use crate::airtask::{self, IssueSeverity, PoolEntry};
    let mut h = Harness::new("air_cache_file");
    h.tab("Map");
    air_targets(&mut h);
    let path = h.out("Strike.Group");
    std::fs::copy(repo("TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group"), &path).unwrap();
    let sortie = airtask::load_user_sortie(&path).unwrap();
    let target = h.app.with_air_context(|ctx| airtask::target_choices(ctx, sortie.side)[0].target);
    let entry = PoolEntry::new(sortie.id.clone(), target);
    airtask::add_library_item(&mut h.app.air_plan, sortie);
    airtask::add_entry(&mut h.app.air_plan, entry);
    h.settle();
    let errors = |h: &Harness| h.app.air_eval_now().issues.iter().filter(|i| i.severity == IssueSeverity::Error).count();
    assert_eq!(errors(&h), 0);
    std::fs::remove_file(&path).unwrap();
    h.settle();
    assert!(errors(&h) > 0, "a missing template file blocks Generate");
}

/// Map counts as unsaved after an Air Tasking edit only when the plan goes
/// into the base map; an unticked plan does not change the file.
#[test]
fn map_unsaved_follows_the_air_plan_only_when_included() {
    let mut h = Harness::new("air_map_dirty");
    h.tab("Map");
    h.app.air_plan.include_in_base_map = false;
    h.app.mark_saved(AppMode::Map);
    h.app.air_plan.budget.per_side[0] = 2;
    assert!(!h.app.is_dirty(AppMode::Map), "an unticked plan is not in the base map");
    h.app.air_plan.include_in_base_map = true;
    assert!(h.app.is_dirty(AppMode::Map));
    h.app.mark_saved(AppMode::Map);
    h.app.air_plan.budget.per_side[0] = 3;
    assert!(h.app.is_dirty(AppMode::Map));
}

/// Load cannot be undone: Ctrl Z after Load brings back neither the pool
/// nor the Map state from before it, on the Map tab or the Air Tasking tab.
#[test]
fn ctrl_z_after_load_does_not_restore_the_pre_load_state() {
    let mut h = Harness::new("air_load_undo");
    h.tab("Map");
    air_targets(&mut h);
    h.app.air_plan.pool.clear();
    h.settle();
    let out = h.out("base_map_no_air.Group");
    dialog::answer(vec![out.clone()]);
    h.app.generate_front_file();
    // An Air Tasking tab edit and a Map dock edit, each with its own undo.
    h.tab("Air Tasking");
    h.click("Auto-fill from objectives");
    assert_eq!(h.app.air_plan.pool.len(), 14);
    h.tab("Map");
    h.click("Air");
    h.click_at(h.find_prefix("Remove entry ").rect.center());
    assert_eq!(h.app.air_plan.pool.len(), 13);
    dialog::answer(vec![out]);
    h.app.load_base_map();
    assert!(h.app.map_undo.peek().is_none() && h.app.air_undo.peek().is_none(), "Load leaves no undo to a state before it");
    h.settle();
    assert!(h.app.air_plan.pool.is_empty() && h.app.east_objectives.is_empty());
    h.key_with(Key::Z, CTRL);
    assert!(h.app.air_plan.pool.is_empty(), "Map's Ctrl Z after Load brought back the pool");
    assert!(h.app.east_objectives.is_empty(), "Map's Ctrl Z after Load brought back the objectives");
    h.tab("Air Tasking");
    h.key_with(Key::Z, CTRL);
    assert!(h.app.air_plan.pool.is_empty(), "Air Tasking's Ctrl Z after Load brought back the pool");
}

// ---- P14 acceptance flight A2 (run sheet: handoff/sessions/2026-10-28-p14-acceptance-a2.md) ----

/// A DPRK player flies east on autopilot through three NATO areas 15 km
/// apart, so each goes HOT in turn and the third asks while two NATO copies
/// fly (the first budget refusal). Built by Map Generate (`generate_front_file`).
const A2_STEM: &str = "P14_Acceptance_A2";
/// A, B and C: NATO objectives on one east-west line (x = 190 km), 17 to
/// 27 km north of the 1951-04-22 front (the Map's default date).
const A2_AREAS: [(f64, f64); 3] = [
    (190_000.0, 280_000.0),
    (190_000.0, 295_000.0),
    (190_000.0, 310_000.0),
];
/// The `--spawns` air start: 22 km west of A on the line, heading east
/// (YOri 90 is +Z), outside every area's 16 km R.
const A2_SPAWN: (f64, f64) = (190_000.0, 258_000.0);
const A2_HEADING: f64 = 90.0;
const A2_ALTITUDE: f64 = 2_000.0;
/// The run sheet's Air Tasking settings: R 16 km, zone chance 100 %,
/// cooldown 5 min (the defaults are 12 km, 50 % and 15 min).
const A2_RADIUS_M: f64 = 16_000.0;
const A2_CHANCE: u8 = 100;
const A2_COOLDOWN_MIN: f64 = 5.0;
const A2_SORTIES: [&str; 3] = [
    "1950_US_F80_HVAR_Strike_4ship.Group",
    "1950_US_F51_CloseSupport_4ship.Group",
    "1950_US_F80_HVAR_Strike_4ship.Group",
];

/// The A2 spawn list for `build_probe_mission.py --spawns`. Country 501, the
/// DPRK player country the NATO areas watch: the Map's own DPRK air start
/// wrote 502 when A2 was built (decision 0042), so A2 could not use it.
fn a2_spawns_json() -> String {
    let (x, z) = A2_SPAWN;
    format!(
        "[{{\"name\":\"P14 A2 DPRK air start\",\"country\":501,\"x\":{x},\"z\":{z},\
         \"heading_deg\":{A2_HEADING},\"altitude_m\":{A2_ALTITUDE},\"script\":\"yak9p\"}}]\n"
    )
}

/// The A2 base map. Map tab at its default date. The Map's own NATO fighter
/// pack is placed by a click (Place NATO). The inputs a user enters on the
/// map or in the Air Tasking panels are set on the app directly, as the other
/// Map tests do: the three NATO objectives, one Zone entry per area, and the
/// settings above. Then Trace build on and Generate Base Map through
/// `generate_front_file`, the Generate button's own call. Returns the written
/// root and every (block, Country) outside Air Tasking (`map_countries`).
fn build_a2(h: &mut Harness) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef, TriggerMode};
    h.tab("Map");
    h.wait_for_map();
    h.app.terrain_use_builtin = true;
    h.app.east_objectives.clear();
    h.app.nato_objectives = A2_AREAS.to_vec();
    h.app.attack_arrows.clear();
    h.app.air_plan.seed = 7;
    h.settle();
    place_nato_fighters(h);
    h.app.air_plan.pool.clear();
    for (file, xz) in A2_SORTIES.into_iter().zip(A2_AREAS) {
        let mut entry = PoolEntry::new(
            format!("builtin:{file}"),
            TargetRef::Objective {
                side: AirSide::Nato,
                xz,
            },
        );
        entry.mode = TriggerMode::Zone;
        airtask::add_entry(&mut h.app.air_plan, entry);
    }
    assert_eq!(h.app.air_plan.pool.len(), 3, "one entry per area");
    h.app.air_plan.threshold.radius_m = A2_RADIUS_M;
    h.app.air_plan.zone.chance_pct = A2_CHANCE;
    h.app.air_plan.zone.cooldown_min = A2_COOLDOWN_MIN;
    assert_eq!(h.app.air_plan.budget.per_side, [2, 2], "the default budget");
    assert!(h.app.air_plan.include_in_base_map);
    h.app.trace_build = true;
    h.settle();
    assert!(
        map_air_check(h).is_some_and(|c| c.ok),
        "the Air Tasking check fails"
    );
    assert!(!h.find("Generate Base Map").disabled);
    generate_map(h, &format!("{A2_STEM}.Group"))
}

/// Copies every `{stem}.*` file the build wrote into `target/p14` and prints
/// its size. Returns `target/p14`.
fn copy_flight_files(h: &Harness, stem: &str) -> PathBuf {
    let dest = repo("target/p14");
    std::fs::create_dir_all(&dest).unwrap();
    for file in std::fs::read_dir(&h.dir).unwrap() {
        let path = file.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with(&format!("{stem}.")) {
            std::fs::copy(&path, dest.join(&name)).unwrap();
            println!("{name}: {} bytes", std::fs::metadata(&path).unwrap().len());
        }
    }
    dest
}

#[test]
#[ignore = "writes target/p14/P14_Acceptance_A2.Group, locale, trace and spawn sidecars"]
fn write_p14_acceptance_a2() {
    let mut h = Harness::new("p14_acceptance_a2_write");
    build_a2(&mut h);
    let dest = copy_flight_files(&h, A2_STEM);
    std::fs::write(
        dest.join(format!("{A2_STEM}.spawns.json")),
        a2_spawns_json(),
    )
    .unwrap();
    println!("{}", h.status());
}

/// The A2 build, as the run sheet describes it. A2 was flown on 2026-10-29, so
/// its frozen files stay as flown and are checked by parse and links only
/// (`frozen_a2_files_parse_and_links_resolve`); the trace build has changed
/// since (switch breadcrumbs), so this checks the current build's
/// facts, not its bytes.
#[test]
fn a2_map_build_watches_three_nato_areas_for_dprk_players() {
    let mut h = Harness::new("p14_acceptance_a2");
    let (root, countries) = build_a2(&mut h);
    let frozen = repo("handoff/p14-probes/acceptance-a2");
    let spawns = std::fs::read_to_string(frozen.join(format!("{A2_STEM}.spawns.json"))).unwrap();
    assert_eq!(spawns, a2_spawns_json());

    let air = root.find_by_name("Air Tasking").expect("Air Tasking group");
    let trees: Vec<_> = air.children.iter().filter_map(|c| c.name()).collect();
    assert_eq!(
        trees,
        ["Air Tasking NATO Logic", "Air Tasking NATO Sorties"],
        "NATO side only"
    );
    let named = |name: &str| air.find_by_name(name).unwrap_or_else(|| panic!("{name}"));
    let mut clock = 0;
    air.for_each(&mut |node| {
        clock += usize::from(node.name().is_some_and(|n| n.starts_with("CLK ")))
    });
    assert_eq!(clock, 0, "Zone-only entries: no clock");

    // Three areas on the line, 16 km R: each watches DPRK players (501) at
    // 8 km. The spawn is on the same line, west of every area (heading 090
    // reaches them) and outside every area's R.
    for (i, &xz) in A2_AREAS.iter().enumerate() {
        let n = i + 1;
        let trigger = named(&format!("THN0{n} TRIGGER"));
        assert_eq!(trigger.pos_xz(), Some(xz));
        assert_eq!(
            (trigger.property("Radius"), trigger.property("Country")),
            (Some("8000"), Some("501"))
        );
        assert_eq!(xz.0, A2_SPAWN.0, "THN0{n}: on the spawn's line");
        assert!(
            xz.1 - A2_SPAWN.1 > A2_RADIUS_M,
            "THN0{n}: east of the spawn, beyond R"
        );
        let inner = named(&format!("THN0{n} INNER"));
        assert_eq!(
            (inner.pos_xz(), inner.property("Zone")),
            (Some(xz), Some("8000"))
        );
        assert_eq!(
            named(&format!("THN0{n} DWELL")).property("Time"),
            Some("80")
        );
        assert_eq!(
            named(&format!("THN0{n} HOLD")).property("Time"),
            Some("160")
        );
        assert_eq!(
            named(&format!("ZN0{n} CHANCE")).property("Random"),
            Some("100")
        );
        assert_eq!(
            named(&format!("ZN0{n} COOLDOWN")).property("Time"),
            Some("300")
        );
    }
    assert!(
        air.find_by_name("THN04 TRIGGER").is_none() && air.find_by_name("THD01 TRIGGER").is_none()
    );
    assert_eq!(
        named("BUD N COUNT").property("Counter"),
        Some("2"),
        "NATO budget 2"
    );

    // Nine copies, three per entry, all AI on 602. Decision 0050 D: no zones of
    // their own; each area's player count fires its copies' EMPTY IN. Decision
    // 0038: each DONE HUB OFF quiets more than the hub.
    let sorties = named("Air Tasking NATO Sorties");
    assert_eq!(sorties.children.len(), 9);
    for (j, copy) in sorties.children.iter().enumerate() {
        let name = copy.name().unwrap();
        let script = if j / 3 == 1 { "f51d" } else { "f80c10" };
        let mut planes = 0;
        copy.for_each(&mut |node| {
            if node.block_type == "Plane" {
                planes += 1;
                assert!(node.property("Script").unwrap().contains(script), "{name}");
                assert_eq!(node.property("Country"), Some("602"), "{name}");
            }
            assert_ne!(node.block_type, "MCU_CheckZone", "{name}: no zones of its own (0050 D)");
        });
        assert_eq!(planes, 4, "{name}");
        let empty_in = copy.find_by_name(&format!("SN0{} EMPTY IN", j + 1)).unwrap().index.unwrap();
        let count = named(&format!("PCN0{} EMPTY", j / 3 + 1));
        assert!(count.targets.contains(&empty_in), "{name}");
        assert_eq!(named(&format!("PCN0{} TRIGGER", j / 3 + 1)).pos_xz(), Some(A2_AREAS[j / 3]), "{name}");
        let off = copy
            .find_by_name(&format!("SN0{} DONE HUB OFF", j + 1))
            .unwrap();
        assert!(
            off.targets.len() > 1,
            "{name}: DONE HUB OFF closes its timers and zones too"
        );
    }

    // Decision 0037: outside Air Tasking the Map's planes are on 602 only,
    // and its NATO fighter pack launches only for planes far from the line.
    let planes: Vec<_> = countries
        .iter()
        .filter(|(block, _)| block == "Plane")
        .collect();
    assert!(
        !planes.is_empty() && planes.iter().all(|(_, c)| c == "602"),
        "{planes:?}"
    );
    // The line runs along x = 190 km, from the spawn to 8 km past C.
    let line_x = A2_SPAWN.0;
    let line_z = (A2_SPAWN.1, A2_AREAS[2].1 + 8_000.0);
    let mut launch_zones = 0;
    for pack in std::iter::once(&root)
        .chain(&root.children)
        .flat_map(|c| c.children.iter())
        .filter(|g| g.name().is_some_and(|n| n.starts_with("NATO Fighters")))
    {
        pack.for_each(&mut |node| {
            if node.block_type == "MCU_CheckZone" && node.property("Closer") == Some("1") {
                let (x, z) = node.pos_xz().unwrap();
                let radius: f64 = node.property("Zone").unwrap().parse().unwrap();
                let gap = (x - line_x).hypot(z - z.clamp(line_z.0, line_z.1)) - radius;
                assert!(
                    gap > 50_000.0,
                    "{:?} launches {gap:.0} m from the line",
                    pack.name()
                );
                launch_zones += 1;
            }
        });
    }
    assert!(
        launch_zones > 0,
        "the Map's own fighter pack and its launch zones are in the file"
    );
}

// ---- P14 acceptance flight A3 (run sheet: handoff/sessions/2026-10-30-p14-acceptance-a3.md) ----

/// A DPRK player spawns on the Map's own air start (decision 0042: country
/// 501, no `--spawns`), orbits one NATO area until a zone copy and a clock copy
/// are both over it, flies out beyond 35 km so both end in one tick, and comes
/// back for a later start. Built by Map Generate (`generate_front_file`).
const A3_STEM: &str = "P14_Acceptance_A3";
/// A: the one NATO objective (A2's area A).
const A3_AREA: (f64, f64) = (190_000.0, 280_000.0);
/// The Map air start's name, as the DPRK spawn list shows it.
const A3_AIR_START: &str = "P14 A3 DPRK air start";
/// The air start: 22.1 km from A, outside its 16 km R, heading east (YOri 90
/// is +Z) on a line 2.5 km south of A, so a left orbit begun abeam A circles A.
const A3_SPAWN: (f64, f64) = (187_500.0, 258_000.0);
const A3_HEADING: f64 = 90.0;
const A3_ALTITUDE_M: i32 = 2_000;
/// The run sheet's Air Tasking settings: R 16 km, zone chance 100 %,
/// cooldown 5 min; the NATO clock keeps its first slot (10 min) and period
/// (20 min) but has one jitter step and no quiet slots, so slot 1 opens at
/// 600.1 s for certain.
const A3_RADIUS_M: f64 = 16_000.0;
const A3_CHANCE: u8 = 100;
const A3_COOLDOWN_MIN: f64 = 5.0;
/// EN01 starts by zone and EN02 by clock: two entries, so the clock's slot
/// after the zone start is not quiet under the LAST rule.
const A3_ZONE_SORTIE: &str = "1950_US_F80_HVAR_Strike_4ship.Group";
const A3_CLOCK_SORTIE: &str = "1950_US_F51_CloseSupport_4ship.Group";

/// The A3 inputs. Map tab at its default date, no fighter pack. The inputs a
/// user enters on the map or in the panels are set on the app directly, as in
/// `build_a2`: the objective, the air start (a DPRK bank field changed to a
/// Yak-9P), the two entries and the settings above, then Trace build on. The
/// standalone check (`build_standalone_l`) starts from the same inputs.
fn a3_inputs(h: &mut Harness) {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef, TriggerMode};
    h.tab("Map");
    h.wait_for_map();
    h.app.terrain_use_builtin = true;
    h.app.east_objectives.clear();
    h.app.nato_objectives = vec![A3_AREA];
    h.app.attack_arrows.clear();
    h.app.air_plan.seed = 7;
    let mut field = crate::airstart::default_field(A3_AIR_START, false);
    field.heading_deg = A3_HEADING;
    let plane = &mut field.planes[0];
    crate::airstart::apply_type(plane, "yak9p");
    plane.altitude_m = A3_ALTITUDE_M;
    plane.fuel = 1.0;
    h.app.placed_air_starts.clear();
    h.app.placed_air_starts.push(crate::airstart::PlacedAirStart {
        field,
        x: A3_SPAWN.0,
        z: A3_SPAWN.1,
    });
    h.settle();
    h.app.air_plan.pool.clear();
    for (file, mode) in [
        (A3_ZONE_SORTIE, TriggerMode::Zone),
        (A3_CLOCK_SORTIE, TriggerMode::Clock),
    ] {
        let mut entry = PoolEntry::new(
            format!("builtin:{file}"),
            TargetRef::Objective {
                side: AirSide::Nato,
                xz: A3_AREA,
            },
        );
        entry.mode = mode;
        airtask::add_entry(&mut h.app.air_plan, entry);
    }
    assert_eq!(h.app.air_plan.pool.len(), 2, "EN01 Zone, EN02 Clock");
    h.app.air_plan.threshold.radius_m = A3_RADIUS_M;
    // A3's air start is 22 km from its area: the player count circle (decision
    // 0050 D) must stay inside that. The radius control comes in step 12e.
    h.app.air_plan.threshold.count_radius_m = 20_000.0;
    h.app.air_plan.zone.chance_pct = A3_CHANCE;
    h.app.air_plan.zone.cooldown_min = A3_COOLDOWN_MIN;
    let clock = &mut h.app.air_plan.schedule[AirSide::Nato.idx()];
    assert!(
        clock.enabled && clock.first_slot_min == 10.0 && clock.period_min == 20.0,
        "the default NATO clock"
    );
    clock.quiet_pct = 0;
    clock.jitter_steps = 1;
    assert_eq!(h.app.air_plan.budget.per_side, [2, 2], "the default budget");
    assert!(h.app.air_plan.include_in_base_map);
    h.app.trace_build = true;
    h.settle();
}

/// The A3 base map: `a3_inputs`, then Generate Base Map. Returns the written
/// root and every (block, Country) outside Air Tasking.
fn build_a3(h: &mut Harness) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    a3_inputs(h);
    assert!(
        map_air_check(h).is_some_and(|c| c.ok),
        "the Air Tasking check fails"
    );
    assert!(!h.find("Generate Base Map").disabled);
    generate_map(h, &format!("{A3_STEM}.Group"))
}

#[test]
#[ignore = "writes target/p14/P14_Acceptance_A3.Group, locale and trace sidecars"]
fn write_p14_acceptance_a3() {
    let mut h = Harness::new("p14_acceptance_a3_write");
    build_a3(&mut h);
    copy_flight_files(&h, A3_STEM);
    println!("{}", h.status());
}

/// The A3 build, as the run sheet describes it. A3 was flown on 2026-10-30, so
/// its frozen files stay as flown. Decision 0050 D changed the build (each copy
/// ends on its area's player count, not its own Zone Out), so the byte check
/// is dropped and the facts are kept, as A2's check did.
#[test]
fn a3_map_build_matches_the_run_sheet() {
    let mut h = Harness::new("p14_acceptance_a3");
    let (root, countries) = build_a3(&mut h);

    let air = root.find_by_name("Air Tasking").expect("Air Tasking group");
    let trees: Vec<_> = air.children.iter().filter_map(|c| c.name()).collect();
    assert_eq!(
        trees,
        ["Air Tasking NATO Logic", "Air Tasking NATO Sorties"],
        "NATO side only"
    );
    let named = |name: &str| air.find_by_name(name).unwrap_or_else(|| panic!("{name}"));

    // One area, 16 km R: it watches DPRK players (501) at 8 km.
    let trigger = named("THN01 TRIGGER");
    assert_eq!(trigger.pos_xz(), Some(A3_AREA));
    assert_eq!(
        (trigger.property("Radius"), trigger.property("Country")),
        (Some("8000"), Some("501"))
    );
    assert!(air.find_by_name("THN02 TRIGGER").is_none());
    // Decision 0050 D: the area's player count, 20 km (inside the air start's
    // 22 km), watching the same players.
    let count = named("PCN01 TRIGGER");
    assert_eq!(count.pos_xz(), Some(A3_AREA));
    assert_eq!((count.property("Radius"), count.property("Country")), (Some("20000"), Some("501")));
    for (name, key, value) in [
        ("THN01 INNER", "Zone", "8000"),
        ("THN01 DWELL", "Time", "80"),
        ("THN01 HOLD", "Time", "160"),
        ("ZN01 CHANCE", "Random", "100"),
        ("ZN01 COOLDOWN", "Time", "300"),
        ("BUD N COUNT", "Counter", "2"),
        // The NATO clock: slot 1 at 10 min, then every 20 min; one jitter
        // step at 0.1 s that always fires (no quiet slots).
        ("CLK N FIRST", "Time", "600"),
        ("CLK N PERIOD", "Time", "1200"),
        ("CLK N JITTER 1", "Time", "0.10"),
        ("CLK N JITTER 1", "Random", "100"),
    ] {
        assert_eq!(named(name).property(key), Some(value), "{name} {key}");
    }
    assert!(air.find_by_name("CLK N JITTER 2").is_none(), "one jitter step");
    // Only EN02 is drawn by the clock; EN01 starts by zone only.
    assert!(air.find_by_name("EN02 RANDOM").is_some());
    assert!(air.find_by_name("EN01 RANDOM").is_none());
    let mut dprk_clock = 0;
    air.for_each(&mut |node| {
        dprk_clock += usize::from(node.name().is_some_and(|n| n.starts_with("CLK D ")))
    });
    assert_eq!(dprk_clock, 0, "no DPRK clock");

    // Six copies, three per entry, all AI on 602: SN01-03 are EN01's F-80s
    // and SN04-06 are EN02's F-51s, all over A; (decision 0038) each DONE HUB
    // OFF quiets more than the hub.
    let sorties = named("Air Tasking NATO Sorties");
    assert_eq!(sorties.children.len(), 6);
    for (j, copy) in sorties.children.iter().enumerate() {
        let name = copy.name().unwrap();
        let script = if j < 3 { "f80c10" } else { "f51d" };
        let mut planes = 0;
        copy.for_each(&mut |node| {
            if node.block_type == "Plane" {
                planes += 1;
                assert!(node.property("Script").unwrap().contains(script), "{name}");
                assert_eq!(node.property("Country"), Some("602"), "{name}");
            }
            if node.block_type == "MCU_CheckZone" {
                assert_eq!(node.pos_xz(), Some(A3_AREA), "{name}");
            }
        });
        assert_eq!(planes, 4, "{name}");
        // 0050 D: no zones of its own; the area's count fires its EMPTY IN.
        assert_eq!(copy.count_block_type("MCU_CheckZone"), 0, "{name}");
        let empty_in = copy.find_by_name(&format!("SN0{} EMPTY IN", j + 1)).unwrap().index.unwrap();
        assert!(named("PCN01 EMPTY").targets.contains(&empty_in), "{name}");
        let off = copy
            .find_by_name(&format!("SN0{} DONE HUB OFF", j + 1))
            .unwrap();
        assert!(
            off.targets.contains(&empty_in) && off.targets.len() > 1,
            "{name}: DONE HUB OFF closes its timers and EMPTY IN too"
        );
    }

    // The Map's own air start (decision 0042): an air-start Yak-9P on 501, the
    // country the area watches, heading east, outside R.
    let field = root.find_by_name(A3_AIR_START).expect("the air start group");
    let airfield = field
        .children
        .iter()
        .find(|c| c.block_type == "Airfield")
        .expect("its Airfield");
    assert_eq!(airfield.property("Country"), trigger.property("Country"));
    assert_eq!(airfield.pos_xz(), Some(A3_SPAWN));
    assert_eq!(airfield.property("YOri"), Some("90.000"));
    // R is twice the entry trigger's radius.
    let radius: f64 = trigger.property("Radius").unwrap().parse().unwrap();
    let ((ax, az), (tx, tz)) = (airfield.pos_xz().unwrap(), trigger.pos_xz().unwrap());
    assert!((tx - ax).hypot(tz - az) > 2.0 * radius, "the air start is inside R");
    let mut planes = Vec::new();
    airfield.for_each(&mut |node| {
        if node.block_type == "Plane" {
            planes.push(
                ["Script", "StartType", "Altitude", "Fuel"].map(|key| node.property(key).unwrap_or("").to_string()),
            );
        }
    });
    let script = format!("\"{}\"", crate::airstart::plane_paths("yak9p").1);
    assert_eq!(planes, [[script, "0".into(), "2000".into(), "1".into()]]);

    // Outside Air Tasking no Plane carries a country (no fighter pack), and the
    // air start is the only Airfield.
    let blocks: Vec<_> = countries
        .iter()
        .filter(|(block, _)| block == "Plane" || block == "Airfield")
        .collect();
    assert_eq!(blocks, [&("Airfield".to_string(), "501".to_string())]);
}

// ---- P14 flight 2N (run sheet: handoff/sessions/2026-11-02-p14-flight-2n.md) ----

/// Up to four human pilots on eszyman's server (decision 0059): a DPRK pair
/// over NATO area A and a NATO pair over DPRK area B, 85 km apart, flying the
/// same cues. Pilot 1 orbits alone (N = 2: nothing starts), pilot 2 joins (HOT,
/// a sortie starts), then they leave the count circle one at a time and the
/// copy ends on the second. Built by Map Generate twice from the same inputs: a
/// trace build to fly and a clean build to share.
const N2_STEM: &str = "P14_Flight_2N";
const N2_CLEAN_STEM: &str = "P14_Flight_2N_Clean";
/// A: the NATO area, A3's (27 km north of the front). It watches DPRK players.
const N2_AREA_A: (f64, f64) = (190_000.0, 280_000.0);
/// B: the DPRK area, about 32 km south of the front. It watches NATO players.
const N2_AREA_B: (f64, f64) = (110_000.0, 250_000.0);
/// Each side's air start: 2.5 km south of its area and 22 km west of it,
/// heading east (YOri 90 is +Z), so the area passes 2.5 km to the left, as in
/// A3. 22.1 km out: outside the 20 km count circle (decision 0051).
const N2_DPRK_AIR_START: &str = "P14 2N DPRK air start";
const N2_NATO_AIR_START: &str = "P14 2N NATO air start";
const N2_DPRK_SPAWN: (f64, f64) = (187_500.0, 258_000.0);
const N2_NATO_SPAWN: (f64, f64) = (107_500.0, 228_000.0);
const N2_HEADING: f64 = 90.0;
/// The DPRK pair flies at 2,000 m, as in A3 (the F-80 copies fly at 3,050 m).
/// The NATO pair flies at 3,000 m, well above the Il-10 copies' 1,500 m and
/// their rear gunners.
const N2_DPRK_ALTITUDE_M: i32 = 2_000;
const N2_NATO_ALTITUDE_M: i32 = 3_000;
/// The run sheet's Air Tasking settings.
const N2_PLAYERS: u8 = 2;
const N2_RADIUS_M: f64 = 16_000.0;
const N2_COUNT_RADIUS_M: f64 = 20_000.0;
const N2_CHANCE: u8 = 100;
const N2_COOLDOWN_MIN: f64 = 5.0;
/// One Zone entry per area. Neither carries air-to-air orders, so the AI
/// leaves the players alone.
const N2_NATO_SORTIE: &str = "1950_US_F80_HVAR_Strike_4ship.Group";
const N2_DPRK_SORTIE: &str = "1950_DPRK_Il10_KimpoAttack_2x4.Group";

fn n2_air_start(
    name: &str,
    nato: bool,
    type_id: &str,
    xz: (f64, f64),
    altitude_m: i32,
) -> crate::airstart::PlacedAirStart {
    let mut field = crate::airstart::default_field(name, nato);
    field.heading_deg = N2_HEADING;
    let plane = &mut field.planes[0];
    crate::airstart::apply_type(plane, type_id);
    plane.altitude_m = altitude_m;
    plane.fuel = 1.0;
    crate::airstart::PlacedAirStart {
        field,
        x: xz.0,
        z: xz.1,
    }
}

/// The 2N inputs, set on the app directly as in `a3_inputs`: Map tab at its
/// default date, no fighter pack, one area per side, one air start per side
/// (a Yak-9P on 501, an F-51D on 601), one Zone entry per area, the settings
/// above. No entry is on the clock, so neither side builds one.
fn n2_inputs(h: &mut Harness, trace: bool) {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef, TriggerMode};
    h.tab("Map");
    h.wait_for_map();
    h.app.terrain_use_builtin = true;
    h.app.nato_objectives = vec![N2_AREA_A];
    h.app.east_objectives = vec![N2_AREA_B];
    h.app.attack_arrows.clear();
    h.app.air_plan.seed = 7;
    h.app.placed_air_starts = vec![
        n2_air_start(N2_DPRK_AIR_START, false, "yak9p", N2_DPRK_SPAWN, N2_DPRK_ALTITUDE_M),
        n2_air_start(N2_NATO_AIR_START, true, "f51d", N2_NATO_SPAWN, N2_NATO_ALTITUDE_M),
    ];
    h.settle();
    h.app.air_plan.pool.clear();
    for (file, side, xz) in [
        (N2_NATO_SORTIE, AirSide::Nato, N2_AREA_A),
        (N2_DPRK_SORTIE, AirSide::Dprk, N2_AREA_B),
    ] {
        let mut entry =
            PoolEntry::new(format!("builtin:{file}"), TargetRef::Objective { side, xz });
        entry.mode = TriggerMode::Zone;
        airtask::add_entry(&mut h.app.air_plan, entry);
    }
    assert_eq!(h.app.air_plan.pool.len(), 2, "one Zone entry per area");
    let threshold = &mut h.app.air_plan.threshold;
    threshold.players_n = N2_PLAYERS;
    threshold.radius_m = N2_RADIUS_M;
    threshold.count_radius_m = N2_COUNT_RADIUS_M;
    h.app.air_plan.zone.chance_pct = N2_CHANCE;
    h.app.air_plan.zone.cooldown_min = N2_COOLDOWN_MIN;
    assert_eq!(h.app.air_plan.budget.per_side, [2, 2], "the default budget");
    assert!(h.app.air_plan.include_in_base_map);
    h.app.trace_build = trace;
    h.settle();
}

/// A 2N base map: `n2_inputs`, then Generate Base Map.
fn build_n2(h: &mut Harness, trace: bool) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    n2_inputs(h, trace);
    assert!(
        map_air_check(h).is_some_and(|c| c.ok),
        "the Air Tasking check fails"
    );
    assert!(!h.find("Generate Base Map").disabled);
    let stem = if trace { N2_STEM } else { N2_CLEAN_STEM };
    generate_map(h, &format!("{stem}.Group"))
}

#[test]
#[ignore = "writes target/p14/P14_Flight_2N(_Clean).Group, locale and trace sidecars"]
fn write_p14_flight_2n() {
    for (trace, stem) in [(true, N2_STEM), (false, N2_CLEAN_STEM)] {
        let mut h = Harness::new(&format!("p14_flight_2n_write_{trace}"));
        build_n2(&mut h, trace);
        copy_flight_files(&h, stem);
        println!("{}", h.status());
    }
}

/// The facts the 2N run sheet relies on, for one build.
fn check_n2_build(root: &crate::ast::Il2Entity, countries: &[(String, String)], trace: bool) {
    let air = root.find_by_name("Air Tasking").expect("Air Tasking group");
    let named = |name: &str| air.find_by_name(name).unwrap_or_else(|| panic!("{name}"));
    let mut trees: Vec<_> = air.children.iter().filter_map(|c| c.name()).collect();
    trees.sort_unstable();
    assert_eq!(
        trees,
        [
            "Air Tasking DPRK Logic",
            "Air Tasking DPRK Sorties",
            "Air Tasking NATO Logic",
            "Air Tasking NATO Sorties"
        ],
        "both sides"
    );
    // Each area watches the other side's players: an 8 km entry trigger that
    // counts to N = 2, and the 20 km player count.
    for (side, area, country) in [("N", N2_AREA_A, "501"), ("D", N2_AREA_B, "601")] {
        let trigger = named(&format!("TH{side}01 TRIGGER"));
        assert_eq!(trigger.pos_xz(), Some(area));
        assert_eq!(
            (trigger.property("Radius"), trigger.property("Country")),
            (Some("8000"), Some(country))
        );
        assert_eq!(
            named(&format!("TH{side}01 COUNT")).property("Counter"),
            Some("2"),
            "N = 2"
        );
        assert!(
            air.find_by_name(&format!("TH{side}02 TRIGGER")).is_none(),
            "one area per side"
        );
        let count = named(&format!("PC{side}01 TRIGGER"));
        assert_eq!(count.pos_xz(), Some(area));
        assert_eq!(
            (count.property("Radius"), count.property("Country")),
            (Some("20000"), Some(country))
        );
        assert_eq!(
            named(&format!("Z{side}01 CHANCE")).property("Random"),
            Some("100")
        );
        assert_eq!(
            named(&format!("Z{side}01 COOLDOWN")).property("Time"),
            Some("300")
        );
        let mut clock = 0;
        air.for_each(&mut |node| {
            clock += usize::from(node.name().is_some_and(|n| n.starts_with("CLK ")))
        });
        assert_eq!(clock, 0, "no clock");
    }
    // Each side's copies are the one entry's sortie, flown by its AI country,
    // and each ends on its own area's count (0050 D).
    for (side, script, ai_country) in [("NATO", "f80c10", "602"), ("DPRK", "il10", "503")] {
        let sorties = named(&format!("Air Tasking {side} Sorties"));
        assert!(!sorties.children.is_empty(), "{side} copies");
        let tag = &side[..1];
        for copy in &sorties.children {
            let name = copy.name().unwrap();
            copy.for_each(&mut |node| {
                if node.block_type == "Plane" {
                    assert!(
                        node.property("Script")
                            .unwrap()
                            .to_lowercase()
                            .contains(script),
                        "{name}"
                    );
                    assert_eq!(node.property("Country"), Some(ai_country), "{name}");
                }
            });
            let prefix = name.split(' ').next().unwrap();
            let empty_in = copy
                .find_by_name(&format!("{prefix} EMPTY IN"))
                .unwrap()
                .index
                .unwrap();
            assert!(
                named(&format!("PC{tag}01 EMPTY"))
                    .targets
                    .contains(&empty_in),
                "{name}"
            );
        }
    }
    // The two air starts: each on the country the other side's area watches,
    // outside its count circle.
    for (name, area, xz, country, type_id, altitude) in [
        (N2_DPRK_AIR_START, N2_AREA_A, N2_DPRK_SPAWN, "501", "yak9p", N2_DPRK_ALTITUDE_M),
        (N2_NATO_AIR_START, N2_AREA_B, N2_NATO_SPAWN, "601", "f51d", N2_NATO_ALTITUDE_M),
    ] {
        let field = root.find_by_name(name).unwrap_or_else(|| panic!("{name}"));
        let airfield = field
            .children
            .iter()
            .find(|c| c.block_type == "Airfield")
            .expect("its Airfield");
        assert_eq!(airfield.property("Country"), Some(country), "{name}");
        assert_eq!(airfield.pos_xz(), Some(xz), "{name}");
        assert_eq!(airfield.property("YOri"), Some("90.000"), "{name}");
        assert!(
            (xz.0 - area.0).hypot(xz.1 - area.1) > N2_COUNT_RADIUS_M,
            "{name} is inside the count circle"
        );
        let mut planes = Vec::new();
        airfield.for_each(&mut |node| {
            if node.block_type == "Plane" {
                planes.push(["Script", "StartType", "Altitude", "Fuel", "Number"].map(|key| node.property(key).unwrap_or("").to_string()));
            }
        });
        let script = format!("\"{}\"", crate::airstart::plane_paths(type_id).1);
        assert_eq!(
            planes,
            [[script, "0".into(), altitude.to_string(), "1".into(), "-1".into()]],
            "{name}: one air-start type, any number of players"
        );
    }
    let mut fields: Vec<_> = countries
        .iter()
        .filter(|(block, _)| block == "Plane" || block == "Airfield")
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            &("Airfield".to_string(), "501".to_string()),
            &("Airfield".to_string(), "601".to_string())
        ],
        "no AI aircraft outside Air Tasking"
    );
    // Breadcrumbs (mission objectives) only in the trace build.
    let mut crumbs = 0;
    root.for_each(&mut |node| crumbs += usize::from(node.block_type == "MCU_TR_MissionObjective"));
    if trace {
        assert!(crumbs > 0, "the trace build carries breadcrumbs");
    } else {
        assert_eq!(crumbs, 0, "the clean build carries no breadcrumbs");
    }
}

/// The 2N build, as the run sheet describes it: both builds have the same Air
/// Tasking, and only the trace build carries breadcrumbs.
#[test]
fn n2_map_builds_match_the_run_sheet() {
    let frozen = repo("handoff/p14-probes/flight-2n");
    let mut h = Harness::new("p14_flight_2n");
    let (root, countries) = build_n2(&mut h, true);
    check_n2_build(&root, &countries, true);
    let mut clean = Harness::new("p14_flight_2n_clean");
    let (clean_root, clean_countries) = build_n2(&mut clean, false);
    check_n2_build(&clean_root, &clean_countries, false);
    // Not flown yet: the build matches the frozen files byte for byte.
    for (dir, file) in [
        (&h.dir, format!("{N2_STEM}.Group")),
        (&h.dir, format!("{N2_STEM}.eng")),
        (&h.dir, format!("{N2_STEM}.trace.json")),
        (&clean.dir, format!("{N2_CLEAN_STEM}.Group")),
        (&clean.dir, format!("{N2_CLEAN_STEM}.eng")),
    ] {
        let built = std::fs::read(dir.join(&file)).unwrap();
        let kept = std::fs::read(frozen.join(&file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(built == kept, "{file} differs from the frozen copy");
    }
}
// ---- P14 step 11 item 4: the standalone file (risk l; run sheet: handoff/sessions/2026-10-30-p14-standalone-l.md) ----

/// The base map the editor imports the standalone file into.
const L_BASE_STEM: &str = "P14_Standalone_L_Base";
/// The Air Tasking tab's standalone file.
const L_AIR_STEM: &str = "P14_Standalone_L_Air";

/// Decision 0054 does not change L's base map: A3's inputs have no Map AI.
#[test]
fn map_generate_0054_standalone_l_base_bytes_unchanged() {
    let mut h = Harness::new("air_map_standalone_l_0054");
    a3_inputs(&mut h);
    h.app.air_plan.include_in_base_map = false;
    generate_map_from_button(&mut h, &format!("{L_BASE_STEM}.Group"));
    let frozen = repo("handoff/p14-probes/standalone-l");
    let mut compared = 0;
    for file in std::fs::read_dir(frozen).unwrap() {
        let file = file.unwrap();
        let name = file.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{L_BASE_STEM}.")) {
            let actual = std::fs::read(h.out(&name)).unwrap();
            let expected = std::fs::read(file.path()).unwrap();
            assert!(actual == expected, "{name}: L's base map bytes changed");
            compared += 1;
        }
    }
    assert_eq!(compared, 8, "Group, six locales and trace sidecar");
}

/// A3's inputs (`a3_inputs`) with "Include in base map" off. Map Generate
/// writes the base map, which then has no Air Tasking group; the Air Tasking
/// tab's Generate File (Ctrl G) writes the standalone file, whose indexes start
/// at 1 like the base map's. Returns both written roots.
fn build_standalone_l(h: &mut Harness) -> (crate::ast::Il2Entity, crate::ast::Il2Entity) {
    a3_inputs(h);
    h.app.air_plan.include_in_base_map = false;
    h.settle();
    assert!(map_air_check(h).is_none(), "an unticked plan does not touch the base map");
    assert!(!h.find("Generate Base Map").disabled);
    let (base, _) = generate_map(h, &format!("{L_BASE_STEM}.Group"));
    h.tab("Air Tasking");
    assert!(!h.find("Generate File").disabled);
    let path = h.out(&format!("{L_AIR_STEM}.Group"));
    dialog::answer(vec![path.clone()]);
    h.key_with(Key::G, CTRL);
    assert!(matches!(h.app.status, Status::Info(_)), "{}", h.status());
    let air = crate::parser::parse_group_file(&std::fs::read_to_string(path).unwrap()).unwrap();
    (base, air)
}

#[test]
#[ignore = "writes target/p14/P14_Standalone_L_Base.* and P14_Standalone_L_Air.*"]
fn write_p14_standalone_l() {
    let mut h = Harness::new("p14_standalone_l_write");
    build_standalone_l(&mut h);
    copy_flight_files(&h, L_BASE_STEM);
    copy_flight_files(&h, L_AIR_STEM);
    println!("{}", h.status());
}

/// The two files the editor check starts from, as the run sheet describes
/// them: the frozen copies' file set and every base file's bytes (check L was
/// flown on 2026-10-30; decision 0050 D only changed the Air output); the base
/// map has the air start and no Air Tasking; the standalone file is one `Air
/// Tasking` group indexed from 1, so its indexes collide with the base map's;
/// and, under that renumbering, it is A3's Air Tasking group as built now (same
/// inputs, same seed).
#[test]
fn standalone_l_build_matches_a3() {
    let mut h = Harness::new("p14_standalone_l");
    let (base, air) = build_standalone_l(&mut h);
    let frozen = repo("handoff/p14-probes/standalone-l");
    // The base map's and the standalone file's files in a folder, sorted.
    let stem_files = |dir: &Path| -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|f| f.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(&format!("{L_BASE_STEM}.")) || n.starts_with(&format!("{L_AIR_STEM}.")))
            .collect();
        names.sort();
        names
    };
    assert_eq!(stem_files(&h.dir), stem_files(&frozen), "the build writes the frozen file set");
    for name in stem_files(&frozen).into_iter().filter(|n| n.starts_with(&format!("{L_BASE_STEM}."))) {
        let expected = std::fs::read(frozen.join(&name)).unwrap();
        let actual = std::fs::read(h.dir.join(&name)).unwrap();
        let first = expected.iter().zip(&actual).position(|(a, b)| a != b);
        assert!(actual == expected, "{name}: first differing byte {:?}; expected {} bytes, got {}", first.or(Some(expected.len().min(actual.len()))), expected.len(), actual.len());
    }

    // The base map: the air start, no Air Tasking, indexes from 1.
    assert!(base.find_by_name("Air Tasking").is_none());
    assert!(base.find_by_name(A3_AIR_START).is_some());
    let mut base_indexes = Vec::new();
    base.collect_indexes(&mut base_indexes);
    // The standalone file: one Air Tasking group, indexes 1..=n.
    assert_eq!((air.name(), air.index), (Some("Air Tasking"), Some(1)));
    let mut air_indexes = Vec::new();
    air.collect_indexes(&mut air_indexes);
    air_indexes.sort_unstable();
    assert_eq!(air_indexes, (1..=air_indexes.len() as i32).collect::<Vec<_>>());
    crate::airtask::testkit::assert_links_resolve(&air);
    let clashes = air_indexes.iter().filter(|i| base_indexes.contains(i)).count();
    assert_eq!(clashes, air_indexes.len(), "every standalone index is also used by the base map");

    // A3's Air Tasking group as built now, renumbered. The Map writes the
    // trace breadcrumbs at the base map's root and the standalone file inside
    // its one group, so A3's are moved into its group, in file order, first.
    let mut a3_h = Harness::new("p14_standalone_l_a3");
    let (a3, _) = build_a3(&mut a3_h);
    let mut a3_air = a3.find_by_name("Air Tasking").unwrap().clone();
    let mut crumbs: Vec<_> = a3.children.iter().filter(|c| c.block_type == "MCU_TR_MissionObjective").cloned().collect();
    let a3_trace = crate::trace::read_trace_sidecar(&a3_h.out(&format!("{A3_STEM}.trace.json"))).unwrap();
    assert_eq!(crumbs.len(), a3_trace.entries.len(), "A3's breadcrumbs are its root objectives");
    // Standalone Air uses its own X band; only that fixed offset differs
    // from Map's unchanged breadcrumb grid. Everything else compares exactly.
    for crumb in &mut crumbs {
        let (x, _) = crumb.pos_xz().unwrap();
        crumb.set_property("XPos", format!("{:.3}", x + crate::trace::TraceBand::AirTasking.x_offset()));
    }
    a3_air.children.extend(crumbs);
    crate::airtask::testkit::assert_same_up_to_indexes(&a3_air, &air, crate::airtask::testkit::Renumbering::default());
}

// ---------------------------------------------------------------------------
// Flight A4 (solo, one NATO player): a Yak copy and a DPRK-side extension.
// ---------------------------------------------------------------------------

/// A4: one NATO player in an F-51D orbits a DPRK area while a Yak-9P raid
/// flies its on-station time, gets its one extension (the area is still HOT),
/// and is sent home. Built by Map Generate as a trace build.
const A4_STEM: &str = "P14_Acceptance_A4";
/// The DPRK area: 2N's area B, about 32 km south of the front. It watches
/// NATO players.
const A4_AREA: (f64, f64) = (110_000.0, 250_000.0);
/// 2N's NATO air start: 2.5 km south of the area and 22 km west of it, heading
/// east, outside the 20 km count circle (decision 0051). Load-tested in 2N.
const A4_AIR_START: &str = "P14 A4 NATO air start";
const A4_SPAWN: (f64, f64) = (107_500.0, 228_000.0);
const A4_HEADING: f64 = 90.0;
/// 4,500 m: about 1,450 m above the Yaks' 3,050 m, out of their way.
const A4_ALTITUDE_M: i32 = 4_500;
const A4_RADIUS_M: f64 = 16_000.0;
const A4_COUNT_RADIUS_M: f64 = 20_000.0;
const A4_CHANCE: u8 = 100;
const A4_COOLDOWN_MIN: f64 = 5.0;
const A4_SORTIE: &str = "1950_DPRK_Yak_AirfieldRaid_2plus2.Group";

/// The A4 inputs, set on the app as in `n2_inputs`: one DPRK area, one NATO
/// air start (an F-51D on 601), one Zone entry (the Yak raid), N = 1.
fn a4_inputs(h: &mut Harness) {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef, TriggerMode};
    h.tab("Map");
    h.wait_for_map();
    h.app.terrain_use_builtin = true;
    h.app.nato_objectives.clear();
    h.app.east_objectives = vec![A4_AREA];
    h.app.attack_arrows.clear();
    h.app.air_plan.seed = 7;
    let mut start = n2_air_start(A4_AIR_START, true, "f51d", A4_SPAWN, A4_ALTITUDE_M);
    start.field.heading_deg = A4_HEADING;
    h.app.placed_air_starts = vec![start];
    h.settle();
    h.app.air_plan.pool.clear();
    let mut entry = PoolEntry::new(
        format!("builtin:{A4_SORTIE}"),
        TargetRef::Objective {
            side: AirSide::Dprk,
            xz: A4_AREA,
        },
    );
    entry.mode = TriggerMode::Zone;
    airtask::add_entry(&mut h.app.air_plan, entry);
    assert_eq!(h.app.air_plan.pool.len(), 1, "one Zone entry");
    let threshold = &mut h.app.air_plan.threshold;
    assert_eq!(threshold.players_n, 1, "N = 1 by default");
    threshold.radius_m = A4_RADIUS_M;
    threshold.count_radius_m = A4_COUNT_RADIUS_M;
    h.app.air_plan.zone.chance_pct = A4_CHANCE;
    h.app.air_plan.zone.cooldown_min = A4_COOLDOWN_MIN;
    assert_eq!(h.app.air_plan.budget.per_side, [2, 2], "the default budget");
    assert!(h.app.air_plan.include_in_base_map);
    h.app.trace_build = true;
    h.settle();
}

fn build_a4(h: &mut Harness) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    a4_inputs(h);
    assert!(
        map_air_check(h).is_some_and(|c| c.ok),
        "the Air Tasking check fails"
    );
    assert!(!h.find("Generate Base Map").disabled);
    generate_map(h, &format!("{A4_STEM}.Group"))
}

#[test]
#[ignore = "writes target/p14/P14_Acceptance_A4.Group, locale and trace sidecars"]
fn write_p14_acceptance_a4() {
    let mut h = Harness::new("p14_acceptance_a4_write");
    build_a4(&mut h);
    copy_flight_files(&h, A4_STEM);
    println!("{}", h.status());
}

/// A timer's `Time` and `Random` in an Air Tasking build.
fn timer_time_random(air: &crate::ast::Il2Entity, name: &str) -> (f64, u32) {
    let node = air.find_by_name(name).unwrap_or_else(|| panic!("{name}"));
    let time = node
        .property("Time")
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(|| panic!("{name} Time"));
    let random = node.property("Random").and_then(|r| r.parse().ok()).unwrap_or(100);
    (time, random)
}

/// The facts the A4 run sheet relies on.
fn check_a4_build(root: &crate::ast::Il2Entity, countries: &[(String, String)]) {
    let air = root.find_by_name("Air Tasking").expect("Air Tasking group");
    let mut trees: Vec<_> = air.children.iter().filter_map(|c| c.name()).collect();
    trees.sort_unstable();
    assert_eq!(trees, ["Air Tasking DPRK Logic", "Air Tasking DPRK Sorties"], "DPRK only");
    assert!(air.find_by_name("Clock").is_none(), "no clock: the one entry is Zone");
    // The area watches NATO players: an 8 km entry trigger (R 16 km), N = 1.
    let trigger = air.find_by_name("THD01 TRIGGER").expect("THD01 TRIGGER");
    assert_eq!(trigger.pos_xz(), Some(A4_AREA));
    assert_eq!(
        (trigger.property("Radius"), trigger.property("Country")),
        (Some("8000"), Some("601"))
    );
    assert!(air.find_by_name("THD02 TRIGGER").is_none(), "one area");
    for (name, time, random) in [
        ("THD01 DWELL", 80.0, 100),
        ("THD01 HOLD", 160.0, 100),
        ("ZD01 CHANCE", 0.1, 100),
        ("ZD01 COOLDOWN", 300.0, 100),
        // The Yak raid's own 5 min over the target, then one 10 min extension.
        ("SD01 ON STATION", 300.0, 100),
        ("SD01 EXTEND", 600.0, 100),
        ("SD01 HARD STOP", 1716.0, 100),
        ("SD01 DELETE DELAY", 0.5, 100),
    ] {
        let (t, r) = timer_time_random(air, name);
        assert!((t - time).abs() < 1e-6 && r == random, "{name}: {t} s, {r} %");
    }
    // Three Yak copies: 2 + 2 Yak-9P at 3,050 m, 30 km from the area, on
    // 503, a country the area does not count (decision 0050 D).
    let sorties = air.find_by_name("Air Tasking DPRK Sorties").unwrap();
    let mut planes = Vec::new();
    sorties.for_each(&mut |node| {
        if node.block_type == "Plane" {
            planes.push((
                node.property("Country").map(str::to_string),
                node.property("Model").unwrap_or("").to_string(),
                node.pos_xz(),
            ));
        }
    });
    assert_eq!(planes.len(), 12, "three copies of four");
    for (country, model, xz) in &planes {
        assert_eq!(country.as_deref(), Some("503"));
        assert!(model.contains("yak9p"), "a Yak-9P: {model}");
        let (x, z) = xz.unwrap();
        let d = ((x - A4_AREA.0).powi(2) + (z - A4_AREA.1).powi(2)).sqrt();
        assert!((29_900.0..30_700.0).contains(&d), "{d:.0} m from the area");
    }
    // The one air start: an F-51D on 601, 22 km out.
    let mut fields = Vec::new();
    root.for_each(&mut |node| {
        if node.block_type == "Airfield" {
            fields.push((
                node.name().map(str::to_string),
                node.property("Country").map(str::to_string),
                node.pos_xz(),
            ));
        }
    });
    assert_eq!(
        fields,
        [(
            Some(A4_AIR_START.to_string()),
            Some("601".to_string()),
            Some(A4_SPAWN)
        )],
        "one air start"
    );
    let mut outside: Vec<_> = countries
        .iter()
        .filter(|(block, _)| block == "Plane" || block == "Airfield")
        .collect();
    outside.sort();
    assert_eq!(
        outside,
        [&("Airfield".to_string(), "601".to_string())],
        "no AI aircraft outside Air Tasking"
    );
}

/// Not flown yet: the build matches the frozen files byte for byte.
#[test]
fn a4_map_build_matches_the_run_sheet() {
    let frozen = repo("handoff/p14-probes/flight-a4");
    let mut h = Harness::new("p14_acceptance_a4");
    let (root, countries) = build_a4(&mut h);
    check_a4_build(&root, &countries);
    for file in [
        format!("{A4_STEM}.Group"),
        format!("{A4_STEM}.eng"),
        format!("{A4_STEM}.trace.json"),
    ] {
        let built = std::fs::read(h.dir.join(&file)).unwrap();
        let kept = std::fs::read(frozen.join(&file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(built == kept, "{file} differs from the frozen copy");
    }
}

// ---------------------------------------------------------------------------
// Flight 2T (no pilot): the quiet roll and the jitter steps, measured.
// ---------------------------------------------------------------------------

/// 2T: both clocks run fast (a slot a minute) for an hour with nobody flying,
/// so about 120 slots roll and the quiet rate can be counted from the log. No
/// area goes HOT, so nothing launches. Built by Map Generate as a trace build.
const T2_STEM: &str = "P14_Probe_2T";
/// One area per side, far from each other and from the front.
const T2_AREA_NATO: (f64, f64) = (190_000.0, 280_000.0);
const T2_AREA_DPRK: (f64, f64) = (110_000.0, 250_000.0);
/// The defaults except the times: first slot 1 min, period 1 min, jitter
/// steps of 15 s. Quiet 20 %, 3 steps, as shipped.
const T2_FIRST_MIN: f64 = 1.0;
const T2_PERIOD_MIN: f64 = 1.0;
const T2_JITTER_STEP_MIN: f64 = 0.25;
const T2_NATO_SORTIE: &str = "1950_US_F80_HVAR_Strike_4ship.Group";
const T2_DPRK_SORTIE: &str = "1950_DPRK_Il10_KimpoAttack_2x4.Group";

fn t2_inputs(h: &mut Harness) {
    use crate::airtask::{self, AirSide, PoolEntry, TargetRef, TriggerMode};
    h.tab("Map");
    h.wait_for_map();
    h.app.terrain_use_builtin = true;
    h.app.nato_objectives = vec![T2_AREA_NATO];
    h.app.east_objectives = vec![T2_AREA_DPRK];
    h.app.attack_arrows.clear();
    h.app.air_plan.seed = 7;
    h.app.placed_air_starts.clear();
    h.settle();
    h.app.air_plan.pool.clear();
    for (file, side, xz) in [
        (T2_NATO_SORTIE, AirSide::Nato, T2_AREA_NATO),
        (T2_DPRK_SORTIE, AirSide::Dprk, T2_AREA_DPRK),
    ] {
        let mut entry =
            PoolEntry::new(format!("builtin:{file}"), TargetRef::Objective { side, xz });
        entry.mode = TriggerMode::Clock;
        airtask::add_entry(&mut h.app.air_plan, entry);
    }
    assert_eq!(h.app.air_plan.pool.len(), 2, "one Clock entry per side");
    for side in [AirSide::Dprk, AirSide::Nato] {
        let clock = &mut h.app.air_plan.schedule[side.idx()];
        assert!(clock.enabled && clock.quiet_pct == 20 && clock.jitter_steps == 3);
        clock.first_slot_min = T2_FIRST_MIN;
        clock.period_min = T2_PERIOD_MIN;
        clock.jitter_step_min = T2_JITTER_STEP_MIN;
    }
    assert!(h.app.air_plan.include_in_base_map);
    h.app.trace_build = true;
    h.settle();
}

fn build_t2(h: &mut Harness) -> (crate::ast::Il2Entity, Vec<(String, String)>) {
    t2_inputs(h);
    assert!(
        map_air_check(h).is_some_and(|c| c.ok),
        "the Air Tasking check fails: {}",
        h.status()
    );
    assert!(!h.find("Generate Base Map").disabled);
    generate_map(h, &format!("{T2_STEM}.Group"))
}

#[test]
#[ignore = "writes target/p14/P14_Probe_2T.Group, locale and trace sidecars"]
fn write_p14_probe_2t() {
    let mut h = Harness::new("p14_probe_2t_write");
    build_t2(&mut h);
    copy_flight_files(&h, T2_STEM);
    println!("{}", h.status());
}

/// The facts the 2T run sheet relies on: each clock ticks every 60 s from
/// 60 s; its three jitter timers start 15 s apart and pass 27, 36 and 57 % of
/// the time, so about 20 % of slots stay quiet.
fn check_t2_build(root: &crate::ast::Il2Entity) {
    let air = root.find_by_name("Air Tasking").expect("Air Tasking group");
    for side in ["D", "N"] {
        for (role, time, random) in [
            ("FIRST", 60.0, 100),
            ("PERIOD", 60.0, 100),
            ("JITTER 1", 0.1, 27),
            ("JITTER 2", 15.1, 36),
            ("JITTER 3", 30.1, 57),
            ("SLOT CLOSE", 16.5, 100),
        ] {
            let name = format!("CLK {side} {role}");
            let (t, r) = timer_time_random(air, &name);
            assert!((t - time).abs() < 1e-6 && r == random, "{name}: {t} s, {r} %");
        }
        assert!(air.find_by_name(&format!("CLK {side} JITTER 4")).is_none());
    }
    let quiet: f64 = [27.0_f64, 36.0, 57.0].iter().map(|p| 1.0 - p / 100.0).product();
    assert!((quiet - 0.20).abs() < 0.005, "quiet {quiet:.3}");
    let mut fields = 0;
    root.for_each(&mut |node| fields += usize::from(node.block_type == "Airfield"));
    assert_eq!(fields, 0, "no pilot: no air start");
}

/// Not flown yet: the build matches the frozen files byte for byte.
#[test]
fn t2_map_build_matches_the_run_sheet() {
    let frozen = repo("handoff/p14-probes/flight-2t");
    let mut h = Harness::new("p14_probe_2t");
    let (root, _) = build_t2(&mut h);
    check_t2_build(&root);
    for file in [
        format!("{T2_STEM}.Group"),
        format!("{T2_STEM}.eng"),
        format!("{T2_STEM}.trace.json"),
    ] {
        let built = std::fs::read(h.dir.join(&file)).unwrap();
        let kept = std::fs::read(frozen.join(&file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(built == kept, "{file} differs from the frozen copy");
    }
}
