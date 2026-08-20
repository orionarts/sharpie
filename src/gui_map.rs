//! Binary-side mapping between the domain [`Ship`] and the Slint UI.
//!
//! Declared only from `main.rs`, keeping the library GUI-free. Each GUI
//! group gets a `*_to_ui` push and an `apply_*` pull; parsing failures
//! leave the corresponding domain value untouched.

use slint::{ModelRc, SharedString, VecModel};

use crate::{HullFields, MainWindow, ShipIdentity};
use sharpie::hull::{BowType, Displacement, Length, SternType};
use sharpie::units::{Measurement, UnitType::LengthLong, Units};
use sharpie::{Ship, YEAR_MAX, YEAR_MIN};

// set_enum_models {{{1
/// Fill dropdown label models from each enum's `.sship` order.
///
pub fn set_enum_models(ui: &MainWindow) {
    ui.set_bow_labels(label_model(BowType::ALL.iter().map(|v| v.label())));
    ui.set_stern_labels(label_model(SternType::ALL.iter().map(|v| v.label())));
}

// label_model {{{2
/// Wrap a list of labels into a Slint string model.
///
fn label_model(labels: impl Iterator<Item = &'static str>) -> ModelRc<SharedString> {
    ModelRc::new(labels.map(SharedString::from).collect::<VecModel<_>>())
}

// Identity {{{1
// identity_to_ui {{{2
/// Push ship identity fields into the UI.
///
pub fn identity_to_ui(ship: &Ship, ui: &MainWindow) {
    ui.set_identity(ShipIdentity {
        name: ship.name.clone().into(),
        country: ship.country.clone().into(),
        kind: ship.kind.clone().into(),
        year: ship.year.to_string().into(),
    });
}

// apply_identity {{{2
/// Pull ship identity fields from the UI into the ship.
///
/// The year is only accepted when it is a four-digit number inside the
/// legal range; anything else leaves the stored year untouched.
///
pub fn apply_identity(ui: &MainWindow, ship: &mut Ship) {
    let id = ui.get_identity();

    ship.name = id.name.to_string();
    ship.country = id.country.to_string();
    ship.kind = id.kind.to_string();

    let year_str = id.year.to_string();
    if year_str.len() == 4 {
        if let Ok(y) = year_str.parse::<u32>() {
            if (YEAR_MIN..=YEAR_MAX).contains(&y) {
                ship.year = y;
            }
        }
    }
}

// Hull inputs {{{1
// hull_to_ui {{{2
/// Push editable hull fields into the UI.
///
/// For each either/or pair the active box shows the stored value and the
/// inactive box shows the derived counterpart (e.g., displacement derived
/// from a given Cb).
///
pub fn hull_to_ui(ship: &Ship, ui: &MainWindow) {
    let h = &ship.hull;
    let u = h.units;

    let (disp_kind, disp_cb, disp_d) = match h.disp {
        Displacement::Cb(v) => (0i32, f3(v), format!("{:.0}", h.d())),
        Displacement::D(v) => (1i32, f3(h.cb()), format!("{:.0}", v)),
    };

    let (len_kind, len_lwl, len_loa) = match h.len {
        Length::Lwl(m) => (0i32, fmt_meas(m, u), fmt_meas(h.loa(), u)),
        Length::Loa(m) => (1i32, fmt_meas(h.lwl(), u), fmt_meas(m, u)),
    };

    ui.set_hull_fields(HullFields {
        disp_kind,
        disp_cb: disp_cb.into(),
        disp_d: disp_d.into(),
        len_kind,
        len_lwl: len_lwl.into(),
        len_loa: len_loa.into(),

        b: fmt_meas(h.b, u).into(),
        bb: fmt_meas(h.bb, u).into(),
        t: fmt_meas(h.t, u).into(),

        bow_type: h.bow_type.index() as i32,
        ram_len: fmt_meas(h.bow_type.ram_len(), u).into(),
        stern_type: h.stern_type.index() as i32,
        stern_overhang: fmt_meas(h.stern_overhang, u).into(),

        fc_len: pct(h.fc_len).into(),
        fd_len: pct(h.fd_len).into(),
        qd_len: pct(h.qd_len).into(),

        fc_fwd: fmt_meas(h.fc_fwd, u).into(),
        fc_aft: fmt_meas(h.fc_aft, u).into(),
        fd_fwd: fmt_meas(h.fd_fwd, u).into(),
        fd_aft: fmt_meas(h.fd_aft, u).into(),
        ad_fwd: fmt_meas(h.ad_fwd, u).into(),
        ad_aft: fmt_meas(h.ad_aft, u).into(),
        qd_fwd: fmt_meas(h.qd_fwd, u).into(),
        qd_aft: fmt_meas(h.qd_aft, u).into(),

        bow_angle: f2(h.bow_angle).into(),
    });
}

// apply_hull {{{2
/// Pull editable hull fields from the UI into the ship.
///
/// Only the box flagged by each either/or pair's kind index is parsed; a
/// failed parse keeps both the prior variant and its value.
///
pub fn apply_hull(ui: &MainWindow, ship: &mut Ship) {
    let f = ui.get_hull_fields();
    let h = &mut ship.hull;

    match f.disp_kind {
        0 => {
            if let Some(v) = parse(&f.disp_cb) {
                h.disp = Displacement::Cb(v);
            }
        },
        _ => {
            if let Some(v) = parse(&f.disp_d) {
                h.disp = Displacement::D(v);
            }
        },
    }

    match f.len_kind {
        0 => {
            if let Some(v) = parse(&f.len_lwl) {
                h.len = Length::Lwl(Measurement::new(v, LengthLong, h.units));
            }
        },
        _ => {
            if let Some(v) = parse(&f.len_loa) {
                h.len = Length::Loa(Measurement::new(v, LengthLong, h.units));
            }
        },
    }

    set_meas(&mut h.b, &f.b, h.units);
    set_meas(&mut h.bb, &f.bb, h.units);
    set_meas(&mut h.t, &f.t, h.units);

    // Rebuild the bow type from the dropdown; protrusion variants reuse
    // the edited length when it parses, otherwise the previous length.
    let probe = match parse(&f.ram_len) {
        Some(v) => Measurement::new(v, LengthLong, h.units),
        None => h.bow_type.ram_len(),
    };
    h.bow_type = match BowType::from_index(f.bow_type.max(0) as usize) {
        BowType::Ram(_) => BowType::Ram(probe),
        BowType::BulbForward(_) => BowType::BulbForward(probe),
        plain => plain,
    };

    h.stern_type = SternType::from_index(f.stern_type.max(0) as usize);
    set_meas(&mut h.stern_overhang, &f.stern_overhang, h.units);

    if let Some(v) = parse(&f.bow_angle) {
        h.bow_angle = v;
    }

    set_frac(&mut h.fc_len, &f.fc_len);
    set_frac(&mut h.fd_len, &f.fd_len);
    set_frac(&mut h.qd_len, &f.qd_len);

    set_meas(&mut h.fc_fwd, &f.fc_fwd, h.units);
    set_meas(&mut h.fc_aft, &f.fc_aft, h.units);
    set_meas(&mut h.fd_fwd, &f.fd_fwd, h.units);
    set_meas(&mut h.fd_aft, &f.fd_aft, h.units);
    set_meas(&mut h.ad_fwd, &f.ad_fwd, h.units);
    set_meas(&mut h.ad_aft, &f.ad_aft, h.units);
    set_meas(&mut h.qd_fwd, &f.qd_fwd, h.units);
    set_meas(&mut h.qd_aft, &f.qd_aft, h.units);
}

// Helpers {{{1
// fmt_meas {{{2
/// Format a Measurement in the ship's unit system.
///
fn fmt_meas(m: Measurement, units: Units) -> String {
    match units {
        Units::Imperial => format!("{:.2}", m.imp()),
        Units::Metric => format!("{:.2}", m.metric()),
    }
}

// pct {{{2
/// Format a deck-length fraction as a percentage.
///
fn pct(frac: f64) -> String {
    format!("{:.1}", frac * 100.0)
}

// f2/f3 {{{3
fn f2(v: f64) -> String {
    format!("{v:.2}")
}

fn f3(v: f64) -> String {
    format!("{v:.3}")
}

// parse {{{2
/// Parse a UI string as a finite number.
///
fn parse(s: &SharedString) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

// set_meas {{{2
/// Overwrite a Measurement from UI text in the ship's unit system.
///
/// Unparsable input leaves the current value untouched.
///
fn set_meas(field: &mut Measurement, s: &str, units: Units) {
    if let Ok(v) = s.trim().parse::<f64>() {
        *field = Measurement::new(v, LengthLong, units);
    }
}

// set_frac {{{2
/// Overwrite a deck-length fraction from UI text entered as percent.
///
/// Unparsable input leaves the current value untouched.
///
fn set_frac(field: &mut f64, s: &str) {
    if let Ok(v) = s.trim().parse::<f64>() {
        *field = v / 100.0;
    }
}
