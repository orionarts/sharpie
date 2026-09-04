//! Binary-side mapping between the domain [`Ship`] and the Slint UI.
//!
//! Declared only from `main.rs`, keeping the library GUI-free. Each GUI
//! group gets a `push_*` to move values ship -> UI and a `pull_*` to move them
//! UI -> ship; parsing failures leave the corresponding domain value
//! untouched.

use slint::{
    Model,
    ModelRc,
    SharedString,
    VecModel,
};

use crate::editor::{armor_default, depth_lock, freeboard_est};
use crate::calc::hull_draw;

use crate::{
    ArmorFields,
    ASWFields,
    ASWDerived,
    BeltFields,
    DeckFields,
    EngineComputed,
    EngineFields,
    HullComputed,
    HullFields,
    MainWindow,
    MineFields,
    MineDerived,
    PerfFields,
    TorpedoFields,
    TorpedoDerived,
    ShipIdentity,
    WeightFields,
};
use crate::calc::{
    ASWType,
    BoilerType,
    BowType,
    BulkheadType,
    DeckType,
    Displacement,
    DriveType,
    Freeboard,
    FuelType,
    Length,
    Measurement,
    MineType,
    Ship,
    SternType,
    TorpedoMountType,
    UnitType,
    UnitType::*,
    Units,
    YEAR_MAX,
    YEAR_MIN,
};
use crate::{num, pct};

// Identity {{{1
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

// Armor {{{1
//
// pull_armor {{{2
/// Pull editable armor fields from the UI into the ship.
///
/// Only the box flagged by each either/or pair's kind index is parsed; a
/// failed parse keeps both the prior variant and its value.
///
pub fn pull_armor(ui: &MainWindow, ship: &mut Ship) {
    let f = ui.get_armor_fields();
    let a = &mut ship.armor;

    a.units = f.units.into();

    set_meas(&mut a.main.thick, &f.main.thick, a.units, LengthSmall);
    set_meas(&mut a.main.len,   &f.main.len, a.units, LengthLong);
    set_meas(&mut a.main.hgt,   &f.main.hgt, a.units, LengthLong);

    set_meas(&mut a.end.thick, &f.end.thick, a.units, LengthSmall);
    set_meas(&mut a.end.len,   &f.end.len, a.units, LengthLong);
    set_meas(&mut a.end.hgt,   &f.end.hgt, a.units, LengthLong);

    set_meas(&mut a.upper.thick, &f.upper.thick, a.units, LengthSmall);
    set_meas(&mut a.upper.len,   &f.upper.len, a.units, LengthLong);
    set_meas(&mut a.upper.hgt,   &f.upper.hgt, a.units, LengthLong);

    if let Some(v) = parse(&f.incline) {
        a.incline = v;
    }

    set_meas(&mut a.bulge.thick, &f.bulge.thick, a.units, LengthSmall);
    set_meas(&mut a.bulge.len,   &f.bulge.len, a.units, LengthLong);
    set_meas(&mut a.bulge.hgt,   &f.bulge.hgt, a.units, LengthLong);

    set_meas(&mut a.bulkhead.thick, &f.bh.thick, a.units, LengthSmall);
    set_meas(&mut a.bulkhead.len,   &f.bh.len, a.units, LengthLong);
    set_meas(&mut a.bulkhead.hgt,   &f.bh.hgt, a.units, LengthLong);
    a.bh_kind = BulkheadType::from_index(f.bh_kind.max(0) as usize);
    set_meas(&mut a.bh_beam, &f.bh_beam, a.units, LengthLong);

    set_meas(&mut a.deck.fc, &f.deck.fc, a.units, LengthSmall);
    set_meas(&mut a.deck.md, &f.deck.md, a.units, LengthSmall);
    set_meas(&mut a.deck.qd, &f.deck.qd, a.units, LengthSmall);
    a.deck.kind = DeckType::from_index(f.deck.kind.max(0) as usize);

    set_meas(&mut a.ct_fwd.thick, &f.ct_fwd, a.units, LengthSmall);
    set_meas(&mut a.ct_aft.thick, &f.ct_aft, a.units, LengthSmall);
}


// push_armor {{{2
/// Push editable armor fields into the UI.
///
/// For each either/or pair the active box shows the stored value and the
/// inactive box shows the derived counterpart (e.g., displacement derived
/// from a given Cb).
///
pub fn push_armor(ship: &Ship, ui: &MainWindow) {
    let a = &ship.armor;
    let u = a.units;

    ui.set_armor_fields(ArmorFields {
        units: u.into(),

        main: BeltFields  { thick: fmt_meas(a.main.thick, u, 2).into(), len: fmt_meas(a.main.len, u, 2).into(), hgt: fmt_meas(a.main.hgt, u, 2).into() },
        end: BeltFields  { thick: fmt_meas(a.end.thick, u, 2).into(), len: fmt_meas(a.end.len, u, 2).into(), hgt: fmt_meas(a.end.hgt, u, 2).into() },
        upper: BeltFields  { thick: fmt_meas(a.upper.thick, u, 2).into(), len: fmt_meas(a.upper.len, u, 2).into(), hgt: fmt_meas(a.upper.hgt, u, 2).into() },
        incline: num!(a.incline, 2).into(),

        bulge: BeltFields  { thick: fmt_meas(a.bulge.thick, u, 2).into(), len: fmt_meas(a.bulge.len, u, 2).into(), hgt: fmt_meas(a.bulge.hgt, u, 2).into() },
        bh: BeltFields  { thick: fmt_meas(a.bulkhead.thick, u, 2).into(), len: fmt_meas(a.bulkhead.len, u, 2).into(), hgt: fmt_meas(a.bulkhead.hgt, u, 2).into() },
        bh_kind: a.bh_kind.index() as i32,
        bh_beam: fmt_meas(a.bh_beam, u, 2).into(),

        deck: DeckFields {
            kind: a.deck.kind.index() as i32,
            fc: fmt_meas(a.deck.fc, u, 2).into(),
            md: fmt_meas(a.deck.md, u, 2).into(),
            qd: fmt_meas(a.deck.qd, u, 2).into() },
        ct_fwd: fmt_meas(a.ct_fwd.thick, u, 2).into(),
        ct_aft: fmt_meas(a.ct_aft.thick, u, 2).into(),
    });

    push_armor_derived(ship, ui);
}

// push_armor_derived {{{2
/// Refresh only the derived, read-only armor boxes in the UI.
///
/// Unlike push_armor(), this leaves the box being entered untouched so that
/// partially-typed input is not reformatted under the caret. Only the
/// read-only derived boxes are updated (e.g., LOA from a given LWL and the
/// average freeboard from the deck freeboards).
///
pub fn push_armor_derived(ship: &Ship, ui: &MainWindow) {
    let s     = ship;
    let b     = s.hull.b;
    let lwl   = s.hull.lwl();
    let cwp   = s.hull.cwp();
    let mut c = ui.get_armor_computed();

    c.main_wgt  = num!(s.armor.main.wgt(lwl.imp(), cwp, b.imp())).into();
    c.end_wgt   = num!(s.armor.end.wgt(lwl.imp(), cwp, b.imp())).into();
    c.upper_wgt = num!(s.armor.upper.wgt(lwl.imp(), cwp, b.imp())).into();
    c.belt_wgt  = num!(
        s.armor.main.wgt(lwl.imp(), cwp, b.imp()) +
        s.armor.end.wgt(lwl.imp(), cwp, b.imp()) +
        s.armor.upper.wgt(lwl.imp(), cwp, b.imp())).into();

    c.bulge_wgt = num!(s.armor.bulge.wgt(lwl.imp(), cwp, b.imp())).into();
    c.bh_wgt    = num!(s.armor.bulkhead.wgt(lwl.imp(), cwp, b.imp())).into();
    c.gun_wgt   = num!(s.wgt_gun_armor()).into();
    c.deck_wgt  = num!(s.deck_wgt()).into();
    c.ct_wgt    = num!(s.ct_wgt()).into();
    c.total_wgt = num!(s.wgt_armor()).into();

    c.coverage = armor_coverage(s);

    ui.set_armor_computed(c);
}

// armor_coverage {{{2
/// Text under the "Default" belt button describing the minimum belt length.
///
/// Mirrors the armour portion of SpringSharp's `changeStatus()`. When the ship
/// is incomplete (composite strength < 0.5) a generic reminder is shown;
/// otherwise the minimum belt length needed to cover machinery and magazines
/// is reported in the armour's unit system.
///
fn armor_coverage(s: &Ship) -> SharedString {
    if s.str_comp() < 0.5 {
        "Default length = distance between forecastle and quarterdeck from Freeboard page".into()
    } else {
        let u = s.armor.units;
        let mut m = Measurement::new(s.vitalspace_length(), LengthLong, Units::Imperial);
        m.set_units(u);
        format!("Minimum main belt length to cover machinery and magazines = {}", fmt_meas(m, u, 2)).into()
    }
}

// push_armor_default {{{2
/// Apply the armor "Default" button's estimated belt dimensions to the ship
/// and refresh the UI.
///
/// Mirrors SpringSharp's `beltButtonClick`, writing the main/end/upper belt
/// and bulkhead lengths and heights. The estimates are expressed in the
/// armor's current unit system so the stored measurements stay consistent.
///
pub fn push_armor_default(ship: &mut Ship, ui: &MainWindow) {
    let est = armor_default::default_belts(ship);
    let u = ship.armor.units;
    let m = |v: f64| {
        let mut x = Measurement::new(v, LengthLong, Units::Imperial);
        x.set_units(u);
        x
    };

    ship.armor.main.len     = m(est.main_len);
    ship.armor.main.hgt     = m(est.main_hgt);
    ship.armor.end.len      = m(est.end_len);
    ship.armor.end.hgt      = m(est.end_hgt);
    ship.armor.upper.len    = m(est.upper_len);
    ship.armor.upper.hgt    = m(est.upper_hgt);
    ship.armor.bulkhead.len = m(est.bulkhead_len);
    ship.armor.bulkhead.hgt = m(est.bulkhead_hgt);

    push_armor(ship, ui);
}

// Hull {{{1
// pull_hull {{{2
/// Pull editable hull fields from the UI into the ship.
///
/// Only the box flagged by each either/or pair's kind index is parsed; a
/// failed parse keeps both the prior variant and its value.
///
pub fn pull_hull(ui: &MainWindow, ship: &mut Ship) {
    let f = ui.get_hull_fields();
    let h = &mut ship.hull;

    h.units = f.units.into();

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

    set_meas(&mut h.b, &f.b, h.units, LengthLong);
    set_meas(&mut h.bb, &f.bb, h.units, LengthLong);
    set_meas(&mut h.t, &f.t, h.units, LengthLong);

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
    set_meas(&mut h.stern_overhang, &f.stern_overhang, h.units, LengthLong);

    if let Some(v) = parse(&f.bow_angle) {
        h.bow_angle = v;
    }

    set_frac(&mut h.freeboard.fc_len, &f.fc_len);
    set_frac(&mut h.freeboard.fd_len, &f.fd_len);
    set_frac(&mut h.freeboard.qd_len, &f.qd_len);

    set_meas(&mut h.freeboard.fc_fwd, &f.fc_fwd, h.units, LengthLong);
    set_meas(&mut h.freeboard.fc_aft, &f.fc_aft, h.units, LengthLong);
    set_meas(&mut h.freeboard.fd_fwd, &f.fd_fwd, h.units, LengthLong);
    set_meas(&mut h.freeboard.fd_aft, &f.fd_aft, h.units, LengthLong);
    set_meas(&mut h.freeboard.ad_fwd, &f.ad_fwd, h.units, LengthLong);
    set_meas(&mut h.freeboard.ad_aft, &f.ad_aft, h.units, LengthLong);
    set_meas(&mut h.freeboard.qd_fwd, &f.qd_fwd, h.units, LengthLong);
    set_meas(&mut h.freeboard.qd_aft, &f.qd_aft, h.units, LengthLong);

    let eng_year = f.engine_year.to_string();
    if eng_year.len() == 4 {
        if let Ok(y) = eng_year.parse::<u32>() {
            if (YEAR_MIN..=YEAR_MAX).contains(&y) {
                ship.engine.year = y;
            }
        }
    }
}

// vital_msg {{{2
/// Text on the Freeboard page describing the vitalspace percentage needed to
/// contain engine and magazine spaces.
///
/// Mirrors the freeboard portion of SpringSharp's `changeStatus()`. When the
/// ship is incomplete (composite strength < 0.5) a generic reminder with the
/// 17.5% average is shown; otherwise the computed vitalspace percentage that
/// sets the belt default to the minimum engine and magazine length is shown.
///
fn vital_msg(s: &Ship) -> SharedString {
    if s.str_comp() < 0.5 {
        "Enter in QD and FC % of Lwl 17.5% for average belt length".into()
    } else {
        let v = s.vitalspace();
        format!("Enter in QD and FC % of Lwl boxes 17.5% for average belt length or {:.2}% to set armour default length to minimum engine and magazine length", v).into()
    }
}

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
        units:     h.units.into(),

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
        vital_msg: vital_msg(ship),
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

// convert_hull_units {{{2
/// Re-express the hull's stored length in a new unit system when its units
/// combobox changes, mirroring convert_mines_units. Only the stored length
/// is converted; the derived counterpart (LOA from LWL or vice-versa) is
/// recomputed by push_hull. The other hull dimensions will follow once their
/// units handling is wired in.
///
pub fn convert_hull_units(ship: &mut Ship, ui: &MainWindow) {
    let f = ui.get_hull_fields();
    let h = &mut ship.hull;
    let new_units: Units = f.units.max(0).into();
    if h.units != new_units {
        h.units = new_units;
    }
    push_hull(ship, ui);
}

// convert_armor_units {{{2
/// Set all armor Measurements to a new unit system when its units combobox
/// changes, mirroring convert_torp_units.
///
/// The combobox presents LengthSmall options, but the Armor struct holds both
/// LengthSmall (thicknesses) and LengthLong (lengths/heights) Measurements,
/// so every one is re-expressed in the selected unit system.
///
pub fn convert_armor_units(ship: &mut Ship, ui: &MainWindow) {
    let f = ui.get_armor_fields();
    let a = &mut ship.armor;
    let new_units: Units = f.units.max(0).into();
    if a.units != new_units {
        a.units = new_units;
        let u = a.units;
        for belt in [&mut a.main, &mut a.end, &mut a.upper,
                     &mut a.bulge, &mut a.bulkhead] {
            belt.thick.set_units(u);
            belt.len.set_units(u);
            belt.hgt.set_units(u);
        }
        a.bh_beam.set_units(u);
        a.deck.fc.set_units(u);
        a.deck.md.set_units(u);
        a.deck.qd.set_units(u);
        a.ct_fwd.thick.set_units(u);
        a.ct_aft.thick.set_units(u);
    }
    push_armor(ship, ui);
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

// Engines {{{1
//
// pull_engine {{{2
/// Pull editable engine fields from the UI into the ship.
///
/// Unparsable input leaves the corresponding domain value untouched. The
/// number of shafts is applied through [`crate::calc::engine::Engine::set_shafts`]
/// so hull parameters that depend on it stay in step, mirroring SpringSharp's
/// `shaftsBoxTextChanged` -> `hull.waterplaneAreaCalc`.
///
pub fn pull_engine(ui: &MainWindow, ship: &mut Ship) {
    let f = ui.get_engine_fields();

    if let Some(v) = parse(&f.vmax) {
        ship.engine.vmax = v.clamp(0.0, 50.0);
    }
    if let Some(v) = parse(&f.vcruise) {
        ship.engine.vcruise = v.clamp(0.0, 50.0);
    }
    if let Some(v) = parse(&f.range) {
        ship.engine.range = v.clamp(0.0, f64::from(u32::MAX)) as u32;
    }
    if let Some(v) = parse(&f.pct_coal) {
        ship.engine.pct_coal = v.clamp(0.0, 100.0);
    }
    if let Some(v) = parse(&f.shafts) {
        ship.engine.set_shafts(v.clamp(1.0, 8.0) as u32, &mut ship.hull);
    }

    let mut fuel = FuelType::empty();
    let mut boiler = BoilerType::empty();
    let mut drive = DriveType::empty();

    if f.fuel_coal     { fuel.insert(FuelType::Coal); }
    if f.fuel_oil      { fuel.insert(FuelType::Oil); }
    if f.fuel_diesel   { fuel.insert(FuelType::Diesel); }
    if f.fuel_petrol   { fuel.insert(FuelType::Gasoline); }
    if f.fuel_battery  { fuel.insert(FuelType::Battery); }

    if f.boiler_simple  { boiler.insert(BoilerType::Simple); }
    if f.boiler_complex { boiler.insert(BoilerType::Complex); }
    if f.boiler_turbine { boiler.insert(BoilerType::Turbine); }

    if f.drive_direct    { drive.insert(DriveType::Direct); }
    if f.drive_geared    { drive.insert(DriveType::Geared); }
    if f.drive_electric  { drive.insert(DriveType::Electric); }
    if f.drive_hydraulic { drive.insert(DriveType::Hydraulic); }

    ship.engine.fuel = fuel;
    ship.engine.boiler = boiler;
    ship.engine.drive = drive;
}

// push_engine {{{2
/// Push editable engine fields from the ship into the UI.
///
pub fn push_engine(ship: &Ship, ui: &MainWindow) {
    ui.set_engine_fields(EngineFields {
        vmax: num!(ship.engine.vmax, 3).into(),
        vmax_value: ship.engine.vmax as f32,
        vcruise: num!(ship.engine.vcruise, 3).into(),
        shafts: ship.engine.shafts().to_string().into(),
        range: ship.engine.range.to_string().into(),
        pct_coal: num!(ship.engine.pct_coal).into(),

        fuel_coal:    ship.engine.fuel.contains(FuelType::Coal),
        fuel_oil:     ship.engine.fuel.contains(FuelType::Oil),
        fuel_diesel:  ship.engine.fuel.contains(FuelType::Diesel),
        fuel_petrol:  ship.engine.fuel.contains(FuelType::Gasoline),
        fuel_battery: ship.engine.fuel.contains(FuelType::Battery),

        boiler_simple:  ship.engine.boiler.contains(BoilerType::Simple),
        boiler_complex: ship.engine.boiler.contains(BoilerType::Complex),
        boiler_turbine: ship.engine.boiler.contains(BoilerType::Turbine),

        drive_direct:    ship.engine.drive.contains(DriveType::Direct),
        drive_geared:    ship.engine.drive.contains(DriveType::Geared),
        drive_electric:  ship.engine.drive.contains(DriveType::Electric),
        drive_hydraulic: ship.engine.drive.contains(DriveType::Hydraulic),
    });
}

// sync_engine_vmax_from_slider {{{2
/// Pull the max speed from the slider into the ship and mirror it into the
/// speed box so both stay in step (mirrors SpringSharp's `speedMaxBarScroll`).
///
pub fn sync_engine_vmax_from_slider(ship: &mut Ship, ui: &MainWindow) {
    ship.engine.vmax = ui.get_engine_fields().vmax_value as f64;
    let mut f = ui.get_engine_fields();
    f.vmax = num!(ship.engine.vmax, 3).into();
    ui.set_engine_fields(f);
}

// push_engine_derived {{{2
/// Push the read-only engine outputs (resistance, power, bunker and weight
/// boxes) from the ship into the UI.
///
/// Mirrors SpringSharp's `resistanceCalc`, `engineCalc`, `bunkerCalc` and
/// `hullWeightCalc` display values. The power-to-wavemaking boxes are stored
/// without a `%` and the suffix is added in the Slint markup, matching the
/// perf-tab convention.
///
pub fn push_engine_derived(ship: &Ship, ui: &MainWindow) {
    ui.set_engine_computed(EngineComputed {
        // Speed & power, max
        frict_max:    num!(ship.rf_max()).into(),
        wave_max:     num!(ship.rw_max()).into(),
        powwave_max:  pct!(ship.pw_max()).into(),
        hp_max:       num!(ship.hp_max().imp()).into(),
        kw_max:       num!(ship.hp_max().metric()).into(),
        // Speed & power, cruise
        frict_cruise: num!(ship.rf_cruise()).into(),
        wave_cruise:  num!(ship.rw_cruise()).into(),
        powwave_cruise: pct!(ship.pw_cruise()).into(),
        hp_cruise:    num!(ship.hp_cruise()).into(),
        kw_cruise:    num!(Measurement::new(ship.hp_cruise(), UnitType::Power, Units::Imperial).metric()).into(),
        // Weights, engine row
        d_engine:     num!(ship.d_engine()).into(),
        wgt_engine:   num!(ship.wgt_engine()).into(),
        bunker_max:   num!(ship.bunker_max()).into(),
        bunker_normal: num!(ship.wgt_bunker()).into(),
        // Weights, weight row
        wgt_load:     num!(ship.wgt_load()).into(),
        wgt_hull:     num!(ship.wgt_hull()).into(),
        d_factor:     num!(ship.d_factor(), 2).into(),
    });
}

// Performance {{{1
//
// pull_perf {{{2
/// Pull editable armor fields from the UI into the ship.
///
/// Only the box flagged by each either/or pair's kind index is parsed; a
/// failed parse keeps both the prior variant and its value.
///
pub fn pull_perf(ui: &MainWindow, ship: &mut Ship) {
    let f = ui.get_perf_fields();

    if let Some(v) = parse(&f.trim) {
        ship.trim = v.clamp(0.0, 100.0) as u8;
    }
}

// sync_trim_from_slider {{{2
/// Pull trim from the slider value into the ship, then mirror it into the
/// text box so both stay in step (mirrors SpringSharp's `trimBarScroll`).
///
pub fn sync_trim_from_slider(ship: &mut Ship, ui: &MainWindow) {
    let f = ui.get_perf_fields();
    ship.trim = f.trim_value.round().clamp(0.0, 100.0) as u8;
    let mut f = ui.get_perf_fields();
    f.trim = num!(ship.trim).into();
    ui.set_perf_fields(f);
    push_perf_derived(ship, ui);
}

// sync_trim_from_box {{{2
/// Pull trim from the text box into the ship, then mirror it onto the slider
/// value so both stay in step (mirrors SpringSharp's `trimBoxTextChanged`).
///
pub fn sync_trim_from_box(ship: &mut Ship, ui: &MainWindow) {
    let f = ui.get_perf_fields();
    if let Some(v) = parse(&f.trim) {
        ship.trim = v.clamp(0.0, 100.0) as u8;
    }
    let mut f = ui.get_perf_fields();
    f.trim_value = ship.trim as f32;
    ui.set_perf_fields(f);
    push_perf_derived(ship, ui);
}


// push_perf {{{2
/// Push editable perf fields into the UI.
///
/// For each either/or pair the active box shows the stored value and the
/// inactive box shows the derived counterpart (e.g., displacement derived
/// from a given Cb).
///
pub fn push_perf(ship: &Ship, ui: &MainWindow) {
    ui.set_perf_fields(PerfFields {
        trim: num!(ship.trim).into(),
        trim_value: ship.trim as f32,
    });

    push_perf_derived(ship, ui);
}

// push_perf_derived {{{2
/// Refresh only the derived, read-only perf boxes in the UI.
///
/// Unlike push_perf(), this leaves the box being entered untouched so that
/// partially-typed input is not reformatted under the caret. Only the
/// read-only derived boxes are updated (e.g., LOA from a given LWL and the
/// average freeboard from the deck freeboards).
///
pub fn push_perf_derived(ship: &Ship, ui: &MainWindow) {
    let mut c = ui.get_perf_computed();

    c.stability                = num!(ship.stability_adj(), 2).into();
    c.recoil                   = num!(ship.recoil(), 2).into();
    c.flotation                = fmt_meas(ship.flotation(), Units::Imperial, 0).into();
    c.steadiness               = num!(ship.steadiness()).into();
    c.metacenter               = fmt_meas(ship.metacenter(), Units::Imperial, 2).into();
    c.seakeeping               = num!(ship.seakeeping(), 2).into();
    c.damage_shell_size_metric = fmt_meas(ship.damage_shell_size(), Units::Metric, 2).into();
    c.damage_shell_size_imp    = fmt_meas(ship.damage_shell_size(), Units::Imperial, 2).into();
    c.damage_shell_num         = num!(ship.damage_shell_num(), 1).into();
    c.damage_torp_num          = num!(ship.damage_torp_num(), 1).into();
    c.hull_room                = pct!(ship.hull_room(), 1).into();
    c.hull_room_quality        = ship.hull_room_quality().into();
    c.deck_room                = pct!(ship.deck_room(), 1).into();
    c.deck_room_quality        = ship.deck_room_quality().into();
    c.d_max                    = num!(ship.d_max()).into();
    c.d_norm                   = num!(ship.hull.d()).into();
    c.d_std                    = num!(ship.d_std()).into();
    c.d_lite                   = num!(ship.d_lite()).into();
    c.wgt_struct               = fmt_meas(ship.wgt_struct(), Units::Imperial, 0).into();
    c.cost_lb                  = num!(ship.cost_lb(), 3).into();
    c.cost_dollar              = num!(ship.cost_dollar(), 3).into();
    c.str_cross                = num!(ship.str_cross(), 2).into();
    c.str_long                 = num!(ship.str_long(), 2).into();
    c.str_comp                 = num!(ship.str_comp(), 2).into();

    c.seakeeping_desc = "".into();
    for s in ship.seakeeping_desc() {
        c.seakeeping_desc.push_str(&s);
        c.seakeeping_desc.push_str("\n");
    }

    ui.set_perf_computed(c);
}


// Weapons {{{1
//
// pull_asw {{{2
pub fn pull_asw(ui: &MainWindow, ship: &mut Ship) {
    let model = ui.get_asw_fields();
    for (i, t) in ship.asw.iter_mut().enumerate() {
        if let Some(row) = model.row_data(i) {
            if let Some(v) = parse(&row.num)    { t.num = v as u32; }
            if let Some(v) = parse(&row.reload) { t.reload = v as u32; }
            t.units = row.units.max(0).into();
            set_meas(&mut t.wgt, &row.wgt, t.units, Weight);
            t.kind = ASWType::from_index(row.kind.max(0) as usize);
            // TODO: FEATURE
            t.year = ship.year;
        }
    }
}

// push_asw {{{2
pub fn push_asw(ship: &Ship, ui: &MainWindow) {
    let model: Vec<ASWFields> = ship.asw.iter().map(|t| {
        let u = t.units;
        ASWFields {
            num:    t.num.to_string().into(),
            reload: t.reload.to_string().into(),
            wgt:   fmt_meas(t.wgt, u, 6).into(),
            units: t.units.into(),
            kind:  t.kind.index() as i32,
        }
    }).collect();

    ui.set_asw_fields(ModelRc::new(VecModel::from(model)));
    push_asw_total_wgt(ship, ui);
}

// push_asw_total_wgt {{{2
/// Refresh only the read-only total weight fields on the ASW tab,
/// leaving the editable fields (and any active caret) untouched.
///
pub fn push_asw_total_wgt(ship: &Ship, ui: &MainWindow) {
    let model: Vec<ASWDerived> = ship.asw.iter().map(|t| {
        ASWDerived { wgt_weaps: num!(t.wgt_weaps(), 3).into() }
    }).collect();

    ui.set_asw_derived(ModelRc::new(VecModel::from(model)));
}

// pull_mines {{{2
pub fn pull_mines(ui: &MainWindow, ship: &mut Ship) {
    let model = ui.get_mine_fields();

    if let Some(row) = model.row_data(0) {
        if let Some(v) = parse(&row.num)    { ship.mines.num = v as u32; }
        if let Some(v) = parse(&row.reload) { ship.mines.reload = v as u32; }
        ship.mines.units = row.units.max(0).into();
        set_meas(&mut ship.mines.wgt,  &row.wgt,  ship.mines.units, Weight);
        ship.mines.kind = MineType::from_index(row.kind.max(0) as usize);
        // TODO: FEATURE
        ship.mines.year = ship.year;
    }
}

// push_mines {{{2
pub fn push_mines(ship: &Ship, ui: &MainWindow) {
    let model: Vec<MineFields> = [
        MineFields {
            num:    ship.mines.num.to_string().into(),
            reload: ship.mines.reload.to_string().into(),
            wgt:    fmt_meas(ship.mines.wgt, ship.mines.units, 6).into(),
            units:  ship.mines.units.into(),
            kind:   ship.mines.kind.index() as i32,
        }
    ].to_vec();

    ui.set_mine_fields(ModelRc::new(VecModel::from(model)));
    push_mine_total_wgt(ship, ui);
}

// push_mine_total_wgt {{{2
pub fn push_mine_total_wgt(ship: &Ship, ui: &MainWindow) {
    let derived: Vec<MineDerived> = [
        MineDerived { wgt_weaps: num!(ship.mines.wgt_weaps(), 3).into() }
    ].to_vec();

    ui.set_mine_derived(ModelRc::new(VecModel::from(derived)));
}

// pull_torpedoes {{{2
pub fn pull_torpedoes(ui: &MainWindow, ship: &mut Ship) {
    let model = ui.get_torp_fields();
    for (i, t) in ship.torps.iter_mut().enumerate() {
        if let Some(row) = model.row_data(i) {
            t.units = row.units.max(0).into();
            if let Some(v) = parse(&row.num)    { t.num = v as u32; }
            if let Some(v) = parse(&row.mounts) { t.mounts = v as u32; }
            set_meas(&mut t.diam, &row.diam, t.units, LengthSmall);
            set_meas(&mut t.len,  &row.len,  t.units, LengthLong);
            t.kind = TorpedoMountType::from_index(row.kind.max(0) as usize);
            // TODO: FEATURE
            t.year = ship.year;
        }
    }
}

// convert_torp_units {{{2
/// Convert a torpedo set's diam and len to a new unit system when its units
/// combobox changes. Re-expresses the stored Measurements in the new units
/// (so the saved ship stays consistent) and refreshes the UI.
///
pub fn convert_torp_units(ship: &mut Ship, ui: &MainWindow, row: i32) {
    let row = row as usize;
    let Some(t) = ship.torps.get_mut(row) else { return };
    let Some(fields) = ui.get_torp_fields().row_data(row) else { return };
    t.units = fields.units.max(0).into();
    t.diam.set_units(t.units);
    t.len.set_units(t.units);
    push_torpedoes(ship, ui);
}

// convert_mines_units {{{2
/// Convert the mines' weight to a new unit system when its units combobox
/// changes, mirroring convert_torp_units.
///
pub fn convert_mines_units(ship: &mut Ship, ui: &MainWindow) {
    let Some(fields) = ui.get_mine_fields().row_data(0) else { return };
    ship.mines.units = fields.units.max(0).into();
    ship.mines.wgt.set_units(ship.mines.units);
    push_mines(ship, ui);
}

// convert_asw_units {{{2
/// Convert one ASW set's weight to a new unit system when its units combobox
/// changes, mirroring convert_torp_units.
///
pub fn convert_asw_units(ship: &mut Ship, ui: &MainWindow, row: i32) {
    let row = row as usize;
    let Some(t) = ship.asw.get_mut(row) else { return };
    let Some(fields) = ui.get_asw_fields().row_data(row) else { return };
    t.units = fields.units.max(0).into();
    t.wgt.set_units(t.units);
    push_asw(ship, ui);
}

// push_torpedoes {{{2
pub fn push_torpedoes(ship: &Ship, ui: &MainWindow) {
    let model: Vec<TorpedoFields> = ship.torps.iter().map(|t| {
        TorpedoFields {
            num:    t.num.to_string().into(),
            mounts: t.mounts.to_string().into(),
            kind:   t.kind.index() as i32,
            diam:   fmt_meas(t.diam, t.units, 2).into(),
            len:    fmt_meas(t.len, t.units, 2).into(),
            units:  t.units.into(),
        }
    }).collect();

    ui.set_torp_fields(ModelRc::new(VecModel::from(model)));
    push_torp_wgt(ship, ui);
}

// push_torp_wgt {{{2
/// Refresh only the read-only weight fields on the torpedoes tab,
/// leaving the editable fields (and any active caret) untouched.
///
pub fn push_torp_wgt(ship: &Ship, ui: &MainWindow) {
    let model: Vec<TorpedoDerived> = ship.torps.iter().map(|t| {
        TorpedoDerived { wgt_weaps: num!(t.wgt_weaps(), 3).into() }
    }).collect();

    ui.set_torp_derived(ModelRc::new(VecModel::from(model)));
}

// pull_weights {{{2
pub fn pull_weights(ui: &MainWindow, ship: &mut Ship) {
    let wgts = ui.get_weight_fields();

    if let Some(v) = parse(&wgts.vital) { ship.wgts.vital = v as u32; }
    if let Some(v) = parse(&wgts.hull)  { ship.wgts.hull  = v as u32; }
    if let Some(v) = parse(&wgts.on)    { ship.wgts.on    = v as u32; }
    if let Some(v) = parse(&wgts.above) { ship.wgts.above = v as u32; }
    if let Some(v) = parse(&wgts.void)  { ship.wgts.void  = v as u32; }
}

// push_weights {{{2
pub fn push_weights(ship: &Ship, ui: &MainWindow) {
    ui.set_weight_fields(WeightFields {
        vital:      ship.wgts.vital.to_string().into(),
        hull:       ship.wgts.hull.to_string().into(),
        on:         ship.wgts.on.to_string().into(),
        above:      ship.wgts.above.to_string().into(),
        void:       ship.wgts.void.to_string().into(),
        hull_space: num!(ship.hull_space(), 4).into(),
        deck_space: num!(ship.deck_space(), 4).into(),
    });
}

// push_weight_derived {{{2
/// Refresh only the read-only hull/deck space fields, leaving the
/// editable weight fields (and any active caret) untouched.
///
pub fn push_weight_derived(ship: &Ship, ui: &MainWindow) {
    let mut w = ui.get_weight_fields();
    w.hull_space = num!(ship.hull_space(), 2).into();
    w.deck_space = num!(ship.deck_space(), 2).into();
    ui.set_weight_fields(w);
}

// Other {{{1
//
// push_hull_image {{{2
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
/// Unparsable input leaves the current value untouched. `ut` supplies the
/// factor for `units` (e.g. `LengthSmall` for a torpedo diameter).
///
fn set_meas(field: &mut Measurement, s: &str, units: Units, ut: UnitType) {
    if let Ok(v) = s.trim().parse::<f64>() {
        *field = Measurement::new(v, ut, units);
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
