//! IL-2 Group Generator — binary crate for IL-2 Sturmovik: Great Battles
//! (Korea map).
//!
//! Declares every `src/` module and boots the egui app via [`ui::run`].
//! There is no `lib.rs`. See `docs/src-guide.md` for the per-file map.
//! Parsing lives in [`parser`] → [`ast`]; writing in [`serialize`].

mod aircraft;
mod airfield;
mod ast;
mod bombers;
mod duplicate;
mod flights;
mod frontlines;
mod geo;
mod harvest;
mod help;
mod locale;
mod mapclip;
mod mapfighters;
mod mapground;
mod mapload;
mod mapnet;
mod mapshipping;
mod model_spec;
mod pack;
mod placement;
mod parser;
mod payloads;
mod recon;
mod serialize;
mod shell;
mod template;
mod theme;
mod ui;
mod watermap;
mod weapon_range;

fn main() -> eframe::Result {
    ui::run()
}
