//! Binary-side Slint GUI: callbacks and window lifecycle.
//!
//! This module is declared from `main.rs` only and lives entirely in the
//! binary crate, where the `slint::include_modules!()` types are available.

use rfd::FileDialog;
use crate::calc::{SHIP_FILE_EXT, SS_SHIP_FILE_EXT, Ship};
use crate::calc::hull_draw;
use slint::ComponentHandle;

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use crate::{AboutDialog, MainWindow};

pub mod gui_map;

// GUI helpers {{{1
//
// pull_all {{{2
/// Read all editable fields from the Slint UI back into the Ship
///
fn pull_all(ui: &MainWindow, ship: &mut Ship) {
    gui_map::pull_asw(ui, ship);
    gui_map::pull_identity(ui, ship);
    gui_map::pull_hull(ui, ship);
    gui_map::pull_mines(ui, ship);
    gui_map::pull_torpedoes(ui, ship);
    gui_map::pull_weights(ui, ship);
}

// push_derived {{{2
/// Refresh the derived, read-only UI boxes, hull image and report.
///
/// The full pushes (push_identity, push_hull) are deliberately not used
/// here: they would reformat the field under an active caret and push_hull
/// resets the depth lock, so only the derived boxes are rewritten.
///
fn push_derived(ship: &Ship, ui: &MainWindow) {
    gui_map::push_hull_derived(ship, ui);
    gui_map::push_hull_image(ship, ui);
    gui_map::push_torp_wgt(ship, ui);
    gui_map::push_mine_total_wgt(ship, ui);
    gui_map::push_asw_total_wgt(ship, ui);
    gui_map::push_weight_derived(ship, ui);
    ui.set_report_str(ship.report().into());
}

// pull_then_push {{{2
/// Pull editable fields from the UI, then push derived values back.
///
fn pull_then_push(ui: &MainWindow, ship: &mut Ship) {
    pull_all(ui, ship);
    push_derived(ship, ui);
}

// push_all {{{2
/// Push Ship fields and report from the Ship into the Slint UI
///
fn push_all(ship: &Ship, ui: &MainWindow) {
    gui_map::push_asw(ship, ui);
    gui_map::push_identity(ship, ui);
    gui_map::push_hull(ship, ui);
    gui_map::push_hull_image(ship, ui);
    gui_map::push_mines(ship, ui);
    gui_map::push_torpedoes(ship, ui);
    gui_map::push_weights(ship, ui);
    ui.set_report_str(ship.report().into());
}

// pick_file {{{2
/// Build a FileDialog with title and filter
///
fn pick_file(title: &str, ext: &str) -> Option<String> {
    FileDialog::new()
        .set_title(title)
        .add_filter(ext, &[ext])
        .add_filter("all", &["*"])
        .pick_file()
        .map(|p| p.into_os_string().into_string().unwrap())
}

// save_file_dialog {{{2
/// Open a save-file dialog. Returns the selected path as a String
///
fn save_file_dialog(title: &str, ext: &str, default_name: &str) -> Option<String> {
    FileDialog::new()
        .set_title(title)
        .set_file_name(default_name)
        .add_filter(ext, &[ext])
        .add_filter("all", &["*"])
        .save_file()
        .map(|p| p.into_os_string().into_string().unwrap())
}

// Callback handlers {{{1
//
// clear_ship {{{2
fn clear_ship(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    *ship.borrow_mut() = Ship::default();
    push_all(&ship.borrow(), ui);
}

// convert_ship {{{2
fn convert_ship(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    if let Some(file) = pick_file("SpringSharp file to convert", SS_SHIP_FILE_EXT) {
        if let Ok(loaded) = Ship::convert(file) {
            *ship.borrow_mut() = loaded;
            push_all(&ship.borrow(), ui);
        }
    }
}

// exit_app {{{2
fn exit_app(ui: &MainWindow) {
    ui.hide().unwrap();
}

// field_edited {{{2
fn field_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    pull_then_push(ui, &mut ship.borrow_mut());
}

// torp_units_edited {{{2
/// Convert a torpedo set's units when its units combobox changes.
///
/// Unlike a text edit, this re-expresses the stored diameters/lengths in the
/// new unit system rather than re-parsing the displayed text.
///
fn torp_units_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>, row: usize) {
    gui_map::convert_torp_units(&mut ship.borrow_mut(), ui, row);
}

// mine_units_edited {{{2
/// Convert the mines' units when its units combobox changes.
///
fn mine_units_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    gui_map::convert_mines_units(&mut ship.borrow_mut(), ui);
}

// asw_units_edited {{{2
/// Convert an ASW set's units when its units combobox changes.
///
fn asw_units_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>, row: usize) {
    gui_map::convert_asw_units(&mut ship.borrow_mut(), ui, row);
}

// set_all_units {{{2
/// Set all entry fields to imperial or metric.
///
fn set_all_units(ui: &MainWindow, ship: &Rc<RefCell<Ship>>, which: i32) {

}

// hull_units_edited {{{2
/// Convert the hull's units when its units combobox changes.
///
fn hull_units_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    gui_map::convert_hull_units(&mut ship.borrow_mut(), ui);
}

// draft_edited {{{2
fn draft_edited(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    let mut s = ship.borrow_mut();
    // While the depth is locked, a draft change moves the freeboards so
    // the keel-to-deck height holds steady.
    pull_all(ui, &mut s);
    gui_map::push_depth_locked(&mut s, ui);
    push_derived(&s, ui);
}

// depth_lock_toggled {{{2
fn depth_lock_toggled(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    let mut s = ship.borrow_mut();
    pull_then_push(ui, &mut s);
    gui_map::stash_depth_lock(&s, ui);
}

// set_freeboards {{{2
fn set_freeboards(ui: &MainWindow, ship: &Rc<RefCell<Ship>>, which: i32) {
    let mut s = ship.borrow_mut();
    pull_all(ui, &mut s);
    gui_map::push_freeboard_est(&mut s, ui, which);
    push_derived(&s, ui);
}

// load_ship {{{2
fn load_ship(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    if let Some(file) = pick_file("Sharpie file to load", SHIP_FILE_EXT) {
        if let Ok(loaded) = Ship::load(file) {
            *ship.borrow_mut() = loaded;
            push_all(&ship.borrow(), ui);
        }
    }
}

// save_ship {{{2
fn save_ship(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    let mut s = ship.borrow_mut();
    pull_then_push(ui, &mut s);
    if let Some(file) = save_file_dialog("Sharpie file to save", SHIP_FILE_EXT, &format!("SHIP.{SHIP_FILE_EXT}")) {
        let _ = s.save(file);
    }
}

// save_picture {{{2
//
/// Export the hull side-profile SVG to a file chosen by the user.
///
fn save_picture(ui: &MainWindow, ship: &Rc<RefCell<Ship>>) {
    let mut s = ship.borrow_mut();

    pull_then_push(ui, &mut s);
    let default = if s.name.is_empty() {
        "hull.svg".to_owned()
    } else {
        format!("{}-hull.svg", s.name)
    };

    if let Some(file) = save_file_dialog("Sharpie picture", "svg", &default) {
        let _ = std::fs::write(file, hull_draw::hull_svg(&s.hull, &s.name));
    }
}

// show_about {{{2
fn show_about(_ui: &MainWindow) {
    let about = AboutDialog::new().unwrap();
    let about_weak = about.as_weak();
    about.on_ok_clicked(move || { about_weak.unwrap().hide().unwrap(); });
    let _ = about.run();
}

// toggle_report {{{2
fn toggle_report(ui: &MainWindow) {
    ui.set_report_visible(!ui.get_report_visible());
}

// Run the GUI {{{1
//
pub fn run_gui() -> Result<(), Box<dyn Error>> {
    let ui   = MainWindow::new().unwrap();
    let ship = Rc::new(RefCell::new(Ship::default()));

    gui_map::set_enum_models(&ui);
    push_all(&ship.borrow(), &ui);

    // Register callbacks
    ui.on_clear_ship         ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { clear_ship         (&h.unwrap(), &s); }});
    ui.on_convert_ship       ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { convert_ship       (&h.unwrap(), &s); }});
    ui.on_depth_lock_toggled ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { depth_lock_toggled (&h.unwrap(), &s); }});
    ui.on_draft_edited       ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { draft_edited       (&h.unwrap(), &s); }});
    ui.on_exit_app           ({ let h = ui.as_weak();                       move ||      { exit_app           (&h.unwrap()); }});
    ui.on_field_edited       ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { field_edited       (&h.unwrap(), &s); }});
    ui.on_freeboards_est     ({ let h = ui.as_weak(); let s = ship.clone(); move |which| { set_freeboards     (&h.unwrap(), &s, which); }});
    ui.on_load_ship          ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { load_ship          (&h.unwrap(), &s); }});
    ui.on_save_picture       ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { save_picture       (&h.unwrap(), &s); }});
    ui.on_save_ship          ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { save_ship          (&h.unwrap(), &s); }});
    ui.on_show_about         ({ let h = ui.as_weak();                       move ||      { show_about         (&h.unwrap()); }});
    ui.on_toggle_report      ({ let h = ui.as_weak();                       move ||      { toggle_report      (&h.unwrap()); }});
    ui.on_torp_units_edited  ({ let h = ui.as_weak(); let s = ship.clone(); move |row|   { torp_units_edited  (&h.unwrap(), &s, row as usize); }});
    ui.on_mine_units_edited  ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { mine_units_edited  (&h.unwrap(), &s); }});
    ui.on_asw_units_edited   ({ let h = ui.as_weak(); let s = ship.clone(); move |row|   { asw_units_edited   (&h.unwrap(), &s, row as usize); }});
    ui.on_hull_units_edited  ({ let h = ui.as_weak(); let s = ship.clone(); move ||      { hull_units_edited  (&h.unwrap(), &s); }});
    ui.on_set_all_units      ({ let h = ui.as_weak(); let s = ship.clone(); move |which| { set_all_units      (&h.unwrap(), &s, which); }});

    match ui.run() {
        Ok(_)    => Ok(()),
        Err(err) => Err(err.into()),
    }
}
