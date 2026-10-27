//! The six bundled sorties and the bounded Activate-only wiring contract.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{AirRole, AirSide, AirSortie, SortieSource};
use crate::ast::Il2Entity;
use crate::{in_theatre, model_spec, parser, weapon_range};

pub const DEFAULT_CATEGORIES: [&str; 5] = ["Air alert", "Strike", "Close support", "Armed recon", "Interdiction"];

/// Built-ins keep their curated roles; user files fall back to their orders.
pub fn default_category(sortie: &AirSortie) -> String {
    if let SortieSource::BuiltIn(file) = sortie.source {
        let category = match file {
            "1950_US_F80_AirAlert_4ship.Group" => Some("Air alert"),
            "1950_US_F80_HVAR_Strike_4ship.Group" => Some("Strike"),
            "1950_US_F51_CloseSupport_4ship.Group" => Some("Close support"),
            "1950_US_F51_ArmedRecon_2plus2.Group" => Some("Armed recon"),
            "1950_DPRK_Yak_AirfieldRaid_2plus2.Group" => Some("Strike"),
            "1950_DPRK_Il10_KimpoAttack_2x4.Group" => Some("Strike"),
            _ => None,
        };
        if let Some(category) = category {
            return category.into();
        }
    }
    if sortie.lead_roles.iter().flatten().any(|role| *role == AirRole::AirPatrol) {
        "Air alert".into()
    } else {
        "Strike".into()
    }
}

fn category_label(root: &Il2Entity) -> Option<&str> {
    root.property("Desc")?.trim_matches('"').lines().find_map(|line| {
        let (key, name) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case("Category").then_some(name.trim())
    }).filter(|name| !name.is_empty())
}

pub(crate) const BUILTINS: [(&str, &str); 6] = [
    (
        "1950_US_F80_AirAlert_4ship.Group",
        include_str!("../../TemplateExamples/Historical1950/1950_US_F80_AirAlert_4ship.Group"),
    ),
    (
        "1950_US_F80_HVAR_Strike_4ship.Group",
        include_str!("../../TemplateExamples/Historical1950/1950_US_F80_HVAR_Strike_4ship.Group"),
    ),
    (
        "1950_US_F51_CloseSupport_4ship.Group",
        include_str!("../../TemplateExamples/Historical1950/1950_US_F51_CloseSupport_4ship.Group"),
    ),
    (
        "1950_US_F51_ArmedRecon_2plus2.Group",
        include_str!("../../TemplateExamples/Historical1950/1950_US_F51_ArmedRecon_2plus2.Group"),
    ),
    (
        "1950_DPRK_Yak_AirfieldRaid_2plus2.Group",
        include_str!(
            "../../TemplateExamples/Historical1950/1950_DPRK_Yak_AirfieldRaid_2plus2.Group"
        ),
    ),
    (
        "1950_DPRK_Il10_KimpoAttack_2x4.Group",
        include_str!("../../TemplateExamples/Historical1950/1950_DPRK_Il10_KimpoAttack_2x4.Group"),
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestedShape {
    S1,
    S2,
    S3,
    S4,
    S7,
}

/// S7 adds RTB to a supported Activate route (decision 0017).
pub const ACCEPTED_SHAPES: [(TestedShape, &str); 5] = [
    (TestedShape::S1, "Activate: one lead, one AttackArea"),
    (
        TestedShape::S2,
        "Activate: one lead, two chained AttackAreas",
    ),
    (
        TestedShape::S3,
        "Activate: strike and waypoint-free Cover escort, either file order",
    ),
    (
        TestedShape::S4,
        "Activate: two attack leads sharing one waypoint",
    ),
    (TestedShape::S7, "S1--S4 with RTB on some or all leads"),
];

/// All MCU and plane references are indexes, independent of names and groups.
/// `planes` and `leads` are entity ids in Plane file order; `plane_objects`
/// maps each entity to its Plane. Optional maps omit waypoint-free escorts.
#[derive(Debug, Clone, PartialEq)]
pub struct SortieShape {
    pub kind: TestedShape,
    pub side: AirSide,
    pub planes: Vec<i32>,
    pub leads: Vec<i32>,
    pub plane_objects: BTreeMap<i32, i32>,
    pub navigation_lead: i32,
    pub path_waypoints: BTreeMap<i32, i32>,
    pub attack_timers: BTreeMap<i32, i32>,
    pub navigation_attack_timer: i32,
    pub mission_begin: i32,
    pub zone_in: i32,
    pub bring_up_timer: i32,
    pub zone_out: i32,
    pub zone_out_activate: i32,
    pub zone_out_pulse: i32,
    pub end_timer: i32,
    pub end_orders_timer: i32,
    pub delayed_end_timer: i32,
    pub delete_timer: Option<i32>,
    pub delete: Option<i32>,
    pub mission_complete_timers: Vec<i32>,
    pub rtb_waypoints: BTreeMap<i32, i32>,
    pub rtb_timer: Option<i32>,
    pub models: Vec<String>,
    pub lead_roles: Vec<Vec<AirRole>>,
    pub wp_speed_kmh: f64,
    pub wp_alt_m: f64,
    pub attack_time_s: f64,
    pub warnings: Vec<String>,
}

pub(crate) const NO_WAYPOINT: &str = "Air Tasking needs a lead with a path waypoint";
pub(crate) const MULTIPLE_WAYPOINTS: &str = "Air Tasking supports only one path waypoint per lead";
const LAST_ATTACK: &str =
    "Air Tasking needs the attack order to be the last order before Mission Complete";
const HOOKS: &str = "Air Tasking does not support event or report hooks yet";
const ACTIVATE_MODE: &str = "Air Tasking needs an Activate-mode template";
pub(crate) const REGENERATE_TEMPLATE: &str = "Load the template in Template Builder, correct the reported options or links, and press Generate again.";
const PLANES_ONLY: &str = "Air Tasking supports plane units only";
const UNTESTED: &str = "Air Tasking supports only tested sortie shapes S1-S4 and S7";

pub fn builtin_library() -> Vec<AirSortie> {
    BUILTINS
        .iter()
        .map(|&(file, text)| {
            let root = parser::parse_group_file(text).expect("bundled sortie must parse");
            library_item(
                &root,
                SortieSource::BuiltIn(file),
                format!("builtin:{file}"),
            )
            .unwrap_or_else(|error| panic!("{file}: {error}"))
        })
        .collect()
}

pub fn load_user_sortie(path: &Path) -> Result<AirSortie, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let root =
        parser::parse_group_file(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    library_item(
        &root,
        SortieSource::File(path.to_path_buf()),
        format!("file:{}", path.display()),
    )
}

fn library_item(root: &Il2Entity, source: SortieSource, id: String) -> Result<AirSortie, String> {
    let shape = inspect_sortie(root).map_err(|errors| errors.join("; "))?;
    let mut valid = in_theatre::WAR_SPAN;
    let mut warnings = shape.warnings;
    for model in &shape.models {
        if let Some(entry) = in_theatre::for_model(model) {
            valid.from = valid.from.max(entry.valid.from);
            valid.to = valid.to.min(entry.valid.to);
        } else {
            warnings.push(format!(
                "No in-theatre dates for {model}; using the war span"
            ));
        }
    }
    let mut sortie = AirSortie {
        id,
        name: root.name().unwrap_or("Air sortie").to_owned(),
        source,
        side: shape.side,
        models: shape.models,
        planes: shape.planes.len(),
        lead_roles: shape.lead_roles,
        wp_speed_kmh: shape.wp_speed_kmh,
        wp_alt_m: shape.wp_alt_m,
        attack_time_s: shape.attack_time_s,
        valid,
        category: String::new(),
        warnings,
    };
    sortie.category = category_label(root).map(str::to_owned)
        .unwrap_or_else(|| default_category(&sortie));
    Ok(sortie)
}

pub(crate) fn nodes(root: &Il2Entity) -> Vec<&Il2Entity> {
    fn walk<'a>(node: &'a Il2Entity, out: &mut Vec<&'a Il2Entity>) {
        out.push(node);
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

pub(crate) fn by_id(root: &Il2Entity, id: i32) -> &Il2Entity {
    nodes(root)
        .into_iter()
        .find(|node| node.index == Some(id))
        .expect("an inspected sortie retains its indexes")
}

pub(crate) fn number(node: &Il2Entity, key: &str) -> Result<f64, String> {
    node.property(key)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            format!(
                "Air Tasking needs a finite {key} on {} {:?}",
                node.block_type, node.index
            )
        })
}

fn id(node: &Il2Entity) -> i32 {
    node.index.expect("indexed graph node")
}
fn is(node: &Il2Entity, key: &str, value: &str) -> bool {
    node.property(key) == Some(value)
}
fn same_ids(a: &[i32], b: &[i32]) -> bool {
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_unstable();
    b.sort_unstable();
    a == b && a.windows(2).all(|pair| pair[0] != pair[1])
}

fn one<'a>(
    part: &str,
    candidates: impl IntoIterator<Item = &'a Il2Entity>,
) -> Result<&'a Il2Entity, String> {
    let candidates: Vec<_> = candidates.into_iter().collect();
    if candidates.len() != 1 {
        return Err(format!(
            "Air Tasking needs exactly one {part} (found {})",
            candidates.len()
        ));
    }
    Ok(candidates[0])
}

struct Graph<'a> {
    all: Vec<&'a Il2Entity>,
    indexed: BTreeMap<i32, &'a Il2Entity>,
    checked: BTreeSet<i32>,
}

impl<'a> Graph<'a> {
    fn new(root: &'a Il2Entity) -> Result<Self, String> {
        let all = nodes(root);
        let mut indexed = BTreeMap::new();
        for &node in &all {
            if let Some(index) = node.index {
                if indexed.insert(index, node).is_some() {
                    return Err(format!(
                        "Air Tasking needs unique indexes (duplicate {index})"
                    ));
                }
            } else if node.block_type.starts_with("MCU_") || node.block_type == "Plane" {
                return Err(format!("Air Tasking needs an Index on {}", node.block_type));
            }
        }
        Ok(Self {
            all,
            indexed,
            checked: BTreeSet::new(),
        })
    }

    fn kind(&self, kind: &str) -> Vec<&'a Il2Entity> {
        self.all
            .iter()
            .copied()
            .filter(|node| node.block_type == kind)
            .collect()
    }

    fn targets(&self, node: &Il2Entity, kind: &str) -> Vec<&'a Il2Entity> {
        node.targets
            .iter()
            .filter_map(|index| self.indexed.get(index).copied())
            .filter(|target| target.block_type == kind)
            .collect()
    }

    fn incoming(&self, index: i32, kind: &str) -> Vec<&'a Il2Entity> {
        self.kind(kind)
            .into_iter()
            .filter(|node| node.targets.contains(&index))
            .collect()
    }

    fn check(&mut self, node: &Il2Entity, targets: &[i32], objects: &[i32]) -> Result<(), String> {
        if !same_ids(&node.targets, targets) || !same_ids(&node.objects, objects) {
            return Err(format!(
                "{UNTESTED}: unsupported wiring on {} {}",
                node.block_type,
                id(node)
            ));
        }
        if node.block_type == "MCU_Timer"
            && (!is(node, "Random", "100") || number(node, "Time")? < 0.0)
        {
            return Err(format!(
                "{UNTESTED}: order timers must be deterministic and nonnegative"
            ));
        }
        self.checked.insert(id(node));
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        for &node in &self.all {
            for target in node.targets.iter().chain(&node.objects) {
                if !self.indexed.contains_key(target) {
                    return Err(format!("Air Tasking found a dangling link to {target}"));
                }
            }
            if node.block_type.starts_with("MCU_") && !self.checked.contains(&id(node)) {
                return Err(format!(
                    "{UNTESTED}: unsupported {} {}",
                    node.block_type,
                    id(node)
                ));
            }
        }
        Ok(())
    }
}

/// Identify and validate the complete supported graph. No Name property is
/// read here, including for RTB: those waypoints are reached from end orders.
/// The Delete chain is optional. When present, it must list every entity or
/// every Plane block; placement fixes Plane-block lists on the one-shot copy.
pub fn inspect_sortie(root: &Il2Entity) -> Result<SortieShape, Vec<String>> {
    inspect(root).map_err(|error| vec![format!("{error}. {REGENERATE_TEMPLATE}")])
}

fn inspect_routes(
    g: &mut Graph<'_>,
    after: &Il2Entity,
    wp: &Il2Entity,
    leads: &[i32],
    paths: &BTreeMap<i32, i32>,
    navigation_lead: i32,
    covers: usize,
) -> Result<(), String> {
    // Only optional Formation -> Goto WP / Cover may precede the tested
    // attack branches. Every extra MCU must fail the final graph check.
    let mut goto_count = 0;
    let mut cover_count = 0;
    let mut formation_leads = BTreeSet::new();
    for first in g.targets(after, "MCU_Timer") {
        let formations = g.targets(first, "MCU_CMD_Formation");
        let (order, owner) = if formations.is_empty() {
            (first, None)
        } else {
            let formation = one("Formation per route", formations)?;
            let owner = *formation
                .objects
                .first()
                .ok_or_else(|| format!("{UNTESTED}: Formation without a lead"))?;
            if !leads.contains(&owner) || !formation_leads.insert(owner) {
                return Err(format!("{UNTESTED}: duplicate or non-lead Formation"));
            }
            let next = one("order after Formation", g.targets(first, "MCU_Timer"))?;
            g.check(first, &[id(formation), id(next)], &[])?;
            g.check(formation, &[], &[owner])?;
            (next, Some(owner))
        };
        if order.targets.contains(&id(wp)) {
            if owner.is_some_and(|lead| !paths.contains_key(&lead)) {
                return Err(format!("{UNTESTED}: escort routed to a path waypoint"));
            }
            g.check(order, &[id(wp)], &[])?;
            goto_count += 1;
        } else {
            let cmd = one(
                "Cover on a waypoint-free escort",
                g.targets(order, "MCU_CMD_Cover"),
            )?;
            let escort = *leads
                .iter()
                .find(|lead| !paths.contains_key(lead))
                .ok_or_else(|| format!("{UNTESTED}: Cover requires a waypoint-free escort"))?;
            if owner.is_some_and(|lead| lead != escort) {
                return Err(format!("{UNTESTED}: Cover on the strike lead"));
            }
            g.check(order, &[id(cmd)], &[])?;
            g.check(cmd, &[navigation_lead], &[escort])?;
            cover_count += 1;
        }
    }
    if goto_count != paths.len() || cover_count != covers {
        return Err(format!("{UNTESTED}: unsupported bring-up order branches"));
    }
    let starts: Vec<_> = g
        .targets(after, "MCU_Timer")
        .iter()
        .map(|node| id(node))
        .collect();
    g.check(after, &starts, &[])
}

fn zone_side(zone: &Il2Entity) -> Result<AirSide, String> {
    let raw: String = zone
        .property("PlaneCoalitions")
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    match raw.as_str() {
        "[1]" => Ok(AirSide::Nato),
        "[2]" => Ok(AirSide::Dprk),
        _ => Err("Air Tasking needs one opposing plane coalition, [1] or [2]".into()),
    }
}

fn inspect_hooks(g: &Graph<'_>) -> Result<(), String> {
    if g.all.iter().any(|node| {
        matches!(node.block_type.as_str(), "OnEvent" | "OnReport")
            || (matches!(node.block_type.as_str(), "OnEvents" | "OnReports")
                && !node.children.is_empty())
    }) {
        return Err(HOOKS.into());
    }
    Ok(())
}

struct Lifecycle<'a> {
    begin: &'a Il2Entity,
    pulse_in: &'a Il2Entity,
    zone_in: &'a Il2Entity,
    zone_out: &'a Il2Entity,
    react_out: &'a Il2Entity,
    pulse_out: &'a Il2Entity,
    bring_up: &'a Il2Entity,
    bring_command: i32,
    after: &'a Il2Entity,
    end: &'a Il2Entity,
    end_orders: &'a Il2Entity,
    force: &'a Il2Entity,
    delayed_end: &'a Il2Entity,
    deactivate: &'a Il2Entity,
    delete_chain: Option<(&'a Il2Entity, &'a Il2Entity)>,
    rtb_timer: Option<&'a Il2Entity>,
}

fn inspect_lifecycle(
    g: &mut Graph<'_>,
    p: Lifecycle<'_>,
    planes: &[i32],
    leads: &[i32],
    plane_objects: &BTreeMap<i32, i32>,
) -> Result<(), String> {
    let deact_in = one(
        "Zone IN self Deactivate",
        g.incoming(id(p.zone_in), "MCU_Deactivate"),
    )?;
    let deact_out = one(
        "Zone Out self Deactivate",
        g.incoming(id(p.zone_out), "MCU_Deactivate"),
    )?;
    let react_in = one(
        "Zone IN Activate",
        g.incoming(id(p.zone_in), "MCU_Activate"),
    )?;
    let in_targets = vec![
        id(deact_in),
        id(p.react_out),
        id(p.pulse_out),
        id(p.bring_up),
    ];
    let cooldown = one(
        "cooldown timer",
        g.targets(p.zone_out, "MCU_Timer")
            .into_iter()
            .filter(|node| id(node) != id(p.end)),
    )?;
    let out_targets = vec![id(deact_out), id(p.end), id(cooldown)];
    g.check(cooldown, &[id(react_in), id(p.pulse_in)], &[])?;
    g.check(p.begin, &[id(p.pulse_in)], &[])?;
    g.check(p.pulse_in, &[id(p.zone_in)], &[])?;
    g.check(p.pulse_out, &[id(p.zone_out)], &[])?;
    g.check(p.zone_in, &in_targets, &[])?;
    g.check(p.zone_out, &out_targets, &[])?;
    for node in [deact_in, react_in] {
        g.check(node, &[id(p.zone_in)], &[])?;
    }
    for node in [deact_out, p.react_out] {
        g.check(node, &[id(p.zone_out)], &[])?;
    }
    g.check(p.bring_up, &[p.bring_command, id(p.after)], &[])?;
    g.check(p.end, &[id(p.end_orders), id(p.delayed_end)], &[])?;
    let mut orders = vec![id(p.force)];
    if let Some(rtb) = p.rtb_timer {
        orders.push(id(rtb));
    }
    g.check(p.end_orders, &orders, &[])?;
    g.check(p.force, &[], leads)?;
    g.check(p.deactivate, &[], planes)?;
    if let Some((delete_timer, delete)) = p.delete_chain {
        g.check(p.delayed_end, &[id(p.deactivate), id(delete_timer)], &[])?;
        g.check(delete_timer, &[id(delete)], &[])?;
        // Accept exactly one complete representation, never a partial or mixed list.
        let plane_block_indexes: Vec<_> =
            planes.iter().map(|entity| plane_objects[entity]).collect();
        g.check(
            delete,
            &[],
            if same_ids(&delete.objects, planes) {
                planes
            } else {
                &plane_block_indexes
            },
        )
    } else {
        g.check(p.delayed_end, &[id(p.deactivate)], &[])
    }
}

fn inspect(root: &Il2Entity) -> Result<SortieShape, String> {
    if root.block_type != "Group" {
        return Err(format!("{UNTESTED}: the sortie must be a Group"));
    }
    let mut g = Graph::new(root)?;
    if g.all.iter().any(|node| {
        node.block_type != "Plane"
            && (node.property("Model").is_some()
                || matches!(
                    node.block_type.as_str(),
                    "Vehicle" | "Train" | "Ship" | "Block" | "Ground"
                ))
    }) {
        return Err(PLANES_ONLY.into());
    }
    let mut planes = Vec::new();
    let mut leads = Vec::new();
    let mut plane_objects = BTreeMap::new();
    let mut models = Vec::new();
    for plane in g.kind("Plane") {
        let entity = plane
            .property("LinkTrId")
            .and_then(|v| v.parse::<i32>().ok())
            .and_then(|index| g.indexed.get(&index).copied())
            .filter(|node| node.block_type == "MCU_TR_Entity")
            .ok_or_else(|| "Air Tasking needs a linked entity for every plane".to_owned())?;
        if entity
            .property("MisObjID")
            .and_then(|v| v.parse::<i32>().ok())
            != Some(id(plane))
            || plane_objects.insert(id(entity), id(plane)).is_some()
        {
            return Err("Air Tasking needs one reciprocal Plane/entity link per aircraft".into());
        }
        for key in ["XPos", "YPos", "ZPos"] {
            number(plane, key)?;
        }
        if !is(plane, "StartType", "0") {
            return Err("Air Tasking needs air-start planes".into());
        }
        planes.push(id(entity));
        if entity.targets.is_empty() {
            leads.push(id(entity));
        }
        let model = model_spec::script_id(plane.property("Script").unwrap_or("unknown"));
        if !models.contains(&model) {
            models.push(model);
        }
    }
    if planes.is_empty() {
        return Err("Air Tasking needs at least one plane".into());
    }
    for &entity in &planes {
        let node = g.indexed[&entity];
        if !node.targets.is_empty()
            && (node.targets.len() != 1 || !leads.contains(&node.targets[0]))
        {
            return Err(format!("{UNTESTED}: wingmen must link directly to a lead"));
        }
        g.check(node, &node.targets, &[])?;
    }

    let begin = one("Mission Begin translator", g.kind("MCU_TR_MissionBegin"))?;
    let pulse_in = one("Zone IN start timer", g.targets(begin, "MCU_Timer"))?;
    let zone_in = one(
        "Zone IN (Closer = 1)",
        g.targets(pulse_in, "MCU_CheckZone")
            .into_iter()
            .filter(|node| is(node, "Closer", "1")),
    )?;
    let zone_out = one(
        "Zone Out (Closer = 0)",
        g.kind("MCU_CheckZone")
            .into_iter()
            .filter(|node| is(node, "Closer", "0")),
    )?;
    let react_out = one(
        "Zone Out Activate",
        g.incoming(id(zone_out), "MCU_Activate"),
    )?;
    let pulse_out = one(
        "Zone Out pulse timer",
        g.incoming(id(zone_out), "MCU_Timer"),
    )?;
    let has_planes = |node: &&Il2Entity| node.objects.iter().any(|entity| planes.contains(entity));
    // Decision 0017: refuse the counter-to-Spawner path before hook inspection,
    // including Repeat templates whose OnKilled hooks are otherwise valid.
    let bring_up_timers = g.targets(zone_in, "MCU_Timer");
    if bring_up_timers.iter().any(|node| {
        g.targets(node, "MCU_Counter")
            .iter()
            .any(|counter| !g.targets(counter, "MCU_Spawner").is_empty())
    }) {
        return Err(ACTIVATE_MODE.into());
    }
    let bring_up = one(
        "bring-up timer",
        bring_up_timers.into_iter().filter(|node| {
            g.targets(node, "MCU_Activate").iter().any(has_planes)
        }),
    )?;
    let end = one(
        "end timer",
        g.targets(zone_out, "MCU_Timer").into_iter().filter(|node| {
            !g.targets(node, "MCU_CMD_ForceComplete").is_empty()
                || g.targets(node, "MCU_Timer")
                    .iter()
                    .any(|next| !g.targets(next, "MCU_CMD_ForceComplete").is_empty())
        }),
    )?;
    let end_orders = one(
        "end-orders timer",
        g.targets(end, "MCU_Timer")
            .into_iter()
            .filter(|node| !g.targets(node, "MCU_CMD_ForceComplete").is_empty()),
    )?;
    let force = one(
        "end Force Complete",
        g.targets(end_orders, "MCU_CMD_ForceComplete"),
    )?;
    let deletes = g.kind("MCU_Delete");
    let delete_chain = if deletes.is_empty() {
        None
    } else {
        let delete = one("Delete command", deletes)?;
        let delete_timer = one("delete timer", g.incoming(id(delete), "MCU_Timer"))?;
        Some((delete_timer, delete))
    };
    let delayed_end = one(
        "delayed end-orders timer",
        g.targets(end, "MCU_Timer")
            .into_iter()
            .filter(|node| !g.targets(node, "MCU_Deactivate").is_empty()),
    )?;
    let deactivate = one("unit Deactivate", g.targets(delayed_end, "MCU_Deactivate"))?;
    let complete = g.incoming(id(end), "MCU_Timer");
    if complete.is_empty() {
        return Err("Air Tasking needs at least one Mission Complete timer".into());
    }

    let after = one("after-bring-up timer", g.targets(bring_up, "MCU_Timer"))?;
    let activate = one("unit Activate", g.targets(bring_up, "MCU_Activate"))?;
    g.check(activate, &[], &planes)?;
    let bring_command = id(activate);
    inspect_hooks(&g)?;

    // RTB is an end-orders branch, never a label. This also works after rename.
    let rtb_timers = g.targets(end_orders, "MCU_Timer");
    let rtb_timer = if rtb_timers.is_empty() {
        None
    } else {
        Some(one("RTB timer", rtb_timers)?)
    };
    let mut rtb_waypoints = BTreeMap::new();
    let mut rtb_ids = Vec::new();
    if let Some(timer) = rtb_timer {
        for wp in g.targets(timer, "MCU_Waypoint") {
            if wp.objects.is_empty() {
                return Err(format!("{UNTESTED}: RTB waypoint without a lead"));
            }
            for &lead in &wp.objects {
                if !leads.contains(&lead) || rtb_waypoints.insert(lead, id(wp)).is_some() {
                    return Err("Air Tasking needs at most one RTB waypoint per lead".into());
                }
            }
            number(wp, "Speed")?;
            number(wp, "YPos")?;
            rtb_ids.push(id(wp));
            g.check(wp, &[], &wp.objects)?;
        }
        if rtb_ids.is_empty() {
            return Err("Air Tasking needs an RTB waypoint on the RTB timer".into());
        }
        g.check(timer, &rtb_ids, &[])?;
    }
    let path_nodes: Vec<_> = g
        .kind("MCU_Waypoint")
        .into_iter()
        .filter(|node| !rtb_ids.contains(&id(node)))
        .collect();
    let mut path_waypoints = BTreeMap::new();
    for &lead in &leads {
        let waypoints: Vec<_> = path_nodes
            .iter()
            .filter(|node| node.objects.contains(&lead))
            .collect();
        if waypoints.len() > 1 {
            return Err(MULTIPLE_WAYPOINTS.into());
        }
        if let Some(wp) = waypoints.first() {
            path_waypoints.insert(lead, id(wp));
        }
    }
    let navigation_lead = *leads
        .iter()
        .find(|lead| path_waypoints.contains_key(lead))
        .ok_or(NO_WAYPOINT)?;
    let mut attack_timers = BTreeMap::new();
    let mut attack_nodes = BTreeMap::new();
    for &lead in path_waypoints.keys() {
        let attacks: Vec<_> = g
            .kind("MCU_CMD_AttackArea")
            .into_iter()
            .filter(|node| node.objects.contains(&lead))
            .collect();
        let timers: Vec<_> = g
            .kind("MCU_Timer")
            .into_iter()
            .filter(|timer| {
                attacks.iter().any(|cmd| timer.targets.contains(&id(cmd)))
                    && complete
                        .iter()
                        .any(|done| timer.targets.contains(&id(done)))
            })
            .collect();
        if timers.is_empty() {
            return Err(LAST_ATTACK.into());
        }
        attack_timers.insert(lead, id(one("last attack timer per lead", timers)?));
        attack_nodes.insert(lead, attacks);
    }
    let cover = g.kind("MCU_CMD_Cover");
    let attacks_per_lead: Vec<_> = leads
        .iter()
        .map(|lead| attack_nodes.get(lead).map_or(0, Vec::len))
        .collect();
    let base = match (
        leads.len(),
        path_nodes.len(),
        attacks_per_lead.as_slice(),
        cover.len(),
    ) {
        (1, 1, [1], 0) => TestedShape::S1,
        (1, 1, [2], 0) => TestedShape::S2,
        (2, 1, [1, 0] | [0, 1], 1) => TestedShape::S3,
        (2, 1, [1, 1], 0) => TestedShape::S4,
        _ => {
            return Err(format!(
                "{UNTESTED}: unsupported lead, waypoint, AttackArea or Cover combination"
            ));
        }
    };
    let kind = if rtb_timer.is_some() {
        TestedShape::S7
    } else {
        base
    };

    let mut wp_attacks = Vec::new();
    let mut used_complete = Vec::new();
    let mut attack_time_s: f64 = 0.0;
    for (&lead, attacks) in &attack_nodes {
        let last = g.indexed[&attack_timers[&lead]];
        let done = one(
            "Mission Complete per attack branch",
            complete
                .iter()
                .copied()
                .filter(|node| last.targets.contains(&id(node))),
        )?;
        used_complete.push(id(done));
        g.check(done, &[id(end)], &[])?;
        for &command in attacks {
            let timer = one("timer per AttackArea", g.incoming(id(command), "MCU_Timer"))?;
            let next = if id(timer) == id(last) {
                id(done)
            } else {
                id(last)
            };
            g.check(timer, &[id(command), next], &[])?;
            g.check(command, &[], &[lead])?;
            if !is(command, "AttackAir", "1") && !weapon_range::is_ground_attack_area(command) {
                return Err(format!("{UNTESTED}: AttackArea has no air or ground role"));
            }
            let duration = number(command, "Time")?;
            if duration < 0.0 {
                return Err("Air Tasking needs a nonnegative AttackArea Time".into());
            }
            attack_time_s = attack_time_s.max(duration);
            wp_attacks.push(id(timer));
        }
    }
    if !same_ids(
        &used_complete,
        &complete.iter().map(|node| id(node)).collect::<Vec<_>>(),
    ) {
        return Err(format!("{UNTESTED}: unexpected Mission Complete branch"));
    }
    let wp = path_nodes[0];
    g.check(
        wp,
        &wp_attacks,
        &path_waypoints.keys().copied().collect::<Vec<_>>(),
    )?;
    for key in ["XPos", "YPos", "ZPos"] {
        number(wp, key)?;
    }

    inspect_routes(
        &mut g,
        after,
        wp,
        &leads,
        &path_waypoints,
        navigation_lead,
        cover.len(),
    )?;
    inspect_lifecycle(
        &mut g,
        Lifecycle {
            begin,
            pulse_in,
            zone_in,
            zone_out,
            react_out,
            pulse_out,
            bring_up,
            bring_command,
            after,
            end,
            end_orders,
            force,
            delayed_end,
            deactivate,
            delete_chain,
            rtb_timer,
        },
        &planes,
        &leads,
        &plane_objects,
    )?;
    g.finish()?;
    let side = zone_side(zone_in)?;
    if zone_side(zone_out)? != side {
        return Err("Air Tasking needs matching Zone IN/Out plane coalitions".into());
    }
    let mut wp_speed_kmh = number(wp, "Speed")?;
    let mut warnings = Vec::new();
    if wp_speed_kmh <= 150.0 {
        let suggested = f64::from(model_spec::suggested_waypoint_speed_kmh(
            models.iter().map(String::as_str),
        ));
        if suggested <= 150.0 {
            return Err(format!(
                "Air Tasking needs a waypoint speed above 150 km/h; no usable model speed for {}",
                models.join(", ")
            ));
        }
        warnings.push(format!(
            "Waypoint speed {wp_speed_kmh} km/h replaced with model suggestion {suggested} km/h"
        ));
        wp_speed_kmh = suggested;
    }
    let lead_roles = leads
        .iter()
        .map(|lead| {
            let commands: Vec<_> = g
                .all
                .iter()
                .filter(|node| node.objects.contains(lead))
                .collect();
            let mut roles = Vec::new();
            if commands
                .iter()
                .any(|node| node.block_type == "MCU_CMD_AttackArea" && is(node, "AttackAir", "1"))
            {
                roles.push(AirRole::AirPatrol);
            }
            if commands
                .iter()
                .any(|node| weapon_range::is_ground_attack_area(node))
            {
                roles.push(AirRole::GroundAttack);
            }
            if commands
                .iter()
                .any(|node| node.block_type == "MCU_CMD_Cover")
            {
                roles.push(AirRole::Escort);
            }
            roles
        })
        .collect();
    Ok(SortieShape {
        kind,
        side,
        planes,
        leads,
        plane_objects,
        navigation_lead,
        navigation_attack_timer: attack_timers[&navigation_lead],
        path_waypoints,
        attack_timers,
        mission_begin: id(begin),
        zone_in: id(zone_in),
        bring_up_timer: id(bring_up),
        zone_out: id(zone_out),
        zone_out_activate: id(react_out),
        zone_out_pulse: id(pulse_out),
        end_timer: id(end),
        end_orders_timer: id(end_orders),
        delayed_end_timer: id(delayed_end),
        delete_timer: delete_chain.map(|(timer, _)| id(timer)),
        delete: delete_chain.map(|(_, delete)| id(delete)),
        mission_complete_timers: used_complete,
        rtb_waypoints,
        rtb_timer: rtb_timer.map(id),
        models,
        lead_roles,
        wp_speed_kmh,
        wp_alt_m: number(wp, "YPos")?,
        attack_time_s,
        warnings,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::airtask::testkit::assert_links_resolve;
    use crate::template::{
        self, BringUp, EntityEvent, EventHook, EventThen, OrderKind, OrderSpec, TemplateOptions,
        TemplateSeat, UnitKind, ZoneCoalition,
        LOOSE_AIR_FORMATION_DENSITY, PAIRS_AIR_FORMATION_TYPE, SAFE_AIR_FORMATION_DENSITY,
    };

    #[test]
    fn category_defaults_match_all_six_builtin_files_and_user_orders() {
        let library = builtin_library();
        assert_eq!(library.iter().map(|s| s.category.as_str()).collect::<Vec<_>>(),
            ["Air alert", "Strike", "Close support", "Armed recon", "Strike", "Strike"]);
        for (i, original) in library.into_iter().enumerate() {
            let mut sortie = original;
            sortie.name = "Renamed".into();
            sortie.category = "Custom".into();
            assert_eq!(default_category(&sortie), ["Air alert", "Strike", "Close support", "Armed recon", "Strike", "Strike"][i]);
            sortie.source = SortieSource::File("custom.Group".into());
            assert_eq!(default_category(&sortie), if i == 0 { "Air alert" } else { "Strike" });
        }
    }

    #[test]
    fn category_reader_uses_first_top_level_label_with_trimmed_preserved_name() {
        for source in [SortieSource::BuiltIn(BUILTINS[1].0), SortieSource::File("custom.Group".into())] {
            let mut root = builtin(1);
            root.set_property("Desc", "\"Notes\r\n  cAtEgOrY:  Naval interdiction  \r\nCategory: Strike\"".to_string());
            root.children[0].set_property("Desc", "\"Category: Ignored child\"".to_string());
            let before = root.clone();
            let sortie = library_item(&root, source, "custom".into()).unwrap();
            assert_eq!(sortie.category, "Naval interdiction");
            assert_eq!(root, before, "reading never writes the template");
        }
    }

    #[test]
    fn category_reader_empty_or_absent_label_uses_default_and_ignores_children() {
        for desc in ["\"Category:   \nCategory: Interdiction\"", "\"No label\"", "\"OtherCategory: Interdiction\""] {
            let mut root = builtin(2);
            root.set_property("Desc", desc.to_string());
            root.children[0].set_property("Desc", "\"Category: Interdiction\"".to_string());
            assert_eq!(library_item(&root, SortieSource::BuiltIn(BUILTINS[2].0), "built-in".into()).unwrap().category, "Close support");
            assert_eq!(library_item(&root, SortieSource::File("custom.Group".into()), "file".into()).unwrap().category, "Strike");
        }
    }

    /// Formation index, lead entity, type and density, in BUILTINS order.
    pub(crate) const BUILTIN_FORMATIONS: [&[(i32, i32, i32, i32)]; BUILTINS.len()] = [
        &[(35, 7, 20, SAFE_AIR_FORMATION_DENSITY)],
        &[(35, 7, 20, SAFE_AIR_FORMATION_DENSITY)],
        &[(35, 7, 20, SAFE_AIR_FORMATION_DENSITY)],
        &[(35, 7, PAIRS_AIR_FORMATION_TYPE, LOOSE_AIR_FORMATION_DENSITY),
          (41, 11, PAIRS_AIR_FORMATION_TYPE, SAFE_AIR_FORMATION_DENSITY)],
        &[(35, 7, PAIRS_AIR_FORMATION_TYPE, LOOSE_AIR_FORMATION_DENSITY),
          (41, 11, PAIRS_AIR_FORMATION_TYPE, LOOSE_AIR_FORMATION_DENSITY)],
        &[(43, 7, 20, SAFE_AIR_FORMATION_DENSITY), (49, 15, 20, SAFE_AIR_FORMATION_DENSITY)],
    ];

    pub(crate) fn builtin(index: usize) -> Il2Entity {
        parser::parse_group_file(BUILTINS[index].1).unwrap()
    }

    pub(crate) fn order(kind: OrderKind) -> OrderSpec {
        OrderSpec {
            kind,
            attack_air: false,
            attack_g_targets: true,
            ..OrderSpec::default()
        }
    }

    pub(crate) fn options(shape: TestedShape) -> TemplateOptions {
        let unit = template::builtin_plane_catalog()
            .into_iter()
            .find(|unit| model_spec::script_id(&unit.script) == "f80c10")
            .unwrap();
        let mut seat = TemplateSeat::new(unit);
        seat.altitude = 2100.0;
        seat.start_type = 0;
        seat.orders = vec![
            order(OrderKind::Formation),
            order(OrderKind::GotoWaypoint),
            order(OrderKind::AttackArea),
            order(OrderKind::MissionComplete),
        ];
        let mut opts = TemplateOptions {
            name: "Generated P14 shape".into(),
            seats: vec![seat],
            per_group: 1,
            waypoint_speed: 660.0,
            waypoint_altitude: 1800.0,
            zone_coalition: ZoneCoalition::Eastern,
            ..TemplateOptions::default()
        };
        match shape {
            TestedShape::S1 => {}
            TestedShape::S2 => {
                let mut air = order(OrderKind::AttackArea);
                air.attack_air = true;
                air.attack_g_targets = false;
                opts.seats[0].orders.insert(2, air);
            }
            TestedShape::S3 => {
                let mut escort = opts.seats[0].clone();
                escort.altitude = 3200.0;
                let mut cover = order(OrderKind::Cover);
                cover.cover_lead = Some(0);
                escort.orders = vec![order(OrderKind::Formation), cover];
                opts.seats.push(escort);
            }
            TestedShape::S4 | TestedShape::S7 => {
                opts.seats.push(opts.seats[0].clone());
                if shape == TestedShape::S7 {
                    opts.seats[0].orders.push(order(OrderKind::RtbOnZoneOut));
                }
            }
        }
        opts
    }

    fn spawn_options() -> TemplateOptions {
        TemplateOptions {
            bring_up: BringUp::Spawn,
            ..options(TestedShape::S1)
        }
    }

    pub(crate) fn generate(opts: &TemplateOptions) -> Il2Entity {
        let generated = template::generate_template(opts).unwrap();
        assert_eq!(
            generated.count_block_type("MCU_Delete"),
            match opts.bring_up {
                BringUp::Activate => 0,
                BringUp::Spawn => 1,
            }
        );
        parser::parse_group_file(&crate::serialize::serialize_group(&generated)).unwrap()
    }

    pub(crate) fn swap_flights(root: &mut Il2Entity) {
        let units = root
            .children
            .iter_mut()
            .find(|group| group.children.iter().any(|node| node.block_type == "Plane"))
            .unwrap();
        let half = units.children.len() / 2;
        units.children.rotate_left(half);
    }

    // Every accepted route, with and without Formation and each RTB combination.
    fn shape_cases() -> Vec<(TestedShape, Vec<Il2Entity>)> {
        use TestedShape::*;
        let mut swapped = builtin(3);
        swap_flights(&mut swapped);
        let mut rows = vec![
            (S1, vec![builtin(1), builtin(2)]),
            (S2, vec![builtin(0)]),
            (S3, vec![builtin(3), swapped]),
            (S4, vec![builtin(4), builtin(5)]),
            (S7, Vec::new()),
        ];
        for (slot, base) in [S1, S2, S3, S4].into_iter().enumerate() {
            for formation in [false, true] {
                let mut opts = options(base);
                if !formation {
                    for seat in &mut opts.seats {
                        seat.orders
                            .retain(|order| order.kind != OrderKind::Formation);
                    }
                }
                rows[slot].1.push(generate(&opts));
                if base == S3 {
                    let mut swapped = generate(&opts);
                    swap_flights(&mut swapped);
                    rows[slot].1.push(swapped);
                }
                let masks = if opts.seats.len() == 1 { vec![1] } else { vec![1, 2, 3] };
                for mask in masks {
                    let mut rtb = opts.clone();
                    for (n, seat) in rtb.seats.iter_mut().enumerate() {
                        if mask & (1 << n) != 0 {
                            seat.orders.push(order(OrderKind::RtbOnZoneOut));
                        }
                    }
                    rows[4].1.push(generate(&rtb));
                }
            }
        }
        // Template Builder can share one RTB among leads in the same bucket.
        let mut shared = options(S4);
        shared.per_group = 4;
        for seat in &mut shared.seats {
            seat.orders.push(order(OrderKind::RtbOnZoneOut));
        }
        rows[4].1.push(generate(&shared));
        rows
    }

    #[test]
    fn library_builtins_load_with_side_and_roles() {
        use AirRole::*;
        let expected = [
            (
                AirSide::Nato,
                vec![vec![AirPatrol, GroundAttack]],
                "f80c10",
                4,
            ),
            (AirSide::Nato, vec![vec![GroundAttack]], "f80c10", 4),
            (AirSide::Nato, vec![vec![GroundAttack]], "f51d", 4),
            (
                AirSide::Nato,
                vec![vec![GroundAttack], vec![Escort]],
                "f51d",
                4,
            ),
            (
                AirSide::Dprk,
                vec![vec![GroundAttack], vec![GroundAttack]],
                "yak9p",
                4,
            ),
            (
                AirSide::Dprk,
                vec![vec![GroundAttack], vec![GroundAttack]],
                "il10",
                8,
            ),
        ];
        let library = builtin_library();
        assert_eq!(library.len(), 6);
        assert_eq!(
            library
                .iter()
                .map(|item| &item.id)
                .collect::<BTreeSet<_>>()
                .len(),
            6
        );
        for (i, (item, (side, roles, model, planes))) in library.iter().zip(expected).enumerate() {
            assert_eq!(item.side, side);
            assert_eq!(item.lead_roles, roles);
            assert_eq!(item.models, [model]);
            assert_eq!(item.planes, planes);
            assert_eq!(item.valid, in_theatre::WAR_SPAN);
            assert!(item.warnings.is_empty());
            assert_eq!(item.source, SortieSource::BuiltIn(BUILTINS[i].0));
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("TemplateExamples/Historical1950")
                .join(BUILTINS[i].0);
            let user = load_user_sortie(&path).unwrap();
            assert_eq!(user.source, SortieSource::File(path));
            assert_eq!(user.name, item.name);
            assert_eq!(user.lead_roles, roles);
            assert_eq!(user.side, side);
            assert_eq!(user.wp_speed_kmh, item.wp_speed_kmh);
        }
        assert!(load_user_sortie(Path::new("a-nonexistent-p14-sortie.Group")).is_err());
    }

    #[test]
    fn inspect_sortie_accepts_cleanup_without_delete() {
        let mut cases = vec![
            (Some(TestedShape::S1), options(TestedShape::S1)),
            (Some(TestedShape::S7), options(TestedShape::S7)),
        ];
        for repeat in [false, true] {
            let mut opts = spawn_options();
            opts.allow_multiple_spawns = repeat;
            cases.push((None, opts));
        }
        for (kind, opts) in cases {
            let mut generated = generate(&opts);
            let deactivate = generated
                .find_by_name("Deactivate Units")
                .unwrap()
                .index
                .unwrap();
            generated
                .find_by_name_mut("DELAYED END ORDERS")
                .unwrap()
                .set_targets(vec![deactivate]);
            for name in ["Trigger Delete", "DELETE DELAY"] {
                if let Some(index) = generated.find_by_name(name).and_then(|node| node.index) {
                    remove(&mut generated, index);
                }
            }
            let Some(kind) = kind else {
                assert_eq!(
                    inspect_sortie(&generated).unwrap_err(),
                    [
                        "Air Tasking needs an Activate-mode template. Load the template in Template Builder, correct the reported options or links, and press Generate again."
                    ]
                );
                continue;
            };
            let shape = inspect_sortie(&generated).unwrap();
            assert_eq!(shape.delete, None);
            assert_eq!(shape.delete_timer, None);
            assert_eq!(shape.kind, kind);

            generated
                .find_by_name_mut("DELAYED END ORDERS")
                .unwrap()
                .append_target(shape.end_orders_timer);
            assert!(inspect_sortie(&generated).is_err());
        }
    }

    #[test]
    fn inspect_sortie_rejects_incomplete_or_duplicate_delete_chains() {
        let original = builtin(1);
        let shape = inspect_sortie(&original).unwrap();
        for index in [shape.delete.unwrap(), shape.delete_timer.unwrap()] {
            let mut missing = original.clone();
            remove(&mut missing, index);
            assert!(inspect_sortie(&missing).is_err(), "missing {index}");
            let mut duplicate = original.clone();
            let mut extra = by_id(&original, index).clone();
            extra.index = Some(duplicate.max_index() + 1);
            extra.set_property("Index", extra.index.unwrap().to_string());
            duplicate.children.push(extra);
            assert!(inspect_sortie(&duplicate).is_err(), "duplicate {index}");
        }
        let mut disconnected = original.clone();
        disconnected
            .find_by_name_mut("DELAYED END ORDERS")
            .unwrap()
            .set_targets(vec![
                original
                    .find_by_name("Deactivate Units")
                    .unwrap()
                    .index
                    .unwrap(),
            ]);
        assert!(inspect_sortie(&disconnected).is_err());
    }

    #[test]
    fn inspect_sortie_delete_contract_accepts_complete_entity_or_legacy_plane_lists() {
        let legacy = builtin(1);
        let shape = inspect_sortie(&legacy).unwrap();
        let mut corrected = legacy.clone();
        corrected.find_by_name_mut("Trigger Delete").unwrap().set_objects(shape.planes.clone());
        assert_eq!(inspect_sortie(&corrected).unwrap(), shape);

        for objects in [
            vec![],
            vec![shape.planes[0]],
            vec![shape.planes[0]; shape.planes.len()],
            shape.planes.iter().enumerate().map(|(i, entity)| {
                if i == 0 { *entity } else { shape.plane_objects[entity] }
            }).collect(),
        ] {
            let mut invalid = legacy.clone();
            invalid.find_by_name_mut("Trigger Delete").unwrap().set_objects(objects);
            assert!(inspect_sortie(&invalid).is_err());
        }
        for repeat in [false, true] {
            let mut opts = spawn_options();
            opts.allow_multiple_spawns = repeat;
            let generated = template::generate_template(&opts).unwrap();
            assert_eq!(inspect_sortie(&generated).unwrap_err(), [
                "Air Tasking needs an Activate-mode template. Load the template in Template Builder, correct the reported options or links, and press Generate again."
            ]);
        }
    }

    #[test]
    fn inspect_sortie_finds_the_parts_by_wiring() {
        for i in 0..6 {
            let root = builtin(i);
            let shape = inspect_sortie(&root).unwrap();
            for (index, name) in [
                (shape.zone_in, "Zone IN"),
                (shape.bring_up_timer, "MISSION BEGIN"),
                (shape.zone_out, "Zone Out"),
                (shape.zone_out_activate, "Zone Out ReActivate"),
                (shape.zone_out_pulse, "PULSE OUT"),
                (shape.end_timer, "MISSION END"),
                (shape.delete_timer.unwrap(), "DELETE DELAY"),
                (shape.end_orders_timer, "MISSION END ORDERS"),
                (shape.delayed_end_timer, "DELAYED END ORDERS"),
            ] {
                assert_eq!(root.find_by_name(name).unwrap().index, Some(index));
            }
            let expected_complete: Vec<_> = nodes(&root)
                .into_iter()
                .filter(|node| {
                    node.name()
                        .is_some_and(|name| name.starts_with("Mission Complete "))
                })
                .map(id)
                .collect();
            assert_eq!(shape.mission_complete_timers, expected_complete);
            assert_eq!(shape.navigation_attack_timer, [39, 37, 37, 37, 37, 45][i]);
            let timer = by_id(&root, shape.navigation_attack_timer);
            assert!(timer.targets.iter().any(|target| {
                let command = by_id(&root, *target);
                command.block_type == "MCU_CMD_AttackArea" && command.objects.contains(&7)
            }));
        }
    }

    #[test]
    fn inspect_sortie_does_not_read_names() {
        let mut roots = vec![
            builtin(1),
            generate(&options(TestedShape::S7)),
        ];
        let mut spawn = spawn_options();
        spawn.allow_multiple_spawns = true;
        roots.push(generate(&spawn));
        for root in &mut roots {
            let expected = inspect_sortie(root);
            root.for_each_mut(&mut |node| node.set_name("X"));
            assert_eq!(inspect_sortie(root), expected);
        }
    }

    #[test]
    fn inspect_sortie_refuses_on_killed_repeat_events() {
        let mut opts = spawn_options();
        opts.allow_multiple_spawns = true;
        opts.seats.push(opts.seats[0].clone());
        let mut root = generate(&opts);
        let mut events = 0;
        root.for_each_mut(&mut |node| {
            if node.block_type == "OnEvent" {
                node.set_property("Type", "13");
                events += 1;
            }
        });
        assert_eq!(events, 2);
        assert_eq!(
            inspect_sortie(&root).unwrap_err(),
            [
                "Air Tasking needs an Activate-mode template. Load the template in Template Builder, correct the reported options or links, and press Generate again."
            ]
        );
    }

    #[test]
    fn inspect_sortie_refuses_legacy_on_plane_destroyed_repeat_events() {
        let mut opts = spawn_options();
        opts.allow_multiple_spawns = true;
        let mut root = generate(&opts);
        let mut events = 0;
        root.for_each_mut(&mut |node| {
            if node.block_type == "OnEvent" {
                node.set_property("Type", "4");
                events += 1;
            }
        });
        assert_eq!(events, 1);
        assert_eq!(
            inspect_sortie(&root).unwrap_err(),
            [
                "Air Tasking needs an Activate-mode template. Load the template in Template Builder, correct the reported options or links, and press Generate again."
            ]
        );
    }

    #[test]
    fn inspect_sortie_accepts_each_tested_shape() {
        for (kind, cases) in shape_cases() {
            assert!(!cases.is_empty());
            for root in cases {
                assert_links_resolve(&root);
                let shape =
                    inspect_sortie(&root).unwrap_or_else(|errors| panic!("{kind:?}: {errors:?}"));
                assert_eq!(shape.kind, kind);
                assert_eq!(root.count_block_type("OnEvent"), 0);
            }
        }
    }

    #[test]
    fn inspect_sortie_refuses_spawn_mode() {
        for repeat in [false, true] {
            let mut opts = spawn_options();
            opts.allow_multiple_spawns = repeat;
            let root = generate(&opts);
            assert_eq!(root.count_block_type("Plane"), 1);
            assert_eq!(inspect_sortie(&root).unwrap_err(), [
                "Air Tasking needs an Activate-mode template. Load the template in Template Builder, correct the reported options or links, and press Generate again."
            ]);
        }
    }

    fn remove(root: &mut Il2Entity, index: i32) {
        root.children.retain(|node| node.index != Some(index));
        for node in &mut root.children {
            remove(node, index);
        }
    }

    #[test]
    fn inspect_sortie_refusals() {
        let mut cases: Vec<(&str, Il2Entity, String)> = Vec::new();
        let base = options(TestedShape::S1);
        let mut no_out = generate(&base);
        let out = no_out.find_by_name("Zone Out").unwrap().index.unwrap();
        remove(&mut no_out, out);
        cases.push((
            "missing Zone Out",
            no_out,
            "Air Tasking needs exactly one Zone Out (Closer = 0) (found 0)".into(),
        ));
        let mut two_out = generate(&base);
        let mut extra = two_out.find_by_name("Zone Out").unwrap().clone();
        extra.index = Some(two_out.max_index() + 1);
        extra.set_property("Index", extra.index.unwrap().to_string());
        two_out.children.push(extra);
        cases.push((
            "two Zone Outs",
            two_out,
            "Air Tasking needs exactly one Zone Out (Closer = 0) (found 2)".into(),
        ));
        let mut vehicle = base.clone();
        vehicle.seats.push(TemplateSeat::new(
            template::bundled_catalog()
                .into_iter()
                .find(|unit| unit.kind == UnitKind::Vehicle)
                .unwrap(),
        ));
        cases.push(("vehicle", generate(&vehicle), PLANES_ONLY.into()));
        let mut no_wp = base.clone();
        no_wp.seats[0]
            .orders
            .retain(|order| order.kind != OrderKind::GotoWaypoint);
        cases.push(("no waypoint", generate(&no_wp), NO_WAYPOINT.into()));
        let mut two_wp = base.clone();
        let mut second = order(OrderKind::GotoWaypoint);
        second.waypoint = 2;
        two_wp.seats[0].orders.insert(2, second);
        cases.push((
            "two waypoints",
            generate(&two_wp),
            MULTIPLE_WAYPOINTS.into(),
        ));
        let mut no_attack = base.clone();
        no_attack.seats[0]
            .orders
            .retain(|order| order.kind != OrderKind::AttackArea);
        cases.push(("no AttackArea", generate(&no_attack), LAST_ATTACK.into()));
        let mut tot = base.clone();
        tot.seats[0]
            .orders
            .insert(3, order(OrderKind::TimeOnTarget));
        cases.push(("time on target", generate(&tot), LAST_ATTACK.into()));
        let mut event = base.clone();
        event.seats[0].events.push(EventHook {
            kind: EntityEvent::OnPlaneBingoFuel,
            then: EventThen::ForceComplete,
        });
        cases.push(("event hook", generate(&event), HOOKS.into()));
        let mut report = base.clone();
        report.seats[0]
            .orders
            .insert(3, order(OrderKind::OnAreaAttacked));
        cases.push(("report hook", generate(&report), HOOKS.into()));
        let mut unknown = generate(&TemplateOptions {
            waypoint_speed: 100.0,
            ..base
        });
        unknown.for_each_mut(&mut |node| {
            if node.block_type == "Plane" {
                node.set_property("Script", "\"unknown-p14-model.txt\"");
            }
        });
        cases.push(("unknown low speed", unknown, "Air Tasking needs a waypoint speed above 150 km/h; no usable model speed for unknown-p14-model".into()));
        for (label, root, message) in cases {
            assert_eq!(
                inspect_sortie(&root).unwrap_err(),
                [format!(
                    "{message}. Load the template in Template Builder, correct the reported options or links, and press Generate again."
                )],
                "{label}"
            );
        }

        // Every unique lifecycle part is mandatory, even when its label exists.
        let generated = generate(&options(TestedShape::S1));
        let shape = inspect_sortie(&generated).unwrap();
        for index in [
            shape.mission_begin,
            shape.zone_in,
            shape.bring_up_timer,
            shape.zone_out_activate,
            shape.zone_out_pulse,
            shape.end_timer,
            shape.end_orders_timer,
            shape.delayed_end_timer,
        ] {
            let mut missing = generated.clone();
            remove(&mut missing, index);
            assert!(inspect_sortie(&missing).is_err(), "missing {index}");
            let mut duplicate = generated.clone();
            let mut extra = by_id(&duplicate, index).clone();
            extra.index = Some(duplicate.max_index() + 1);
            extra.set_property("Index", extra.index.unwrap().to_string());
            duplicate.children.push(extra);
            assert!(inspect_sortie(&duplicate).is_err(), "duplicate {index}");
        }
    }

    #[test]
    fn inspect_sortie_accepts_nothing_untested() {
        let cases = shape_cases();
        assert_eq!(ACCEPTED_SHAPES.len(), cases.len());
        assert_eq!(
            ACCEPTED_SHAPES.iter().map(|row| row.0).collect::<Vec<_>>(),
            cases.iter().map(|row| row.0).collect::<Vec<_>>()
        );
        let mut three_attacks = options(TestedShape::S2);
        three_attacks.seats[0]
            .orders
            .insert(2, order(OrderKind::AttackArea));
        let mut three_leads = options(TestedShape::S4);
        three_leads.seats.push(three_leads.seats[0].clone());
        let mut separate_waypoints = options(TestedShape::S4);
        separate_waypoints.seats[1].orders[1].waypoint = 2;
        let mut extra_order = options(TestedShape::S1);
        extra_order.seats[0]
            .orders
            .insert(1, order(OrderKind::Behaviour));
        for opts in [three_attacks, three_leads, separate_waypoints, extra_order] {
            assert!(inspect_sortie(&generate(&opts)).is_err());
        }
        let mut non_group = generate(&options(TestedShape::S1));
        non_group.block_type = "Airfield".into();
        assert_eq!(
            inspect_sortie(&non_group).unwrap_err(),
            [
                "Air Tasking supports only tested sortie shapes S1-S4 and S7: the sortie must be a Group. Load the template in Template Builder, correct the reported options or links, and press Generate again."
            ]
        );
        let mut extra_end = generate(&options(TestedShape::S1));
        let end = extra_end
            .find_by_name("MISSION END")
            .unwrap()
            .index
            .unwrap();
        let mut next = extra_end.max_index() + 1;
        let mut timer = template::timer("unexpected end input", 1.0, &mut next, 0.0, 0.0);
        timer.set_targets(vec![end]);
        extra_end.children.push(timer);
        assert!(inspect_sortie(&extra_end).is_err());
    }

    #[test]
    fn inspect_sortie_escort_first_picks_the_second_lead() {
        let mut root = builtin(3);
        swap_flights(&mut root);
        let shape = inspect_sortie(&root).unwrap();
        assert_eq!(shape.leads, [11, 7]);
        assert_eq!(shape.navigation_lead, 7);
        assert_eq!(shape.navigation_attack_timer, 37);
        assert_eq!(
            shape.lead_roles,
            [vec![AirRole::Escort], vec![AirRole::GroundAttack]]
        );
    }

    #[test]
    fn builtin_air_formations_use_loose_pairs_except_cover_and_match_shared_rule() {
        let mut formations = 0;
        for ((file, text), expected) in BUILTINS.into_iter().zip(BUILTIN_FORMATIONS) {
            let root = parser::parse_group_file(text).unwrap_or_else(|error| panic!("{file}: {error}"));
            let mut in_file = 0;
            root.for_each(&mut |node| {
                if node.block_type == "MCU_CMD_Formation" {
                    let &(index, lead, formation_type, density) = expected.iter()
                        .find(|row| Some(row.0) == node.index)
                        .unwrap_or_else(|| panic!("{file}: unexpected Formation {:?}", node.index));
                    assert_eq!(node.objects, [lead], "{file}: Formation {index} lead");
                    assert_eq!(node.property("FormationType"), Some(formation_type.to_string().as_str()), "{file}: Formation {index}");
                    assert_eq!(node.property("FormationDensity"), Some(density.to_string().as_str()), "{file}: Formation {index}");
                    in_file += 1;
                }
            });
            assert_eq!(in_file, expected.len(), "{file}: wrong Formation command count");
            formations += in_file;
            let mut written = root.clone();
            template::set_air_formation_density(&mut written);
            assert_eq!(written, root, "{file}: built-in must already follow the shared rule");
            assert_eq!(crate::serialize::serialize_group(&written), text, "{file}: every byte, including CRLF, must match");
        }
        assert_eq!(formations, 9);
    }

    #[test]
    fn library_leads_are_entities_with_empty_targets() {
        for (i, expected) in [
            vec![7],
            vec![7],
            vec![7],
            vec![7, 11],
            vec![7, 11],
            vec![7, 15],
        ]
        .into_iter()
        .enumerate()
        {
            let root = builtin(i);
            let shape = inspect_sortie(&root).unwrap();
            assert_eq!(shape.leads, expected);
            for entity in shape.planes {
                let node = by_id(&root, entity);
                assert_eq!(node.block_type, "MCU_TR_Entity");
                assert_eq!(node.targets.is_empty(), expected.contains(&entity));
                let plane = by_id(&root, shape.plane_objects[&entity]);
                assert_eq!(plane.block_type, "Plane");
                assert_eq!(plane.property("LinkTrId").unwrap(), entity.to_string());
            }
        }
    }

    #[test]
    fn library_speed_alt_attack_time_per_file() {
        let library = builtin_library();
        for (i, (speed, altitude, duration)) in [
            (660.0, 3050.0, 1080.0),
            (660.0, 3050.0, 600.0),
            (520.0, 1500.0, 900.0),
            (520.0, 900.0, 1200.0),
            (500.0, 3050.0, 300.0),
            (390.0, 1500.0, 600.0),
        ]
        .into_iter()
        .enumerate()
        {
            let item = &library[i];
            assert_eq!(
                (item.wp_speed_kmh, item.wp_alt_m, item.attack_time_s),
                (speed, altitude, duration)
            );
            let shape = inspect_sortie(&builtin(i)).unwrap();
            assert_eq!(
                (shape.wp_speed_kmh, shape.wp_alt_m, shape.attack_time_s),
                (speed, altitude, duration)
            );
        }
    }
}
