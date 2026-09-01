//! Binary-side mapping between the domain [`Ship`] and the Slint UI.
//!
//! Declared only from `main.rs`, keeping the library GUI-free. Each GUI
//! group gets a `push_*` to move values ship -> UI and a `pull_*` to move them
//! UI -> ship; parsing failures leave the corresponding domain value
//! untouched.

use slint::{ModelRc, SharedString, VecModel};

use crate::editor::{depth_lock, freeboard_est};
use crate::calc::hull_draw;

use crate::{HullComputed, HullFields, MainWindow, ShipIdentity};
use crate::calc::{
    BowType,
    Displacement,
    Freeboard,
    Length,
    Measurement,
    Ship,
    SternType,
    UnitType::{Area, LengthLong},
    Units,
    YEAR_MAX,
    YEAR_MIN,
};
use crate::{num, pct};

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
// push_identity {{{2
/// Push ship identity fields into the UI.
///
pub fn push_identity(ship: &Ship, ui: &MainWindow) {
    ui.set_identity(ShipIdentity {
        name: ship.name.clone().into(),
        country: ship.country.clone().into(),
        kind: ship.kind.clone().into(),
        year: ship.year.to_string().into(),
    });
}

// pull_identity {{{2
/// Pull ship identity fields from the UI into the ship.
///
/// The year is only accepted when it is a four-digit number inside the
/// legal range; anything else leaves the stored year untouched.
///
pub fn pull_identity(ui: &MainWindow, ship: &mut Ship) {
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
// push_hull {{{2
/// Push editable hull fields into the UI.
///
/// For each either/or pair the active box shows the stored value and the
/// inactive box shows the derived counterpart (e.g., displacement derived
/// from a given Cb).
///
pub fn push_hull(ship: &Ship, ui: &MainWindow) {
    let h = &ship.hull;
    let u = h.units;

    let (disp_kind, disp_cb, disp_d) = match h.disp {
        Displacement::Cb(v) => (0i32, num!(v, 3),      num!(h.d(), 0)),
        Displacement::D(v)  => (1i32, num!(h.cb(), 3), num!(v, 0)),
    };

    let (len_kind, len_lwl, len_loa) = match h.len {
        Length::Lwl(m) => (0i32, fmt_meas(m, u, 2),       fmt_meas(h.loa(), u, 2)),
        Length::Loa(m) => (1i32, fmt_meas(h.lwl(), u, 2), fmt_meas(m, u, 2)),
    };

    ui.set_hull_fields(HullFields {
        disp_kind,
        disp_cb: disp_cb.into(),
        disp_d:  disp_d.into(),

        len_kind,
        len_lwl: len_lwl.into(),
        len_loa: len_loa.into(),

        b:  fmt_meas(h.b, u, 2).into(),
        bb: fmt_meas(h.bb, u, 2).into(),
        t:  fmt_meas(h.t, u, 2).into(),

        bow_type:       h.bow_type.index() as i32,
        stern_type:     h.stern_type.index() as i32,
        ram_len:        fmt_meas(h.bow_type.ram_len(), u, 2).into(),
        stern_overhang: fmt_meas(h.stern_overhang, u, 2).into(),
        bow_angle:      num!(h.bow_angle, 2).into(),

        fc_len: pct!(h.freeboard.fc_len).into(),
        fd_len: pct!(h.freeboard.fd_len).into(),
        qd_len: pct!(h.freeboard.qd_len).into(),
        ad_len: pct!(h.freeboard.ad_len()).into(),

        fc_fwd: fmt_meas(h.freeboard.fc_fwd, u, 2).into(),
        fc_aft: fmt_meas(h.freeboard.fc_aft, u, 2).into(),

        fd_fwd: fmt_meas(h.freeboard.fd_fwd, u, 2).into(),
        fd_aft: fmt_meas(h.freeboard.fd_aft, u, 2).into(),

        ad_fwd: fmt_meas(h.freeboard.ad_fwd, u, 2).into(),
        ad_aft: fmt_meas(h.freeboard.ad_aft, u, 2).into(),

        qd_fwd: fmt_meas(h.freeboard.qd_fwd, u, 2).into(),
        qd_aft: fmt_meas(h.freeboard.qd_aft, u, 2).into(),

        freeboard: fmt_meas(h.freeboard.average(), u, 2).into(),

        depth_locked: false,
        depth_fc_for: 0.0,
        depth_fc_aft: 0.0,
        depth_fd_for: 0.0,
        depth_fd_aft: 0.0,
        depth_ad_for: 0.0,
        depth_ad_aft: 0.0,
        depth_qd_for: 0.0,
        depth_qd_aft: 0.0,

        engine_year: ship.engine.year.to_string().into(),
    });

    ui.set_hull_computed(HullComputed {
        t_max:     fmt_meas(ship.t_max(), u, 2).into(),
        cb:        h.cb() as f32,
        cb_max:    num!(ship.cb_max(), 3).into(),
        d_max:     num!(ship.d_max(), 0).into(),
        wp_imp:    num!(h.wp().imp(), 0).into(),
        wp_metric: num!(h.wp().metric(), 0).into(),
        ws_imp:    num!(h.ws(), 2).into(),
        ws_metric: num!(Measurement::new(h.ws(), Area, Units::Imperial).metric(), 2).into(),
        len2beam:  format!("{} : 1", num!(h.len2beam(), 2)).into(),
        vn:        num!(h.vn(), 2).into(),
    });
}

// pull_hull {{{2
/// Pull editable hull fields from the UI into the ship.
///
/// Only the box flagged by each either/or pair's kind index is parsed; a
/// failed parse keeps both the prior variant and its value.
///
pub fn pull_hull(ui: &MainWindow, ship: &mut Ship) {
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

    set_frac(&mut h.freeboard.fc_len, &f.fc_len);
    set_frac(&mut h.freeboard.fd_len, &f.fd_len);
    set_frac(&mut h.freeboard.qd_len, &f.qd_len);

    set_meas(&mut h.freeboard.fc_fwd, &f.fc_fwd, h.units);
    set_meas(&mut h.freeboard.fc_aft, &f.fc_aft, h.units);
    set_meas(&mut h.freeboard.fd_fwd, &f.fd_fwd, h.units);
    set_meas(&mut h.freeboard.fd_aft, &f.fd_aft, h.units);
    set_meas(&mut h.freeboard.ad_fwd, &f.ad_fwd, h.units);
    set_meas(&mut h.freeboard.ad_aft, &f.ad_aft, h.units);
    set_meas(&mut h.freeboard.qd_fwd, &f.qd_fwd, h.units);
    set_meas(&mut h.freeboard.qd_aft, &f.qd_aft, h.units);

    let eng_year = f.engine_year.to_string();
    if eng_year.len() == 4 {
        if let Ok(y) = eng_year.parse::<u32>() {
            if (YEAR_MIN..=YEAR_MAX).contains(&y) {
                ship.engine.year = y;
            }
        }
    }
}

// push_hull_derived {{{2
/// Refresh only the derived, read-only hull boxes in the UI.
///
/// Unlike push_hull(), this leaves the box being entered untouched so that
/// partially-typed input is not reformatted under the caret. Only the
/// read-only derived boxes are updated (e.g., LOA from a given LWL and the
/// average freeboard from the deck freeboards).
///
pub fn push_hull_derived(ship: &Ship, ui: &MainWindow) {
    let mut f = ui.get_hull_fields();
    let h = &ship.hull;

    match f.len_kind {
        0 => f.len_loa = fmt_meas(h.loa(), h.units, 2).into(),
        _ => f.len_lwl = fmt_meas(h.lwl(), h.units, 2).into(),
    }
    match f.disp_kind {
        0 => f.disp_d = num!(h.d(), 0).into(),
        _ => f.disp_cb = num!(h.cb(), 3).into(),
    }

    f.ad_len = pct!(h.freeboard.ad_len()).into();
    f.freeboard = fmt_meas(h.freeboard.average(), h.units, 2).into();

    ui.set_hull_fields(f);

    let mut c = ui.get_hull_computed();
    c.cb = h.cb() as f32;
    ui.set_hull_computed(c);
}

// stash_depth_lock {{{2
/// Capture the depth stashes when the hull depth is locked.
///
/// Each stash is the (freeboard + draft) total in imperial feet for one of
/// the eight deck corners; while locked, draft changes are compensated by
/// moving the freeboards so the total stays put (see push_depth_locked).
///
pub fn stash_depth_lock(ship: &Ship, ui: &MainWindow) {
    let mut f = ui.get_hull_fields();
    let depths = depth_lock::stash_depths(ship.hull.t, ship.hull.freeboard.clone());

    if f.depth_locked {
        f.depth_fc_for = depths.fc_fwd.imp() as f32;
        f.depth_fc_aft = depths.fc_aft.imp() as f32;
        f.depth_fd_for = depths.fd_fwd.imp() as f32;
        f.depth_fd_aft = depths.fd_aft.imp() as f32;
        f.depth_ad_for = depths.ad_fwd.imp() as f32;
        f.depth_ad_aft = depths.ad_aft.imp() as f32;
        f.depth_qd_for = depths.qd_fwd.imp() as f32;
        f.depth_qd_aft = depths.qd_aft.imp() as f32;
    }

    ui.set_hull_fields(f);
}

// push_depth_locked {{{2
/// Re-derive the freeboard boxes from the depth stashes after a draft
/// change while the depth is locked.
///
/// Each freeboard becomes (stash - draft), so the keel-to-deck height
/// holds steady. Both the ship and the UI text are updated.
///
pub fn push_depth_locked(ship: &mut Ship, ui: &MainWindow) {
    let mut f = ui.get_hull_fields();
    if !f.depth_locked {
        return;
    }

    let m = |v: f64| Measurement::new(v, LengthLong, Units::Imperial);
    let depths = Freeboard {
        fc_fwd: m(f.depth_fc_for as f64),
        fc_aft: m(f.depth_fc_aft as f64),
        fd_fwd: m(f.depth_fd_for as f64),
        fd_aft: m(f.depth_fd_aft as f64),
        ad_fwd: m(f.depth_ad_for as f64),
        ad_aft: m(f.depth_ad_aft as f64),
        qd_fwd: m(f.depth_qd_for as f64),
        qd_aft: m(f.depth_qd_aft as f64),
        ..Freeboard::default()
    };

    let fb = depth_lock::derive_freeboards(ship.hull.t, &depths);
    copy_heights(&mut ship.hull.freeboard, &fb);

    f.fc_fwd = fmt_meas(fb.fc_fwd, ship.hull.units, 2).into();
    f.fc_aft = fmt_meas(fb.fc_aft, ship.hull.units, 2).into();

    f.fd_fwd = fmt_meas(fb.fd_fwd, ship.hull.units, 2).into();
    f.fd_aft = fmt_meas(fb.fd_aft, ship.hull.units, 2).into();

    f.ad_fwd = fmt_meas(fb.ad_fwd, ship.hull.units, 2).into();
    f.ad_aft = fmt_meas(fb.ad_aft, ship.hull.units, 2).into();

    f.qd_fwd = fmt_meas(fb.qd_fwd, ship.hull.units, 2).into();
    f.qd_aft = fmt_meas(fb.qd_aft, ship.hull.units, 2).into();

    ui.set_hull_fields(f);
}

// push_freeboard_est {{{2
/// Fill the eight freeboard boxes from the "Flush deck"/"Mid break" estimates.
///
/// Mirrors SpringSharp's `btnFlushClick`/`btnBreakClick`, but also writes the
/// values back into the ship so the display, image and report stay in sync.
///
pub fn push_freeboard_est(ship: &mut Ship, ui: &MainWindow, which: i32) {
    let mut f = ui.get_hull_fields();
    let h = &mut ship.hull;

    let est = freeboard_est::freeboard_est(h.lwl(), ship.year, which);
    copy_heights(&mut h.freeboard, &est);

    f.fc_fwd = SharedString::from(fmt_meas(est.fc_fwd, h.units, 2));
    f.fc_aft = SharedString::from(fmt_meas(est.fc_aft, h.units, 2));
    f.fd_fwd = SharedString::from(fmt_meas(est.fd_fwd, h.units, 2));
    f.fd_aft = SharedString::from(fmt_meas(est.fd_aft, h.units, 2));
    f.ad_fwd = SharedString::from(fmt_meas(est.ad_fwd, h.units, 2));
    f.ad_aft = SharedString::from(fmt_meas(est.ad_aft, h.units, 2));
    f.qd_fwd = SharedString::from(fmt_meas(est.qd_fwd, h.units, 2));
    f.qd_aft = SharedString::from(fmt_meas(est.qd_aft, h.units, 2));

    ui.set_hull_fields(f);
}

// push_hull_image {{{1
//
/// Render the hull side profile in memory and push it into the UI.
///
/// The SVG is rasterized directly, without touching the filesystem. A decode
/// error (essentially impossible for the deterministic SVG) leaves the previous
/// image in place.
///
pub fn push_hull_image(ship: &Ship, ui: &MainWindow) {
    if let Ok(img) = slint::Image::load_from_svg_data(
        hull_draw::hull_svg(&ship.hull, &ship.name).as_bytes(),
    ) {
        ui.set_hull_image(img);
    }
}

// Helpers {{{1
// copy_heights {{{2
/// Copy src's eight freeboard height fields into dst, leaving the length
/// fractions untouched.
///
fn copy_heights(dst: &mut Freeboard, src: &Freeboard) {
    dst.fc_fwd = src.fc_fwd;
    dst.fc_aft = src.fc_aft;
    dst.fd_fwd = src.fd_fwd;
    dst.fd_aft = src.fd_aft;
    dst.ad_fwd = src.ad_fwd;
    dst.ad_aft = src.ad_aft;
    dst.qd_fwd = src.qd_fwd;
    dst.qd_aft = src.qd_aft;
}

// fmt_meas {{{2
/// Format a Measurement in the ship's unit system.
///
fn fmt_meas(m: Measurement, units: Units, digits: u32) -> String {
    match units {
        Units::Imperial => num!(m.imp(), digits),
        Units::Metric   => num!(m.metric(), digits),
    }
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
