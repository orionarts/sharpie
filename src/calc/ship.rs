use crate::calc::{
    ASW,
    Armor,
    Battery,
    Belt,
    BoilerType,
    BowType,
    BulkheadType,
    DriveType,
    Engine,
    FuelType,
    GunDistributionType,
    Hull,
    Measurement,
    Mines,
    MiscWgts,
    MountType,
    Torpedoes,
    UnitType::*,
    Units::*,
    YEAR_MIN,
    YEAR_MAX,
};
use crate::{addto, addif, num, pct};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::cell::Cell;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

/// The Ship file version created by this version of sharpie.
pub const SHIP_FILE_VERSION: u32 = 1;

// plural {{{1
/// Return an "s" if num is anything other than 1.
///
pub fn plural(num: u32) -> String {
    match num { 1 => "".to_string(), _ => "s".to_string() }
}


// Version {{{1
/// Holds Ship file version information.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Version {
    version: u32,
}

// Ship {{{1
/// All the parts of a ship.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ship {
    /// Name of ship.
    pub name: String,
    /// Country of ship.
    pub country: String,
    /// Type of ship.
    ///
    /// This is informative only and does not affect any calculations.
    pub kind: String,
    /// Year ship laid down
    pub year: u32,

    /// Balance between stability and seakeeping.
    // TODO
    pub trim: u8,

    /// Hull configuration.
    pub hull: Hull,
    /// Armor configuration.
    pub armor: Armor,
    /// Engine configuration.
    pub engine: Engine,
    /// Gun batteries.
    pub batteries: Vec<Battery>,
    /// Torpedo mounts.
    pub torps: Vec<Torpedoes>,
    /// Mines.
    pub mines: Mines,
    /// ASW gear.
    pub asw: Vec<ASW>,
    /// Miscellaneous weights.
    pub wgts: MiscWgts,

    /// Custom notes
    pub notes: Vec<String>,

    /// Cache engine weight due to circular dependencies
    #[serde(skip)]
    cached_wgt_engine: Cell<Option<f64>>,
}

impl Default for Ship { // {{{2
    fn default() -> Ship {
        let mut ship = Ship {
            name: "".into(),
            country: "".into(),
            kind: "".into(),
            year: YEAR_MAX,

            trim: 50,

            hull: Hull::default(),
            wgts: MiscWgts::default(),
            engine: Engine::default(),
            armor: Armor::default(),
            torps: vec![Torpedoes::default(), Torpedoes::default()],
            mines: Mines::default(),
            asw: vec![ASW::default(), ASW::default()],
            batteries: vec![
                Battery::default(),
                Battery::default(),
                Battery::default(),
                Battery::default(),
                Battery::default(),
            ],

            notes: Vec::new(),
            cached_wgt_engine: Cell::new(None),
        };

        ship.engine.year = ship.year;
        ship.mines.year  = ship.year;

        for b in ship.batteries.iter_mut() { b.year = ship.year; }
        for a in ship.asw.iter_mut()       { a.year = ship.year; }
        for t in ship.torps.iter_mut()     { t.year = ship.year; }

        ship
    }
}

impl Ship { // {{{2
    /// Pounds in a long ton.
    pub(crate) const POUND2TON: f64 = 2240.0;

    // year_adj {{{3
    /// Year adjustment factor for various calculations.
    ///
    pub fn year_adj(year: u32) -> f64 {
             if (YEAR_MIN ..= 1890)
                 .contains(&year) { 1.0 - (1890 - year) as f64 / 66.666664 }
        else if year <= YEAR_MAX  { 1.0 }
        else                      { 0.0 }
    }

    // deck_space {{{3
    /// Relative measure of hull space based on waterplane area, freeboard and
    /// displacement adjusted for above water torpedoes.
    ///
    pub fn deck_space(&self) -> f64 {
        if self.hull.wp().imp() == 0.0 { return 0.0; }

        let mut space = 0.0;
        for w in self.torps.iter() {
            space += w.deck_space(self.hull.b.imp());
        }

        space / self.hull.wp().imp()
    }

    // hull_space {{{3
    /// Proportional measure of weights of engines, guns, magazines,
    /// miscellaneous weights, ships stores, torpedo bulkheads and hull mounted
    /// torpedoes to displacement to estimate the minimum length of the
    /// "vitalspace" needed to contain these relative to a norm of 65% of water
    /// length.
    ///
    pub fn hull_space(&self) -> f64 {
        if self.hull.d() == 0.0 { return 0.0; }

        let mut space = 0.0;
        for w in self.torps.iter() {
            space += w.hull_space();
        }
        space / (self.hull.d() * Hull::FT3_PER_TON_SEA)
    }

    // wgt_bunker {{{3
    /// Convenience function to get bunkerage weight from the engine.
    ///
    pub fn wgt_bunker(&self) -> f64 {
        self.engine.bunker(
            self.hull.d(),
            self.hull.lwl().imp(),
            self.hull.leff(),
            self.hull.cs(),
            self.hull.ws(),
        )
    }

    // wgt_load {{{3
    /// Weight of bunkerage, magazine and stores.
    ///
    pub fn wgt_load(&self) -> f64 {
        self.hull.d() * 0.02 + self.wgt_bunker() + self.wgt_mag()
    }

    // d_lite {{{3
    /// Light Displacement (t): Displacement without bunkerage, magazine or
    /// stores.
    ///
    pub fn d_lite(&self) -> f64 {
        self.hull.d() - self.wgt_load()
    }

    // d_std {{{3
    /// Standard Displacement (t): Standardized displacement per the Washington
    /// and London Naval Treaties. Does not include bunkerage or reserve
    /// feedwater.
    ///
    pub fn d_std(&self) -> f64 {
        self.hull.d() - self.wgt_bunker()
    }

    // d_max {{{3
    /// Maximum Displacement (t): Displacement including full bunker, magazines,
    /// feedwater and stores.
    ///
    pub fn d_max(&self) -> f64 {
        self.hull.d() + 0.8 * self.wgt_bunker()
    }

    // t_max {{{3
    /// Draft at maximum displacement.
    ///
    pub fn t_max(&self) -> Measurement {
        Measurement::new(self.hull.t_calc(self.d_max()), LengthLong, Imperial)
    }

    // cb_max {{{3
    /// Block coefficient at maximum displacement.
    ///
    pub fn cb_max(&self) -> f64 {
        self.hull.cb_calc(self.d_max(), self.t_max().imp())
    }

    // crew_max {{{3
    /// Estimated maximum crew size based on displacement.
    ///
    pub fn crew_max(&self) -> u32 {
        (self.hull.d().powf(0.75) * 0.65) as u32
    }

    // crew_min {{{3
    /// Estimated minimum crew size based on displacement.
    ///
    pub fn crew_min(&self) -> u32 {
        (self.crew_max() as f64 * 0.7692) as u32
    }

    // vitalspace {{{3
    /// Forecastle and Quarterdeck length required
    /// to cover engine and magazine spaces.
    ///
    #[allow(dead_code)]
    pub fn vitalspace(&self) -> f64 {
        (1.0 - 0.65 * self.hull_room()) * 50.0 - 0.01
    }

    // vitalspace_length {{{3
    /// Minimum armor belt length to cover
    /// engine and magazine spaces.
    ///
    #[allow(dead_code)]
    pub fn vitalspace_length(&self) -> f64 {
        self.hull.lwl().imp() * 0.65 * self.hull_room() + 0.01
    }

    // room {{{3
    /// Ratio of the sum of weights of the engine, magazines, ship's stores, torpedo
    /// bulkheads, hull mounted torpedoes and miscellaneous weights to displacement.
    ///
    fn room(&self) -> f64 {
        let divisor = 1.0 - self.hull_space();
        if divisor == 0.0 { return 0.0; }

        (
            self.wgt_mag() +
            self.hull.d() * 0.02 +
            self.wgt_borne() * 6.4 +
            self.wgt_engine() * 3.0 +
            self.wgts.vital as f64 +
            self.wgts.hull as f64
        ) / (self.hull.d() * 0.94) / divisor
    }

    // hull_room {{{3
    /// A numerical measure of the amount of available space within the hull.
    ///
    pub fn hull_room(&self) -> f64 {
        if self.armor.bh_beam.imp() == 0.0 { return 0.0; }

        self.room() *
            if self
                .armor
                .bulkhead
                .wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) > 0.1 {
                self.hull.b.imp() / self.armor.bh_beam.imp()
            } else { 1.0 }
    }

    // deck_room {{{3
    /// A numerical measure of the amount of available deck space.
    ///
    pub fn deck_room(&self) -> f64 {
        if self.crew_min() == 0 { return 0.0; }

        self.hull.wp().imp() /
            Hull::FT3_PER_TON_SEA /
            15.0 * (1.0 - self.deck_space()) /
            self.crew_min() as f64 * self.hull.freeboard.distributed()
    }

    // deck_room_quality {{{3
    /// Return a string describing the deck space.
    ///
    pub fn deck_room_quality(&self) -> String {
        let sp = self.deck_room();

               if sp > 1.2 {
            "Excellent".into()
        } else if sp > 0.9 {
            "Adequate".into()
        } else if sp >= 0.5 {
            "Cramped".into()
        } else {
            "Poor".into()
        }
    }

    // hull_room_quality {{{3
    /// Return a string describing the hull space.
    ///
    pub fn hull_room_quality(&self) -> String {
        let sp = self.hull_room();

               if sp < 5.0/6.0 {
            "Excellent".into()
        } else if sp < 1.1111112 {
            "Adequate".into()
        } else if sp <= 2.0 {
            "Cramped".into()
        } else {
            "Extremely poor".into()
        }
    }

    // cost_dollar {{{3
    /// Cost in millions of US dollars.
    ///
    pub fn cost_dollar(&self) -> f64 {
        ((self.hull.d() - self.wgt_load()) * 0.00014 +
            self.wgt_engine() * 0.00056 + (self.wgt_borne() * 8.0) * 0.00042) *
            if self.year as f64 + 2.0 > 1914.0 {
                1.0 + (self.year as f64 + 1.5 - 1914.0) / 5.5
            } else { 1.0 }
    }

    // cost_lb {{{3
    /// Cost in millions of British pounds
    ///
    pub fn cost_lb(&self) -> f64 {
        self.cost_dollar() / 4.0
    }

    // recoil {{{3
    /// A relative calculation of the ability of the ship to handle her weight of gunfire.
    ///
    pub fn recoil(&self) -> f64 {
        if self.hull.bb.imp() == 0.0 { return 0.0; }

        (
            (self.wgt_broad().imp()/self.hull.d() * self.hull.freeboard.distributed() * self.gun_super_factor() / self.hull.bb.imp()) *

            ( self.hull.d().powf(1.0 / 3.0) / self.hull.bb.imp() * 3.0 ).powf(2.0) * 7.0
        ) /
            if self.stability_adj() > 0.0 {
                self.stability_adj() * ((50.0 - self.steadiness()) / 150.0 + 1.0)
            } else { 1.0 }
    }

    // metacenter {{{3
    /// A measure of vertical equilibrium.
    ///
    pub fn metacenter(&self) -> Measurement {
        Measurement::new(self.hull.b.imp().powf(1.5) * (self.stability_adj() - 0.5) / 0.5 / 200.0, LengthLong, Imperial)
    }

    // seaboat {{{3
    /// Intermediate calculations for seakeeping() and steadiness().
    ///
    fn seaboat(&self) -> f64 {
        if self.hull.d() == 0.0 ||
           self.hull.bb.imp() == 0.0 ||
           self.hull.lwl().imp() == 0.0 ||
           self.rf_max() + self.rw_max() == 0.0
        {
            return 0.0;
        }

        let a = (self.hull.free_cap(self.cap_calc_broadside()) / (2.4 * self.hull.d().powf(0.2))).sqrt() *
            (
                (self.stability() * 5.0 * (self.hull.bb.imp() / self.hull.lwl().imp())).powf(0.2) *
                (self.hull.free_cap(self.cap_calc_broadside()) / self.hull.lwl().imp() * 20.0).sqrt() *
                (
                    self.hull.d() /
                        (
                            self.hull.d() +
                            self.armor.end.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) * 3.0 +
                            self.wgt_hull_plus() / 3.0 +
                            (
                                self.wgt_borne() +
                                self.wgt_gun_armor()
                            ) * self.super_factor_long()
                        )
                )
            ) * 8.0;

        let b = a *
            if (self.hull.t.imp() / self.hull.bb.imp()) < 0.3 {
                (self.hull.t.imp() / self.hull.bb.imp() / 0.3).sqrt()
            } else {
                1.0
            };

        let c = b *
            if (self.rf_max() / (self.rf_max() + self.rw_max())) < 0.55 &&
                self.engine.vmax > 0.0
            {
                (self.rf_max() / (self.rf_max() + self.rw_max())).powf(2.0)
            } else {
                0.3025
            };

        f64::min(c, 2.0)
    }

    // seakeeping {{{3
    /// The seakeeping ability of the ship.
    ///
    pub fn seakeeping(&self) -> f64 {
        self.seaboat() * f64::min(self.steadiness(), 50.0) / 50.0
    }

    // tender_warn {{{3
    /// If ship has an excessive risk of capsizing.
    ///
    fn tender_warn(&self) -> bool {
        self.stability_adj() <= 0.995
    }

    // capsize_warn {{{3
    /// If ship will capsize.
    ///
    fn capsize_warn(&self) -> bool {
        self.metacenter().imp() <= 0.0
    }

    // hull_strained {{{3
    /// If hull will be subject to strain in the open sea.
    ///
    fn hull_strained(&self) -> bool {
        self.str_comp() >= 0.5
            && self.str_comp() < 0.885
            && (self.engine.vmax < 24.0 || self.hull.d() > 4000.0)
    }

    // bh_beam_too_wide {{{3
    /// If bulkhead beam is too wide
    ///
    fn bh_beam_too_wide(&self) -> bool {
        match self.armor.bh_kind {
            BulkheadType::Additional => self.armor.bh_beam.imp() >= (self.hull.b.imp() - 6.0),
            BulkheadType::Strengthened   => self.armor.bh_beam.imp() >   self.hull.b.imp(),
        }
    }

    // is_steady {{{3
    /// If ship is a steady gun platform.
    ///
    fn is_steady(&self) -> bool {
        self.steadiness() >= 69.5
    }

    // is_unsteady {{{3
    /// If ship is not a steady gun platform.
    ///
    fn is_unsteady(&self) -> bool {
        self.steadiness() < 30.0
    }

    // type_sea {{{3
    /// Convert seakeeping() value into SeaType.
    ///
    fn type_sea(&self) -> SeaType {
               if self.seakeeping() < 0.7 {
            SeaType::BadSea
        } else if self.seakeeping() < 0.995 {
            SeaType::PoorSea
        } else if self.seakeeping() >= 1.5 {
            SeaType::FineSea
        } else if self.seakeeping() >= 1.2 {
            SeaType::GoodSea
        } else {
            SeaType::Error
        }
    }

    // seakeeping desc {{{3
    /// Return a string describing risk of capsizing,
    /// hull strain, steadiness and seaworthiness.
    ///
    pub fn seakeeping_desc(&self) -> Vec<String> {
        let mut s: Vec<String> = Vec::new();

        if self.is_steady() {
            s.push("Ship has slow, easy roll, a good, steady gun platform".into());
        } else if self.is_unsteady() {
            s.push("Ship has quick, lively roll, not a steady gun platform".into());
        }

        let sea = match self.type_sea() {
            SeaType::BadSea  => "Caution: Lacks seaworthiness - very limited seakeeping ability".into(),
            SeaType::PoorSea => "Poor seaboat, wet and uncomfortable, reduced performance in heavy weather".into(),
            SeaType::GoodSea => "Good seaboat, rides out heavy weather easily".into(),
            SeaType::FineSea => format!("Excellent seaboat, comfortable, {}",
                    if self.wgt_guns() > 0.0 {
                        "can fire her guns in the heaviest weather"
                    } else {
                        "rides out heavy weather easily"
                    }),
            SeaType::Error   => "Invalid SeaType".into(),
        };

        s.push(sea);

        s
    }

    // roll_period {{{3
    /// Roll period of the ship.
    ///
    pub fn roll_period(&self) -> f64 {
        if self.metacenter().imp() == 0.0 { return 0.0; }
        0.42 * self.hull.bb.imp() / self.metacenter().imp().sqrt()
    }

    // steadiness {{{3
    /// Dynamic hull steadiness in open sea based
    /// on trim adjustment and seakeeping value.
    ///
    pub fn steadiness(&self) -> f64 {
        f64::min(self.trim as f64 * self.seaboat(), 100.0)
    }

    // stability {{{3
    /// Inherent stability of the ship before applying
    /// the trim adjustment.
    ///
    fn stability(&self) -> f64 {
        if self.hull.t.imp() == 0.0 || self.hull.len2beam() == 0.0 { return 0.0; }

        let a =
            (self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())) * 5.0 +
            (self.wgt_borne() + self.wgt_gun_armor()) * (2.0 * self.gun_super_factor() - 1.0) * 4.0 +
            self.wgts.hull as f64 * 2.0 +
            self.wgts.on as f64 * 3.0 +
            self.wgts.above as f64 * 4.0 +
            self.armor.upper.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp()) * 2.0 +
            self.armor.main.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp()) +
            self.armor.end.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp()) +
            self.deck_wgt() +
            (self.wgt_hull_plus() + self.wgt_guns() + self.wgt_gun_mounts() - self.wgt_borne()) * 1.5 * self.hull.freeboard.average().imp() / self.hull.t.imp();

        let b = a +
            if self.deck_room() < 1.0 {
                (self.wgt_engine() + self.wgts.vital as f64 + self.wgts.void as f64) * (1.0 - self.deck_room().powf(2.0))
            } else { 0.0 };

        if b > 0.0 {
            ((self.hull.d() * (self.hull.bb.imp() / self.hull.t.imp()) / b) * 0.5).sqrt() *
            (8.76755 / self.hull.len2beam()).powf(0.25)
        } else {
            b
        }
    }

    // stability_adj {{{3
    /// A measure of the effect of vertical weights
    /// on the stability of the ship.
    ///
    pub fn stability_adj(&self) -> f64 {
        self.stability() * ((50.0 - self.trim as f64) / 150.0 + 1.0)
    }

    // d_factor {{{3
    /// Adjustment factor to reduce engine weight in a highly
    /// stressed ship of less than 5,000 tons.
    ///
    pub fn d_factor(&self) -> f64 {
        let divisor =
                self.engine.d_engine(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws()) +
                    8.0 * self.wgt_borne() + self.wgt_armor() + self.wgts.wgt() as f64;
        if divisor == 0.0 { return 0.0; }

        f64::min( self.hull.d() / divisor, 10.0)
    }

    // cap_calc_broadside {{{3
    /// Return true if the ship has any below deck broadside guns.
    ///
    pub fn cap_calc_broadside(&self) -> bool {
        for b in self.batteries.iter() {
            if b.broad_and_below() { return true; }
        }

        false
    }

    // flotation {{{3
    /// Estimate of the pounds of non-critical shell
    /// hits required to sink or destroy the ship.
    ///
    pub fn flotation(&self) -> Measurement {
        if self.room() == 0.0 { return Measurement::new(0.0, Weight, Imperial); }

        let a =
            if self.cap_calc_broadside() {
                self.hull.free_cap(self.cap_calc_broadside())
            } else {
                self.hull.freeboard.distributed()
            };

        let b = (a * self.hull.wp().imp() / Hull::FT3_PER_TON_SEA + self.hull.d()) / 2.0;

        let c = b * self.stability_adj().powf(
            if self.stability_adj() > 1.0 { 0.5 } else { 4.0 }
            );

        let d = c * if self.str_comp() < 1.0 { self.str_comp() } else { 1.0 };

        let e = d / self.room().powf(if self.room() > 1.0 { 2.0 } else { 1.0 });

        Measurement::new(f64::max(e * Self::year_adj(self.year), 0.0), Weight, Imperial)
    }

    // str_cross {{{3
    /// Cross-sectional strength.
    ///
    pub fn str_cross(&self) -> f64 {
        let mut concentration: f64 = 1.0;

        if self.wgt_broad().imp() > 0.0 {
            concentration = 1.0 + self.gun_concentration();
        }

        let a = f64::sqrt(self.hull.bb.imp() * (self.hull.t.imp() + self.hull.freeboard.distributed()));
        let b = (self.hull.d() + ((self.wgt_broad().imp() + self.wgt_borne() + self.wgt_gun_armor() + self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())) * (concentration * self.gun_super_factor()) + f64::max(self.hp_max().imp(), 0.0) / 100.0)) / self.hull.d();

        if a == 0.0 || b == 0.0 { return 0.0; }

        let mut str_cross = self.wgt_struct().imp() / a / b * 0.6;

        if self.year < 1900 {
            str_cross *= 1.0 - (1900.0 - self.year as f64) / 100.0;
        }

        str_cross
    }

    // str_long {{{3
    /// Longitudinal strength.
    ///
    pub fn str_long(&self) -> f64 {
        let divisor = self.hull.t.imp() + self.hull.free_cap(self.cap_calc_broadside());
        if divisor == 0.0 { return 0.0; }

        let a = (self.hull.lwl().imp() / divisor).powf(2.0) *
                (
                    self.hull.d() +
                    self.armor.end.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) *
                    3.0 + (
                        self.wgt_borne() +
                        self.wgt_gun_armor()
                        ) * self.super_factor_long() * 2.0
                );
        if a == 0.0 { return 0.0; }

        (
            self.wgt_hull_plus() + match self.armor.bh_kind {
                BulkheadType::Additional =>
                    self.armor.bulkhead.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()),
                BulkheadType::Strengthened => 0.0,
            }
        ) / a * 850.0 * if self.year < 1900 { 1 - (1900 - self.year) / 100 } else { 1 } as f64
    }

    // str_comp {{{3
    /// Composite strength.
    ///
    pub fn str_comp(&self) -> f64 {
        if self.str_long() == 0.0 || self.str_cross() == 0.0 { return 0.0; }

        if self.str_cross() > self.str_long() {
            self.str_long() * (self.str_cross() / self.str_long()).powf(0.25)
        } else {
            self.str_cross() * (self.str_long() / self.str_cross()).powf(0.1)
        }
    }

    // gun_concentration {{{3
    /// XXX: I do not know what this does.
    ///
    fn gun_concentration(&self) -> f64 {
        let mut concentration = 0.0;
        for b in self.batteries.iter() {
            concentration += b.concentration(self.wgt_broad().imp());
        }
        concentration
    }

    // damage_shell_size {{{3
    /// Size of shells used to calculate flotation().
    ///
    pub fn damage_shell_size(&self) -> Measurement {
        if self.batteries[0].diam.imp() > 0.0 {
            self.batteries[0].diam
        } else {
            Measurement::new(6.0, LengthSmall, Imperial)
        }
    }

    // damage_shell_num {{{3
    /// Number of non-critical shell hits of the same caliber as the
    /// main battery or 6" shells if the ship has no main battery.
    ///
    pub fn damage_shell_num(&self) -> f64 {
        if Self::year_adj(self.year) == 0.0 { return 0.0; }

        self.flotation().imp() / (
            self.damage_shell_size().imp().powf(3.0) /
            2.0 * Self::year_adj(self.year)
            )
    }

    // damage_torp_size {{{3
    /// Size of torpedoes used when displaying the number of
    /// torpedo hits required to sink the ship.
    ///
    pub fn damage_torp_size(&self) -> Measurement {
        let size = if self.torps[0].wgt_weaps() > 0.0 {
            self.torps[0].diam.imp()
        } else {
            20.0
        };

        Measurement::new(size, LengthSmall, Imperial)
    }

    // damage_torp_num {{{3
    /// Number of non-critical torpedo hits required to sink the ship.
    ///
    pub fn damage_torp_num(&self) -> f64 {
        if self.hull.lwl().imp() == 0.0 ||
           self.hull.t.imp() == 0.0 ||
           (self.hull.t.imp() + self.hull.t.imp()) == 0.0 ||
           self.room() == 0.0 ||
           self.torps[0].num == 0 ||
           self.torps[0].wgt_weaps() == 0.0
        { return 0.0; }

        (
            (
                (
                    (self.flotation().imp() / 10_000.0).powf(1.0/3.0) +
                    (self.hull.bb.imp() / 75.0).powf(2.0) +
                    (
                        (self.armor.bulkhead.thick.imp() / 2.0 * self.armor.bulkhead.len.imp() / self.hull.lwl().imp()) /
                        0.65 * self.armor.bulkhead.hgt.imp() / self.hull.t.imp()
                    ).powf(1.0/3.0) *
                    self.flotation().imp() / 35_000.0 * self.hull.bb.imp() / 50.0
                ) / self.room() * self.hull.lwl().imp() / (self.hull.lwl().imp() + self.hull.bb.imp())
            ) * if self.stability_adj() < 1.0 {
                    self.stability_adj().powf(4.0)
                } else {
                    1.0
                } * (1.0 - self.hull_space())
        ) * if self.torps[0].wgt_weaps() > 0.0 {
                1.313 / (self.torps[0].wgt_weaps() / self.torps[0].num as f64)
            } else {
                1.0
            }
    }

    // wgt_engine {{{3
    /// Weight of the engine, adjusted by the displacement factor (d_factor()).
    ///
    // There is a circular dependency involving engine weight when DeckType::BoxOverMachinery or
    // DeckType::BoxOverBoth is used:
    //
    // wgt_engine() -> d_factor() -> wgt_armor() -> deck_wgt() -> wgt_engine()
    //
    // Fortunately, wgt_engine() an iterative calculation converges to a stable value. Once the
    // value converges it is cached so future calls to wgt_engine() can retrieve the cached value
    //
    // The cached value is **required** because of the call to d_factor() **inside** wgt_engine().
    // Without caching a value, the wgt_engine() -> d_factor() chain would eventually overflow the
    // stack
    //
    // Solving this problem revealed a bug in SpringSharp as you have to "force" the GUI to do the
    // iterative calculations. If you don't, it will still report values but they will be incorrect
    //
    pub fn wgt_engine(&self) -> f64 {
        if let Some(wgt) = self.cached_wgt_engine.get() { return wgt; }

        const TOLERANCE: f64 = 0.05;
        const MAX_ITER: usize = 50;

        let mut prev = 0.0;

        for _ in 0..MAX_ITER {
            self.cached_wgt_engine.set(Some(prev));

            let p =
                if (self.hull.d() < 5000.0) && (self.hull.d() >= 600.0) && (self.d_factor() < 1.0) {
                    1.0 - self.hull.d() / 5000.0
                } else if (self.hull.d() < 600.0) && (self.d_factor() < 1.0) {
                    0.88
                } else {
                    0.0
                };

            let new = (self.engine.d_engine(
                self.hull.d(),
                self.hull.lwl().imp(),
                self.hull.leff(),
                self.hull.cs(),
                self.hull.ws(),
            ) / 2.0)
                * self.d_factor().powf(p);

            if (new - prev).abs() < TOLERANCE {
                self.cached_wgt_engine.set(Some(prev));
                return new;
            }
            prev = new;
        }

        self.cached_wgt_engine.set(Some(prev));
        prev
    }

    // wgt_struct {{{3
    /// Weight per square feet of hull.
    ///
    pub fn wgt_struct(&self) -> Measurement {
        let divisor = self.hull.ws() +
                2.0 * self.hull.lwl().imp() * self.hull.free_cap(self.cap_calc_broadside()) +
                self.hull.wp().imp();
        if divisor == 0.0 { return Measurement::new(0.0, WeightPerArea, Imperial); }

        Measurement::new(
            (
                self.wgt_hull_plus() +
                match self.armor.bh_kind {
                    BulkheadType::Additional =>
                        self.armor.bulkhead.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()),
                    BulkheadType::Strengthened => 0.0,
                }
            ) * Self::POUND2TON / divisor,
            WeightPerArea, Imperial
        )
    }

    // wgt_hull {{{3
    /// Weight of the hull.
    ///
    pub fn wgt_hull(&self) -> f64 {
        self.hull.d() -
            self.wgt_guns() -
            self.wgt_gun_mounts() -
            self.wgt_weaps() -
            self.wgt_armor() -
            self.wgt_engine() -
            self.wgt_load() -
            self.wgts.wgt() as f64
    }

    // wgt_hull_plus {{{3
    /// Weight of the hull plus weight of guns and mounts
    /// (excluding wgt_borne()).
    ///
    fn wgt_hull_plus(&self) -> f64 {
        self.wgt_hull() +
        self.wgt_guns() +
        self.wgt_gun_mounts() -
        self.wgt_borne()
    }

    // wgt_borne {{{3
    /// Sum of gun barrel and mount weights.
    ///
    fn wgt_borne(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.gun_wgt() * b.mount_kind.wgt_adj();
        }
        wgt * 2.0
    }

    // wgt_weaps {{{3
    /// Weight of torpedoes, mines and ASW weapons
    ///
    fn wgt_weaps(&self) -> f64 {
        let mut wgt = 0.0;
        for w in self.torps.iter() { wgt += w.wgt(); }
        for w in self.asw.iter()   { wgt += w.wgt(); }
        wgt += self.mines.wgt();

        wgt
    }

    // wgt_guns {{{3
    /// Weight of guns (excluding mounts).
    ///
    pub fn wgt_guns(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.gun_wgt();
        }
        wgt
    }

    // wgt_gun_mounts {{{3
    /// Weight of gun mounts.
    ///
    fn wgt_gun_mounts(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.mount_wgt();
        }
        wgt
    }

    // wgt_gun_armor {{{3
    /// Weight of gun mount armor.
    ///
    pub fn wgt_gun_armor(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.armor_wgt(self.hull.clone());
        }
        wgt
    }

    // wgt_mag {{{3
    /// Weight of the ship's magazines.
    ///
    pub fn wgt_mag(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.mag_wgt();
        }
        wgt
    }

    // wgt_broad {{{3
    /// Sum of the broadside weights of all batteries.
    ///
    pub fn wgt_broad(&self) -> Measurement {
        let mut broad = 0.0;
        for b in self.batteries.iter() {
            broad += b.broadside_wgt();
        }
        Measurement::new(broad, Weight, Imperial)
    }

    // wgt_armor {{{3
    /// Weight of ship and battery armor.
    ///
    pub fn wgt_armor(&self) -> f64 {
        self.armor.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine()) + self.wgt_gun_armor()
    }

    // gun_wtf {{{3
    /// XXX: I do not know what this does.
    ///
    fn gun_wtf(&self) -> f64 {
        let mut wtf = 0.0;
        for b in self.batteries.iter() {
            if b.diam.imp() == 0.0 { continue; }
            wtf += (
                b.gun_wgt() +
                b.mount_wgt() +
                b.armor_wgt(self.hull.clone())
             ) *
                b.super_(self.hull.clone()) *
                b.mount_kind.wgt_adj();
        }
        wtf
    }

    // gun_super_factor {{{3
    /// XXX: I do not know what this does.
    ///
    fn gun_super_factor(&self) -> f64 {
        if self.wgt_gun_armor() + self.wgt_guns() + self.wgt_gun_mounts() == 0.0 { return 0.0; }
        self.gun_wtf() / (self.wgt_gun_armor() + self.wgt_guns() + self.wgt_gun_mounts())
    }

    // super_factor_long {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn super_factor_long(&self) -> f64 {
        let a = self.hull_room() *
            if (
                    self.batteries[0].groups[0].distribution == GunDistributionType::CenterlineEven ||
                    self.batteries[0].groups[0].distribution == GunDistributionType::SidesEven ||
                    self.batteries[0].groups[1].distribution == GunDistributionType::CenterlineEven ||
                    self.batteries[0].groups[1].distribution == GunDistributionType::SidesEven
                ) && (
                    self.batteries[0].mount_num == 3 ||
                    self.batteries[0].mount_num == 4
                )
            {
                self.gun_super_factor()
            } else {
                1.0
            };
        a *
            if (
                    self.batteries[0].groups[0].num_mounts() > 0 &&
                    self.batteries[0].groups[1].num_mounts() == 0 &&
                    self.batteries[0].groups[0].distribution.super_factor_long()
                ) || (
                    self.batteries[0].groups[1].num_mounts() > 0 &&
                    self.batteries[0].groups[0].num_mounts() == 0 &&
                    self.batteries[0].groups[1].distribution.super_factor_long()
                ) || (
                    self.batteries[0].groups[0].num_mounts() > 0 &&
                    self.batteries[0].groups[1].num_mounts() > 0 &&
                    (self.batteries[0].groups[0].distribution.g1_gun_position(self.hull.freeboard.fd_len, self.hull.freeboard.ad_len()) -
                     self.batteries[0].groups[1].distribution.g2_gun_position(self.hull.freeboard.fd_len, self.hull.freeboard.ad_len())).abs() < 0.2
                )
            {
                0.8 * self.gun_super_factor()
            } else {
                2.0 * self.gun_super_factor() - 1.0
            }
    }

    // percent_calc {{{3
    /// Return the ratio of a value to the displacement as a percentage.
    ///
    fn percent_calc(&self, portion: f64) -> String {
        format!("{} tons, {:.1} %", num!(portion),
            if self.hull.d() > 0.0 {
                (portion / self.hull.d()) * 100.0
            } else {
                0.0
            }
        )
    }

    // convert {{{3
    /// Load a ship from a SpringSharp 3 file and output a sharpie ship
    ///
    pub fn convert(p: String) -> Result<Ship, Box<dyn Error>> {
        let mut ship = Ship::default();

        let f = File::open(p)?;
        let reader = BufReader::new(f);
        let mut lines = reader.lines().map(|l| l.unwrap());

        let line = lines.next().unwrap();
        if line.contains("SpringSharp Version 3.0") {
            // Supported format: no special handling required
        } else if line.contains("SpringSharp") {
            Err("SpringSharp file too old")?;
        } else {
            Err("Unknown file format")?;
        }

        ship.name       = lines.next().unwrap();
        ship.country    = lines.next().unwrap();
        ship.kind       = lines.next().unwrap();
        ship.hull.units = lines.next().unwrap().into();

        for b in ship.batteries.iter_mut() { b.units = lines.next().unwrap().into(); }
        ship.torps[0].units = lines.next().unwrap().into();
        ship.armor.units    = lines.next().unwrap().into();

        ship.year = lines.next().unwrap().parse()?;

        ship.wgts.vital = lines.next().unwrap().parse()?;

        ship.hull.set_lwl(lines.next().unwrap().parse()?, ship.hull.units);
        ship.hull.b  = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.t  = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.stern_type = lines.next().unwrap().into();
        ship.hull.set_cb(lines.next().unwrap().parse()?);

        ship.hull.freeboard.qd_aft         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);

        ship.hull.stern_overhang = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);

        ship.hull.freeboard.qd_len         = lines.next().unwrap().parse()?;
        ship.hull.freeboard.qd_len /= 100.0; // convert from % to decimal
        ship.hull.freeboard.qd_fwd         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.ad_aft         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.fd_len         = lines.next().unwrap().parse()?;
        ship.hull.freeboard.fd_len /= 100.0; // convert from % to decimal
        ship.hull.freeboard.ad_fwd         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.fd_aft         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.fc_len         = lines.next().unwrap().parse()?;
        ship.hull.freeboard.fc_len /= 100.0; // convert from % to decimal
        ship.hull.freeboard.fd_fwd         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.fc_aft         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.freeboard.fc_fwd         = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.hull.bow_angle      = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() {
            b.num             = lines.next().unwrap().parse()?;
            let diam_val: f64 = lines.next().unwrap().parse()?;
            b.diam            = Measurement::new(diam_val, LengthSmall, b.units);
            b.kind            = lines.next().unwrap().into();
            b.groups[0].above = lines.next().unwrap().parse()?;
            b.groups[0].below = lines.next().unwrap().parse()?;

            // Have to remove the commas from the string or it fails
            // to convert to a float
            b.set_shell_wgt( lines.next().unwrap().replace(",", "").parse()?, Imperial );
        }

        ship.batteries[0].shells                 = lines.next().unwrap().parse()?;
        ship.batteries[0].mount_num              = lines.next().unwrap().parse()?;
        ship.batteries[0].mount_kind             = lines.next().unwrap().into();
        ship.batteries[0].groups[0].distribution = lines.next().unwrap().into();

        ship.batteries[1].mount_num              = lines.next().unwrap().parse()?;
        ship.batteries[1].mount_kind             = lines.next().unwrap().into();
        ship.batteries[1].groups[0].distribution = lines.next().unwrap().into();

        ship.batteries[2].mount_num              = lines.next().unwrap().parse()?;
        ship.batteries[2].mount_kind             = lines.next().unwrap().into();
        ship.batteries[2].groups[0].distribution = lines.next().unwrap().into();

        ship.batteries[3].mount_num              = lines.next().unwrap().parse()?;
        ship.batteries[3].mount_kind             = lines.next().unwrap().into();
        ship.batteries[3].groups[0].distribution = lines.next().unwrap().into();

        ship.batteries[4].mount_num              = lines.next().unwrap().parse()?;
        ship.batteries[4].mount_kind             = lines.next().unwrap().into();
        ship.batteries[4].groups[0].distribution = lines.next().unwrap().into();

        ship.torps[0].num  = lines.next().unwrap().parse()?;
        ship.torps[1].num  = lines.next().unwrap().parse()?;
        let diam_val: f64  = lines.next().unwrap().parse()?;
        ship.torps[0].diam = Measurement::new(diam_val, LengthSmall, ship.torps[0].units);

        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.main.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.main.len   = Measurement::new(val, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.main.hgt   = Measurement::new(val, LengthLong, ship.armor.units);

        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.end.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.end.len   = Measurement::new(val, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.end.hgt   = Measurement::new(val, LengthLong, ship.armor.units);

        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.upper.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.upper.len   = Measurement::new(val, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.upper.hgt   = Measurement::new(val, LengthLong, ship.armor.units);

        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulkhead.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulkhead.len   = Measurement::new(val, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulkhead.hgt   = Measurement::new(val, LengthLong, ship.armor.units);

        for b in ship.batteries.iter_mut() {
            let val: f64 = lines.next().unwrap().parse()?;
            b.armor_face = Measurement::new(val, LengthSmall, ship.armor.units);
            let val: f64 = lines.next().unwrap().parse()?;
            b.armor_back = Measurement::new(val, LengthSmall, ship.armor.units);
            let val: f64 = lines.next().unwrap().parse()?;
            b.armor_barb = Measurement::new(val, LengthSmall, ship.armor.units);
        }

        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.deck.md = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.ct_fwd.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        ship.engine.vmax        = lines.next().unwrap().parse()?;
        ship.engine.vcruise     = lines.next().unwrap().parse()?;
        ship.engine.range       = lines.next().unwrap().parse()?;
        ship.engine.set_shafts(lines.next().unwrap().parse()?, &mut ship.hull);
        ship.engine.pct_coal    = lines.next().unwrap().parse()?;
        ship.engine.pct_coal /= 100.0; // convert from % to decimal

        ship.engine.fuel = FuelType::empty();
        if lines.next().unwrap().as_str() == "True" { ship.engine.fuel.toggle(FuelType::Coal) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.fuel.toggle(FuelType::Oil) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.fuel.toggle(FuelType::Diesel) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.fuel.toggle(FuelType::Gasoline) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.fuel.toggle(FuelType::Battery) };

        ship.engine.boiler = BoilerType::empty();
        if lines.next().unwrap().as_str() == "True" { ship.engine.boiler.toggle(BoilerType::Simple) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.boiler.toggle(BoilerType::Complex) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.boiler.toggle(BoilerType::Turbine) };

        ship.engine.drive = DriveType::empty();
        if lines.next().unwrap().as_str() == "True" { ship.engine.drive.toggle(DriveType::Direct) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.drive.toggle(DriveType::Geared) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.drive.toggle(DriveType::Electric) };
        if lines.next().unwrap().as_str() == "True" { ship.engine.drive.toggle(DriveType::Hydraulic) };

        ship.trim        = lines.next().unwrap().parse()?;
        ship.hull.bb     = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.hull.units);
        ship.engine.year = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() { b.year = lines.next().unwrap().parse()?; }

        ship.hull.bow_type = lines.next().unwrap().into();
        let ram_len        = lines.next().unwrap().parse()?;
        ship.hull.bow_type = match ship.hull.bow_type {
            BowType::Ram(_)         => BowType::Ram(Measurement::new(ram_len, LengthLong, ship.hull.units)),
            BowType::BulbForward(_) => BowType::BulbForward(Measurement::new(ram_len, LengthLong, ship.hull.units)),
            _ => ship.hull.bow_type,
        };

        ship.torps[1].units = lines.next().unwrap().into();
        ship.mines.units    = lines.next().unwrap().into();
        ship.asw[0].units   = lines.next().unwrap().into();
        ship.asw[1].units   = lines.next().unwrap().into();

        for b in ship.batteries.iter_mut() { b.len = lines.next().unwrap().parse()?; }

        ship.batteries[1].shells = lines.next().unwrap().parse()?;
        ship.batteries[2].shells = lines.next().unwrap().parse()?;
        ship.batteries[3].shells = lines.next().unwrap().parse()?;
        ship.batteries[4].shells = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() { b.groups[1].distribution  = lines.next().unwrap().into(); }
        for b in ship.batteries.iter_mut() { b.groups[1].above         = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].two_mounts_up = matches!(lines.next().unwrap().as_str(), "True"); }
        for b in ship.batteries.iter_mut() { b.groups[1].on            = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].below         = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].lower_deck    = matches!(lines.next().unwrap().as_str(), "True"); }

        ship.torps[0].mounts = lines.next().unwrap().parse()?;
        ship.torps[1].mounts = lines.next().unwrap().parse()?;
        let diam_val: f64    = lines.next().unwrap().parse()?;
        ship.torps[1].diam   = Measurement::new(diam_val, LengthSmall, ship.torps[1].units);
        let len_val: f64     = lines.next().unwrap().parse()?;
        ship.torps[0].len    = Measurement::new(len_val, LengthLong, ship.torps[0].units);
        let len_val: f64     = lines.next().unwrap().parse()?;
        ship.torps[1].len    = Measurement::new(len_val, LengthLong, ship.torps[1].units);
        ship.torps[0].kind   = lines.next().unwrap().into();
        ship.torps[1].kind   = lines.next().unwrap().into();

        ship.mines.num    = lines.next().unwrap().parse()?;
        ship.mines.reload = lines.next().unwrap().parse()?;
        let wgt_val: f64  = lines.next().unwrap().parse()?;
        ship.mines.wgt    = Measurement::new(wgt_val, Weight, ship.mines.units);
        ship.mines.kind   = lines.next().unwrap().into();

        ship.asw[0].num    = lines.next().unwrap().parse()?;
        ship.asw[1].num    = lines.next().unwrap().parse()?;
        ship.asw[0].reload = lines.next().unwrap().parse()?;
        ship.asw[1].reload = lines.next().unwrap().parse()?;
        let asw0_wgt: f64  = lines.next().unwrap().parse()?;
        ship.asw[0].wgt    = Measurement::new(asw0_wgt, Weight, ship.asw[0].units);
        let asw1_wgt: f64  = lines.next().unwrap().parse()?;
        ship.asw[1].wgt    = Measurement::new(asw1_wgt, Weight, ship.asw[1].units);
        ship.asw[0].kind   = lines.next().unwrap().into();
        ship.asw[1].kind   = lines.next().unwrap().into();

        ship.wgts.hull  = lines.next().unwrap().parse()?;
        ship.wgts.on    = lines.next().unwrap().parse()?;
        ship.wgts.above = lines.next().unwrap().parse()?;

        ship.armor.incline     = lines.next().unwrap().parse()?;
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulge.thick = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulge.len   = Measurement::new(val, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.bulge.hgt   = Measurement::new(val, LengthLong, ship.armor.units);

        ship.armor.bh_kind = lines.next().unwrap().into();

        ship.armor.bh_beam      = Measurement::new(lines.next().unwrap().parse()?, LengthLong, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.deck.fc = Measurement::new(val, LengthSmall, ship.armor.units);
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.deck.qd = Measurement::new(val, LengthSmall, ship.armor.units);
        ship.armor.deck.kind    = lines.next().unwrap().into();
        let val: f64 = lines.next().unwrap().parse()?;
        ship.armor.ct_aft.thick = Measurement::new(val, LengthSmall, ship.armor.units);

        for b in ship.batteries.iter_mut() { b.groups[0].above = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[0].below = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].above = lines.next().unwrap().parse()?; }
        // Ignore extra reads of ship.batteries.groups[1].on, because, duplicate data in the file makes sense
        for _ in ship.batteries.iter_mut() { lines.next(); }
        for b in ship.batteries.iter_mut() { b.groups[1].below  = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[0].layout = lines.next().unwrap().into(); }
        for b in ship.batteries.iter_mut() { b.groups[1].layout = lines.next().unwrap().into(); }

        ship.wgts.void = lines.next().unwrap().parse()?;

        // Superfluous ship.batteries[4].layout
        for _ in 1..34 { lines.next(); }

        for line in lines.by_ref() { ship.notes.push(line); }

        // SpringSharp does not store the number of mounts in Group 0 that
        // are on the deck so we have to calculate it from the other numbers
        for b in ship.batteries.iter_mut() {
            b.groups[0].on = b.mount_num -
                b.groups[0].above - b.groups[0].below -
                b.groups[1].above - b.groups[1].below - b.groups[1].on;
        }

        // SpringSharp uses hull year for torpedo, mine and ASW year
        for t in ship.torps.iter_mut() { t.year = ship.year; }
        ship.mines.year = ship.year;
        for a in ship.asw.iter_mut() { a.year = ship.year; }

        Ok(ship)
    }

    // load {{{3
    /// Load ship from a file.
    ///
    pub fn load(p: String) -> Result<Ship, Box<dyn Error>> {
        let s = fs::read_to_string(p)?;

        let mut stream = serde_json::Deserializer::from_str(&s).into_iter::<Value>();

        // Handle opening older ship file formats
        //
        let version: Version = serde_json::from_value(stream.next().ok_or("")??)?;
        if version.version == SHIP_FILE_VERSION {
            // No special handling required
        } else { // Cannot open any other versions
            let err = format!("Cannot open ship files of this version: {}!", version.version);
            return Err(err.into());
        }

        let mut ship: Ship = serde_json::from_value(stream.next().ok_or("")??)?;

        // Set any derived values
        //
        ship.engine.set_shafts(ship.engine.shafts(), &mut ship.hull);

        Ok(ship)
    }

    // save {{{3
    /// Save ship to a file.
    ///
    pub fn save(&self, p: String) -> Result<(), Box<dyn Error>> {
        let version = serde_json::to_string(&Version { version: SHIP_FILE_VERSION })?;
        let ship    = serde_json::to_string(&self)?;

        // Empty or clear the ship file
        let _ = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&p)?;

        // Append to the ship file
        let mut file = OpenOptions::new().append(true).open(&p)?;

        writeln!(file, "{}", version)?;
        writeln!(file, "{}", ship)?;

        Ok(())
    }

    // ship_type {{{3
    /// Get a string describing the type of ship based
    /// on gun distribution, mounts and armor.
    ///
    fn ship_type(&self) -> String {
        let mut s: Vec<String> = Vec::new();

        let main = self.batteries[0].clone();
        let sec = self.batteries[1].clone();
        let ter = self.batteries[2].clone();

        if main.mount_kind == MountType::OpenBarbette ||
            sec.mount_kind == MountType::OpenBarbette
        { s.push("Barbette Ship".into()); }

        if main.groups[0].distribution == GunDistributionType::CenterlineFD ||
            main.groups[0].distribution == GunDistributionType::SidesEndsFD
        { s.push("Central Citadel Ship".into()); }

        let main_broad = main.mount_kind == MountType::Broadside;
        let sec_broad  = sec.mount_kind == MountType::Broadside;
        let ter_broad  = ter.mount_kind == MountType::Broadside;

        let main_below = (main.groups[0].below + main.groups[1].below) > 0;
        let sec_below  = (sec.groups[0].below + main.groups[1].below) > 0;
        let ter_below  = (ter.groups[0].below + main.groups[1].below) > 0;

        let main_broad_below = main_broad && main_below;
        let sec_broad_below  = sec_broad  && sec_below;
        let ter_broad_below  = ter_broad  && ter_below;

        let main_no_back = main.armor_face.imp() > 0.0;
        let sec_no_back  = sec.armor_face.imp() > 0.0;
        let ter_no_back  = ter.armor_face.imp() > 0.0;

        let main_broad_no_back = main_broad && main_no_back;
        let sec_broad_no_back  = sec_broad && sec_no_back;
        let ter_broad_no_back  = ter_broad && ter_no_back;

        let has_belt = (
            self.armor.main.wgt (self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) +
            self.armor.end.wgt  (self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) +
            self.armor.upper.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp())
        ) > 0.0;

        if main_broad || sec_broad || ter_broad {
            if has_belt {
                if main_broad_no_back || sec_broad_no_back || ter_broad_no_back {
                    s.push("Armoured Casemate Ship".into());
                } else if self.hull.freeboard.fc_len + self.hull.freeboard.fd_len < 0.5 {
                    if main_broad_below || sec_broad_below || ter_broad_below {
                        s.push("Armoured Frigate (Broadside Ironclad)".into());
                    } else {
                        s.push("Armoured Corvette (Broadside Ironclad)".into());
                    }
                } else if main_broad_below || sec_broad_below || ter_broad_below {
                    s.push("Armoured Frigate (Central Battery Ironclad)".into());
                } else {
                    s.push("Armoured Corvette (Central Battery Ironclad)".into());
                }
            } else if main_broad_below || sec_broad_below || ter_broad_below {
                s.push("Frigate (Unarmoured)".into());
            } else {
                s.push("Corvette (Unarmoured)".into());
            }
        }

        s.join("\n")
    }
}

impl Ship { // Convenience wrappers {{{2
    #[allow(dead_code)]
    pub fn ct_wgt(&self) -> f64 { // {{{3
        self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())
    }

    pub fn deck_wgt(&self) -> f64 { // {{{3
        self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine())
    }

    #[allow(dead_code)]
    pub fn battery_armor_wgt(&self, btry: &Battery) -> f64 { // {{{3
        btry.armor_wgt(self.hull.clone())
    }

    #[allow(dead_code)]
    pub fn belt_wgt(&self, belt: Belt) -> f64 { // {{{3
        belt.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp())
    }

    pub fn hp_max(&self) -> Measurement { // {{{3
        Measurement::new(self.engine.hp_max(
            self.hull.d(),
            self.hull.lwl().imp(),
            self.hull.leff(),
            self.hull.cs(),
            self.hull.ws(),
        ), Power, Imperial)
    }

    #[allow(dead_code)]
    pub fn hp_cruise(&self) -> f64 { // {{{3
        self.engine.hp_cruise(
            self.hull.d(),
            self.hull.lwl().imp(),
            self.hull.leff(),
            self.hull.cs(),
            self.hull.ws(),
        )
    }

    pub fn rf_max(&self) -> f64 { // {{{3
        self.engine.rf_max(self.hull.ws())
    }

    #[allow(dead_code)]
    pub fn rf_cruise(&self) -> f64 { // {{{3
        self.engine.rf_cruise(self.hull.ws())
    }

    pub fn rw_max(&self) -> f64 { // {{{3
        self.engine.rw_max(self.hull.d(), self.hull.lwl().imp(), self.hull.cs())
    }

    #[allow(dead_code)]
    pub fn rw_cruise(&self) -> f64 { // {{{3
        self.engine.rw_cruise(self.hull.d(), self.hull.lwl().imp(), self.hull.cs())
    }

    pub fn pw_max(&self) -> f64 { // {{{3
        self.engine.pw_max(self.hull.d(), self.hull.lwl().imp(), self.hull.cs(), self.hull.ws())
    }

    #[allow(dead_code)]
    pub fn pw_cruise(&self) -> f64 { // {{{3
        self.engine.pw_cruise(self.hull.d(), self.hull.lwl().imp(), self.hull.cs(), self.hull.ws())
    }

    pub fn d_engine(&self) -> f64 { // {{{3
        self.engine.d_engine(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())
    }

    pub fn bunker_max(&self) -> f64 { // {{{3
        self.engine.bunker_max(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())
    }
}

// Report {{{2
impl Ship { // {{{3
    // report {{{4
    /// Print report.
    ///
    pub fn report(&self) -> String {
        let mut r: Vec<String> = Vec::new();

        // Header {{{5
        addto!(r, "{}, {} {} laid down {}{}",
            self.name,
            self.country,
            self.kind,
            self.year,
            addif!(self.year != self.engine.year, " (Engine {})", self.engine.year),
        );
        if self.ship_type() != "" {
            addto!(r, "{}", self.ship_type());
        }

        // Warnings {{{5
        if self.hull.cb() <= 0.0 || self.hull.cb() > 1.0
            { addto!(r, "DESIGN FAILURE: Displacement impossible with given dimensions"); }
        if self.hull.d() < (self.wgt_broad().imp() / 4.0)
            { addto!(r, "DESIGN FAILURE: Gun weight too much for hull"); }
        if self.wgt_armor() > self.hull.d()
            { addto!(r, "DESIGN FAILURE: Armour weight too much for hull"); }
        if self.str_comp() < 0.5
            { addto!(r, "DESIGN FAILURE: Overall load weight too much for hull"); }
        if self.capsize_warn()
            { addto!(r, "DESIGN FAILURE: Ship will capsize"); }

        addto!(r);

        addto!(r, "Displacement:"); // {{{5
        addto!(r, "    {} t light; {} t standard; {} t normal; {} t full load",
            num!(self.d_lite()),
            num!(self.d_std()),
            num!(self.hull.d()),
            num!(self.d_max())
        );
        addto!(r);

        addto!(r, "Dimensions: Length (overall / waterline) x beam x draught (normal/deep)"); // {{{5
        addto!(r, "    ({:.2} ft / {:.2} ft) x {:.2} ft {}x ({:.2} / {:.2} ft)",
            self.hull.loa().imp(),
            self.hull.lwl().imp(),
            self.hull.b.imp(),
            addif!(self.hull.bb.imp() > self.hull.b.imp(), "(Bulges {:.2} ft) ", self.hull.bb.imp()),
            self.hull.t.imp(),
            self.t_max().imp()
        );
        addto!(r, "    ({:.2} m / {:.2} m) x {:.2} m {}x ({:.2} / {:.2} m)",
            self.hull.loa().metric(),
            self.hull.lwl().metric(),
            self.hull.b.metric(),
            addif!(self.hull.bb.imp() > self.hull.b.imp(), "(Bulges {:.2} m) ", self.hull.bb.metric()),
            self.hull.t.metric(),
            self.t_max().metric()
        );
        addto!(r);

        addto!(r, "Armament:"); // {{{5
        for (i, b) in self.batteries.iter().enumerate() {
            for s in b.long_desc(i == 0, self.hull.clone()).iter() {
                if !s.is_empty() {
                    addto!(r, "    {}", s);
                }
            }
        }
        addto!(r, "    Weight of broadside {} lbs / {} kg",
            num!(self.wgt_broad().imp()),
            num!(self.wgt_broad().metric()),
        );

        // Weapons {{{5
        for (i, torp) in self.torps.iter().enumerate() {
            if torp.num == 0 { continue; }

            addto!(r, "    {} Torpedoes",
                match i { 0 => "Main", 1 => "2nd", _ => "Other", }
            );
            addto!(r, "    {} - {:.1}\" / {:.0} mm, {:.2} ft / {:.2} m torpedo{} {:.3} t total",
                torp.num,
                torp.diam.imp(),
                torp.diam.metric(),
                torp.len.imp(),
                torp.len.metric(),
                match torp.num {
                    1 => " -".to_string(),
                    _ => format!("es - {:.3} t each,", torp.wgt_weaps() / torp.num as f64),
                },
                torp.wgt_weaps()
            );
            addto!(r, "        {}",
                torp.kind.desc(torp.num, torp.mounts)
            );
        }

        if self.mines.num != 0 {
            addto!(r, "    Mines");
            for s in self.mines.long_desc().iter() {
                if !s.is_empty() {
                    addto!(r, "    {}", s);
                }
            }
        }

        for (i, asw) in self.asw.iter().enumerate() {
            if asw.num == 0 { continue; }

            addto!(r, "    {} DC/AS Mortars",
                match i { 0 => "Main", 1 => "2nd", _ => "Other", }
            );
            for s in asw.long_desc().iter() {
                if !s.is_empty() {
                    addto!(r, "    {}", s);
                }
            }
        }

        // Armor {{{5
        addto!(r);
        addto!(r, "Armour:");

        if self.armor.main.thick.imp() + self.armor.end.thick.imp() + self.armor.upper.thick.imp() + self.armor.bulkhead.thick.imp() > 0.0 {
            addto!(r, " - Belts:    Width (max)    Length (avg)    Height (avg)");
            if self.armor.main.thick.imp() > 0.0 {
                addto!(r, "    Main:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.main.thick.imp(), if self.armor.main.thick.imp() < 10.0 { 2 } else { 1 }),
                    self.armor.main.thick.metric(),
                    self.armor.main.len.imp(),
                    self.armor.main.len.metric(),
                    self.armor.main.hgt.imp(),
                    self.armor.main.hgt.metric(),
                );
            }

            if self.armor.end.thick.imp() > 0.0 {
                addto!(r, "    Ends:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.end.thick.imp(), if self.armor.end.thick.imp() < 10.0 { 2 } else { 1 }),
                    self.armor.end.thick.metric(),
                    self.armor.end.len.imp(),
                    self.armor.end.len.metric(),
                    self.armor.end.hgt.imp(),
                    self.armor.end.hgt.metric(),
                );
                if self.armor.main.len.imp() + self.armor.end.len.imp() < self.hull.lwl().imp() {
                    addto!(r, "    {:.2} ft / {:.2} m Unarmoured ends",
                        self.hull.lwl().imp() - self.armor.main.len.imp() - self.armor.end.len.imp(),
                        self.hull.lwl().metric() - self.armor.main.len.metric() - self.armor.end.len.metric(),
                    );
                }
            } else if self.armor.main.len.imp() < self.hull.lwl().imp() {
                addto!(r, "    Ends:    Unarmoured");
            }

            if self.armor.upper.thick.imp() > 0.0 {
                addto!(r, "    Upper:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.upper.thick.imp(), if self.armor.upper.thick.imp() < 10.0 { 2 } else { 1 }),
                    self.armor.upper.thick.metric(),
                    self.armor.upper.len.imp(),
                    self.armor.upper.len.metric(),
                    self.armor.upper.hgt.imp(),
                    self.armor.upper.hgt.metric(),
                );
            }

            if self.armor.main.thick.imp() > 0.0 {
                addto!(r, "    Main Belt covers {} % of normal length",
                    pct!(self.armor.belt_coverage(self.hull.lwl().imp()))
                );
                if self.armor.belt_coverage(self.hull.lwl().imp()) < self.hull_room() {
                    addto!(r, "    Main belt does not fully cover magazines and engineering spaces");
                }
            }

            if self.armor.incline != 0.0 {
                addto!(r, "    Main Belt inclined {:.2} degrees (positive = in)",
                    self.armor.incline
                );
            }

            if self.armor.bulkhead.thick.imp() > 0.0 {
                addto!(r);
                addto!(r, "- Torpedo Bulkhead - {}:", self.armor.bh_kind);
                addto!(r, "        {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.bulkhead.thick.imp(), if self.armor.bulkhead.thick.imp() < 10.0 { 2 } else { 1 }),
                    self.armor.bulkhead.thick.metric(),
                    self.armor.bulkhead.len.imp(),
                    self.armor.bulkhead.len.metric(),
                    self.armor.bulkhead.hgt.imp(),
                    self.armor.bulkhead.hgt.metric(),
                );
                addto!(r, "    Beam between torpedo bulkheads {:.2} ft / {:.2} m",
                    self.armor.bh_beam.imp(),
                    self.armor.bh_beam.metric()
                );
                addto!(r);
            }

            if self.armor.bulge.thick.imp() > 0.0 || self.wgts.void > 0 {
                addto!(r, "- Hull {}:",
                    if self.hull.b.imp() == self.hull.bb.imp() { "void" }
                    else { "Bulges" }
                );
                addto!(r, "        {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.bulge.thick.imp(), if self.armor.bulge.thick.imp() < 10.0 { 2 } else { 1 }),
                    self.armor.bulge.thick.metric(),
                    self.armor.bulge.len.imp(),
                    self.armor.bulge.len.metric(),
                    self.armor.bulge.hgt.imp(),
                    self.armor.bulge.hgt.metric(),
                );
                addto!(r);
            }
        }

        if self.wgt_gun_armor() > 0.0 {
            addto!(r, "- Gun armour:    Face (max)    Other gunhouse (avg)    Barbette/hoist (max)");

            for (i, b) in self.batteries.iter().enumerate() {
                if b.armor_face.imp() == 0.0 &&
                b.armor_back.imp() == 0.0 &&
                b.armor_barb.imp() == 0.0 { continue; }
                addto!(r, "    {}:    {}        {}            {}",
                    match i { 0 => "Main", 1 => "2nd", 2 => "3rd", 3 => "4th", 4 => "5th", _ => "Other", },
                    if b.armor_face.imp() == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_face.imp(), if b.armor_face.imp() >= 10.0 { 1 } else { 2 }), b.armor_face.metric()) },
                    if b.armor_back.imp() == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_back.imp(), if b.armor_back.imp() >= 10.0 { 1 } else { 2 }), b.armor_back.metric()) },
                    if b.armor_barb.imp() == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_barb.imp(), if b.armor_barb.imp() >= 10.0 { 1 } else { 2 }), b.armor_barb.metric()) },
                );
            }
            addto!(r);
        }

        if self.armor.deck.fc.imp() + self.armor.deck.md.imp() + self.armor.deck.qd.imp() > 0.0 {
            addto!(r, "- {}:", self.armor.deck.kind);
            // TODO: Change spelling to Fore (required to match SpringSharp reports)
            addto!(r, "    For and Aft decks: {:.2}\" / {:.0} mm",
                self.armor.deck.md.imp(),
                self.armor.deck.md.metric()
            );
            // TODO: Change spelling to Quarterdeck (required to match SpringSharp reports)
            addto!(r, "    Forecastle: {:.2}\" / {:.0} mm    Quarter deck: {:.2}\" / {:.0} mm",
                self.armor.deck.fc.imp(),
                self.armor.deck.fc.metric(),
                self.armor.deck.qd.imp(),
                self.armor.deck.qd.metric()
            );
            addto!(r);
        }

        if self.armor.ct_fwd.thick.imp() + self.armor.ct_aft.thick.imp() > 0.0 {
            // TODO: Remove stray space before comma (required to match SpringSharp reports)
            addto!(r, "- Conning towers: Forward {:.2}\" / {:.0} mm, Aft {:.2}\" / {:.0} mm",
                self.armor.ct_fwd.thick.imp(),
                self.armor.ct_fwd.thick.metric(),
                self.armor.ct_aft.thick.imp(),
                self.armor.ct_aft.thick.metric()
            );
            addto!(r);
        }

        addto!(r, "Machinery:"); // {{{5
        if self.engine.vmax != 0.0 {
            addto!(r, "    {}, {},", self.engine.fuel, self.engine.boiler);
            addto!(r, "    {}, {} shaft{}, {} {} / {} Kw = {:.2} kts",
                self.engine.drive,
                self.engine.shafts(),
                plural(self.engine.shafts()),
                num!(self.hp_max().imp()),
                self.engine.boiler.hp_type(),
                num!(self.hp_max().metric()),
                self.engine.vmax
            );
            addto!(r, "    Range {}nm at {:.2} kts",
                num!(self.engine.range),
                self.engine.vcruise
            );
            addto!(r, "    Bunker at max displacement = {} tons{}",
                num!(self.bunker_max()),
                if self.engine.pct_coal > 0.0 { format!(" ({}% coal)", pct!(self.engine.pct_coal)) } else { "".into() }
            );
            let ratio = self.hp_max().imp() / self.engine.shafts() as f64;

            if ratio > 20_000.0 && self.engine.boiler.is_reciprocating()
                { addto!(r, "    Caution: Too much power for reciprocating engines."); }
            else if ratio > 75_000.0
                { addto!(r, "    Caution: Too much power for number of propellor shafts."); }

            if self.wgt_engine() < self.d_engine() / 5.0 {
                addto!(r, "    Caution: Delicate, lightweight machinery.");
            }
        } else {
            addto!(r, "    Immobile floating battery");
        }
        addto!(r);

        addto!(r, "Complement:"); // {{{5
        addto!(r, "    {} - {}", self.crew_min(), self.crew_max());
        addto!(r);

        addto!(r, "Cost:"); // {{{5
        addto!(r, "    £{:.3} million / ${:.3} million",
            self.cost_lb(),
            self.cost_dollar()
        );
        addto!(r);

        addto!(r, "Distribution of weights at normal displacement:"); // {{{5
        addto!(r, "    Armament: {}",
            self.percent_calc(self.wgt_guns() + self.wgt_gun_mounts() + self.wgt_weaps()),
        );

        if self.wgt_guns() > 0.0 {
            addto!(r, "    - Guns: {}",
                self.percent_calc(self.wgt_guns() + self.wgt_gun_mounts()),
            );
        }

        if self.torps[0].wgt() + self.torps[1].wgt() + self.mines.wgt() + self.asw[0].wgt() + self.asw[1].wgt() > 0.0 {
            addto!(r, "    - Weapons: {}",
                self.percent_calc(self.torps[0].wgt() + self.torps[1].wgt() + self.mines.wgt() + self.asw[0].wgt() + self.asw[1].wgt()),
            );
        }

        if self.wgt_armor() > 0.0 {
            addto!(r, "    Armour: {}", self.percent_calc(self.wgt_armor()),);

            if self.armor.main.thick.imp() + self.armor.end.thick.imp() + self.armor.upper.thick.imp() > 0.0 {
                addto!(r, "    - Belts: {}",
                    self.percent_calc(self.armor.main.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) +
                        self.armor.end.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) +
                        self.armor.upper.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp())),
                );
            }

            if self.armor.bulkhead.thick.imp() > 0.0 {
                addto!(r, "    - Torpedo bulkhead: {}",
                    self.percent_calc(self.armor.bulkhead.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp())),
                );
            }

            if self.armor.bulge.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp()) > 0.0 {
                addto!(r, "    - {}: {}",
                    if self.hull.b.imp() == self.hull.bb.imp() { "Void" } else { "Bulges" },
                    self.percent_calc(self.armor.bulge.wgt(self.hull.lwl().imp(), self.hull.cwp(), self.hull.b.imp())),
                );
            }

            if self.wgt_gun_armor() > 0.0 {
                addto!(r, "    - Armament: {}",
                    self.percent_calc(self.wgt_gun_armor()),
                );
            }

            if self.armor.deck.fc.imp() + self.armor.deck.md.imp() + self.armor.deck.qd.imp() > 0.0 {
                addto!(r, "    - Armour Deck: {}",
                    self.percent_calc(self.deck_wgt()),
                );
            }

            if self.armor.ct_fwd.thick.imp() + self.armor.ct_aft.thick.imp() > 0.0 {
                addto!(r, "    - Conning Tower{}: {}",
                    if self.armor.ct_fwd.thick.imp() > 0.0 && self.armor.ct_aft.thick.imp() > 0.0 {
                        "s"
                    } else { "" },
                    self.percent_calc(self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())),
                );
            }
        }

        addto!(r, "    Machinery: {}", self.percent_calc(self.wgt_engine()),);
        addto!(r, "    Hull, fittings & equipment: {}",
            self.percent_calc(self.wgt_hull()),
        );
        addto!(r, "    Fuel, ammunition & stores: {}",
            self.percent_calc(self.wgt_load()),
        );

        addto!(r, "    Miscellaneous weights: {}",
            self.percent_calc(self.wgts.wgt() as f64),
        );
        if self.wgts.vital > 0 { addto!(r, "    - Hull below water: {} tons",
                num!(self.wgts.vital)
            );
        }
        if self.wgts.void > 0 {
            addto!(r, "    - {} void weights: {} tons",
                if self.hull.bb.imp() > self.hull.b.imp() { "Bulge" } else { "Hull" },
                num!(self.wgts.void),
            );
        }
        if self.wgts.hull > 0  { addto!(r, "    - Hull above water: {:.0} tons", self.wgts.hull) };
        if self.wgts.on > 0    { addto!(r, "    - On freeboard deck: {:.0} tons", self.wgts.on) };
        if self.wgts.above > 0 { addto!(r, "    - Above deck: {:.0} tons", self.wgts.above) };

        addto!(r);

        addto!(r, "Overall survivability and seakeeping ability:"); // {{{5
        addto!(r, "    Survivability (Non-critical penetrating hits needed to sink ship):");
        addto!(r, "    {} lbs / {} Kg = {:.1} x {:.1} \" / {:.0} mm shells or {:.1} torpedoes",
            num!(self.flotation().imp()),
            num!(self.flotation().metric()),
            self.damage_shell_num(),
            self.damage_shell_size().imp(),
            self.damage_shell_size().metric(),
            self.damage_torp_num()
        );
        addto!(r, "    Stability (Unstable if below 1.00): {:.2}",
            self.stability_adj()
        );
        addto!(r, "    Metacentric height {:.1} ft / {:.1} m",
            self.metacenter().imp(),
            self.metacenter().metric()
        );
        addto!(r, "    Roll period: {:.1} seconds", self.roll_period());
        addto!(r, "    Steadiness    - As gun platform (Average = 50 %): {:.0} %", self.steadiness());
        addto!(r, "        - Recoil effect (Restricted arc if above 1.00): {:.2}",
            self.recoil()
        );
        addto!(r, "    Seaboat quality (Average = 1.00): {:.2}", self.seakeeping());
        addto!(r);

        addto!(r, "Hull form characteristics:"); // {{{5
        addto!(r, "    Hull has {},", self.hull.freeboard_desc());
        addto!(r, "    {} and {}", self.hull.bow_type, self.hull.stern_type);
        addto!(r, "    Block coefficient (normal/deep): {:.3} / {:.3}", self.hull.cb(), self.cb_max());
        addto!(r, "    Length to Beam Ratio: {:.2} : 1", self.hull.len2beam());
        addto!(r, "    'Natural speed' for length: {:.2} kts", self.hull.vn());
        addto!(r, "    Power going to wave formation at top speed: {} %", pct!(self.pw_max()));
        addto!(r, "    Trim (Max stability = 0, Max steadiness = 100): {}", self.trim);
        addto!(r, "    Bow angle (Positive = bow angles forward): {:.2} degrees", self.hull.bow_angle);
        addto!(r, "    Stern overhang: {:.2} ft / {:.2} m",
            self.hull.stern_overhang.imp(),
            self.hull.stern_overhang.metric()
        );
        addto!(r, "    Freeboard (% = length of deck as a percentage of waterline length):");
        addto!(r, "            Fore end, Aft end");
        addto!(r, "    - Forecastle:    {} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            pct!(self.hull.freeboard.fc_len, 2),   self.hull.freeboard.fc_fwd.imp(), self.hull.freeboard.fc_fwd.metric(), self.hull.freeboard.fc_aft.imp(), self.hull.freeboard.fc_aft.metric()
        );
        addto!(r, "    - Forward deck:    {} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            pct!(self.hull.freeboard.fd_len, 2),   self.hull.freeboard.fd_fwd.imp(), self.hull.freeboard.fd_fwd.metric(), self.hull.freeboard.fd_aft.imp(), self.hull.freeboard.fd_aft.metric()
        );
        addto!(r, "    - Aft deck:    {} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            pct!(self.hull.freeboard.ad_len(), 2), self.hull.freeboard.ad_fwd.imp(), self.hull.freeboard.ad_fwd.metric(), self.hull.freeboard.ad_aft.imp(), self.hull.freeboard.ad_aft.metric()
        );
        addto!(r, "    - Quarter deck:    {} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            pct!(self.hull.freeboard.qd_len, 2),   self.hull.freeboard.qd_fwd.imp(), self.hull.freeboard.qd_fwd.metric(), self.hull.freeboard.qd_aft.imp(), self.hull.freeboard.qd_aft.metric()
        );
        addto!(r, "    - Average freeboard:        {:.2} ft / {:.2} m",
            self.hull.freeboard.average().imp(), self.hull.freeboard.average().metric()
        );
        if self.hull.is_wet_fwd() {
            addto!(r, "    Ship tends to be wet forward");
        }
        addto!(r);

        addto!(r, "Ship space, strength and comments:"); // {{{5
        addto!(r, "    Space    - Hull below water (magazines/engines, low = better): {} %",
            pct!(self.hull_room(), 1)
        );
        addto!(r, "        - Above water (accommodation/working, high = better): {} %",
            pct!(self.deck_room(), 1)
        );
        addto!(r, "    Waterplane Area: {} Square feet or {} Square metres",
            num!(self.hull.wp().imp()),
            num!(self.hull.wp().metric())
        );
        addto!(r, "    Displacement factor (Displacement / loading): {} %",
            pct!(self.d_factor())
        );
        addto!(r, "    Structure weight / hull surface area: {:.0} lbs/sq ft or {:.0} Kg/sq metre",
            self.wgt_struct().imp(),
            self.wgt_struct().metric()
        );
        addto!(r, "Hull strength (Relative):");
        addto!(r, "        - Cross-sectional: {:.2}", self.str_cross());
        addto!(r, "        - Longitudinal: {:.2}", self.str_long());
        addto!(r, "        - Overall: {:.2}", self.str_comp());

        if self.tender_warn() && !self.capsize_warn() {
            addto!(r, "Caution: Poor stability - excessive risk of capsizing");
        }
        if self.hull_strained() {
            addto!(r, "Caution: Hull subject to strain in open-sea");
        }
        addto!(r, "    {} machinery, storage, compartmentation space", self.hull_room_quality());
        addto!(r, "    {} accommodation and workspace room", self.deck_room_quality());
        for s in self.seakeeping_desc() {
            addto!(r, "    {}", s);
        }
        if self.bh_beam_too_wide() {
            addto!(r, "Beam between bulkheads too wide");
        }

        addto!(r);

        // Custom Notes {{{5
        for s in self.notes.iter() {
            addto!(r, "{}", s);
        }

        r.join("\n")
    }
}

// Internals Output {{{2
#[cfg(debug_assertions)]
impl Ship {
    // Print internal values {{{3
    pub fn internals(&self) -> String {
        let mut s: Vec<String> = Vec::new();

        s.push("Internal values".to_string());
        s.push("===============".to_string());
        s.push("".to_string());
        s.push("Gun Batteries".to_string());
        s.push("------------".to_string());
        s.push(format!("wgt_guns = {}", self.wgt_guns()));
        s.push(format!("wgt_gun_mounts = {}", self.wgt_gun_mounts()));
        s.push(format!("wgt_mag = {}", self.wgt_mag()));
        s.push(format!("wgt_gun_armor = {}", self.wgt_gun_armor()));
        s.push(format!("wgt_borne = {}", self.wgt_borne()));
        s.push(format!("super_factor = {}", self.gun_super_factor()));
        s.push(format!("gun_wtf = {}", self.gun_wtf()));
        s.push("".to_string());

        for (i, b) in self.batteries.iter().enumerate() {
            s.push(format!("battery[{}]", i));
            s.push("-----------".to_string());
            b.internals(self.hull.clone(), self.wgt_broad().imp());
            s.push("".to_string());
        }

        s.push(format!("Cs = {}", self.hull.cs()));
        s.push(format!("Cm = {}", Hull::cm(self.hull.cb())));
        s.push(format!("Cp = {}", Hull::cp(self.hull.cb())));
        s.push(format!("Cwp = {}", self.hull.cwp()));
        s.push(format!("WP = {}", self.hull.wp().imp()));
        s.push(format!("WS = {}", self.hull.ws()));
        s.push(format!("Ts = {}", self.hull.ts()));
        s.push("".to_string());
        s.push(format!("Stem length = {}", self.hull.stem_len()));
        if let BowType::Ram(len) = self.hull.bow_type { s.push(format!("Ram length = {}", len.imp())); }
        if let BowType::BulbForward(len) = self.hull.bow_type { s.push(format!("Bulb length = {}", len.imp())); }
        s.push(format!("Freeboard dist = {}", self.hull.freeboard.distributed()));
        s.push(format!("Leff = {}", self.hull.leff()));
        s.push("".to_string());
        s.push(format!("Rf max = {}", self.engine.rf_max(self.hull.ws())));
        s.push(format!("Rf cruise = {}", self.engine.rf_cruise(self.hull.ws())));
        s.push(format!("Rw max = {}", self.engine.rw_max(self.hull.d(), self.hull.lwl().imp(), self.hull.cs())));
        s.push(format!("Rw cruise = {}", self.engine.rw_cruise(self.hull.d(), self.hull.lwl().imp(), self.hull.cs())));
        s.push(format!("Pw max = {}", self.engine.pw_max(self.hull.d(), self.hull.lwl().imp(), self.hull.cs(), self.hull.ws())));
        s.push(format!("Pw cruise = {}", self.engine.pw_cruise(self.hull.d(), self.hull.lwl().imp(), self.hull.cs(), self.hull.ws())));
        s.push("".to_string());
        s.push(format!("hp max = {}", self.engine.hp_max(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push(format!("hp cruise = {}", self.engine.hp_cruise(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push("".to_string());

        s.push(format!("wgt_load = {}", self.wgt_load()));
        s.push(format!("wgt_hull = {}", self.wgt_hull()));
        s.push(format!("wgt_hull_plus = {}", self.wgt_hull_plus()));
        s.push(format!("wgt_misc = {}", self.wgts.wgt()));
        s.push(format!("wgt_armor = {}", self.wgt_armor()));
        s.push("".to_string());

        s.push(format!("main belt = {}", self.armor.main.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp())));
        s.push(format!("upper belt = {}", self.armor.upper.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp())));
        s.push(format!("end belt = {}", self.armor.end.wgt(self.hull.d(), self.hull.cwp(), self.hull.b.imp())));
        s.push(format!("deck = {}", self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine())));
        s.push("".to_string());

        s.push(format!("wgt_engine = {}", self.wgt_engine()));
        s.push(format!("d_engine = {}", self.engine.d_engine(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push(format!("d_factor = {}", self.d_factor()));
        s.push(format!("bunker (normal) = {}", self.engine.bunker(self.hull.d(), self.hull.lwl().imp(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push(format!("bunker_factor = {}", self.engine.boiler.bunker_factor(self.engine.year)));
        s.push("".to_string());

        s.push(format!("stability = {}", self.stability()));
        s.push(format!("seaboat = {}", self.seaboat()));
        s.push("".to_string());

        s.push(format!("{:?}", self.engine.fuel));
        s.push(format!("{:?}", self.engine.boiler));
        s.push(format!("{:?}", self.engine.drive));
        s.push(format!("num_engines = {}", self.engine.num_engines()));

        s.push("".to_string());

        s.push(format!("gun_concentration = {}", self.gun_concentration()));
        s.push(format!("str_cross = {}", self.str_cross()));
        s.push(format!("str_long = {}", self.str_long()));
        s.push(format!("str_comp = {}", self.str_comp()));
        s.push(format!("flotation = {}", self.flotation().imp()));

        s.join("\n")
    }
}

// Testing Ship {{{2
#[cfg(test)]
mod ship {
    use super::*;
    use crate::calc::test_support::*;
    use crate::calc::TorpedoMountType;
    use tempfile::NamedTempFile;

    // Test year_adj {{{3
    macro_rules! test_year_adj {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, year) = $value;

                    assert_eq!(expected, to_place(Ship::year_adj(year), 5));
                }
            )*
        }
    }

    test_year_adj! {
        // name:    (year_adj, year)
        year_adj_1: (0.985, 1889),
        year_adj_2: (1.0, 1890),
        year_adj_3: (1.0, 1949),
        year_adj_4: (1.0, 1950),
        year_adj_5: (0.0, 1951),
    }

    // Test deck_space {{{3
    macro_rules! test_deck_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind) = $value;

                    let mut ship = test_ship();

                    ship.hull.set_d(7000.0);
                    ship.hull.set_lwl(500.0, Imperial);

                    ship.torps[0].year = 1920;
                    ship.torps[0].num = 3;
                    ship.torps[0].mounts = 2;
                    ship.torps[0].diam = Measurement::new(20.0, LengthSmall, ship.torps[0].units);
                    ship.torps[0].len  = Measurement::new(10.0, LengthLong,  ship.torps[0].units);
                    ship.torps[0].kind = kind;

                    ship.torps[1].num = 0;

                    assert_eq!(expected, to_place(ship.deck_space(), 4));
                }
            )*
        }
    }

    test_deck_space! {
        // name:    (deck_space, kind)
        deck_space_1: (0.002, TorpedoMountType::FixedTubes),
        deck_space_2: (0.0039, TorpedoMountType::DeckSideTubes),
        deck_space_3: (0.0415, TorpedoMountType::CenterTubes),
        deck_space_4: (0.0039, TorpedoMountType::DeckReloads),
        deck_space_5: (0.0, TorpedoMountType::BowTubes),
        deck_space_6: (0.0, TorpedoMountType::SternTubes),
        deck_space_7: (0.0, TorpedoMountType::BowAndSternTubes),
        deck_space_8: (0.0, TorpedoMountType::SubmergedSideTubes),
        deck_space_9: (0.0, TorpedoMountType::SubmergedReloads),
    }

    // Test hull_space {{{3
    macro_rules! test_hull_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind) = $value;

                    let mut ship = test_ship();

                    ship.hull.set_d(7000.0);
                    ship.hull.set_lwl(500.0, Imperial);

                    ship.torps[0].year = 1920;
                    ship.torps[0].num = 3;
                    ship.torps[0].mounts = 2;
                    ship.torps[0].diam = Measurement::new(20.0, LengthSmall, ship.torps[0].units);
                    ship.torps[0].len  = Measurement::new(10.0, LengthLong,  ship.torps[0].units);
                    ship.torps[0].kind = kind;

                    ship.torps[1].num = 0;

                    assert_eq!(expected, to_place(ship.hull_space(), 4));
                }
            )*
        }
    }

    test_hull_space! {
        // name:    (hull_space, kind)
        hull_space_1: (0.0, TorpedoMountType::FixedTubes),
        hull_space_2: (0.0, TorpedoMountType::DeckSideTubes),
        hull_space_3: (0.0, TorpedoMountType::CenterTubes),
        hull_space_4: (0.0, TorpedoMountType::DeckReloads),
        hull_space_5: (0.0064, TorpedoMountType::BowTubes),
        hull_space_6: (0.0064, TorpedoMountType::SternTubes),
        hull_space_7: (0.0064, TorpedoMountType::BowAndSternTubes),
        hull_space_8: (0.0064, TorpedoMountType::SubmergedSideTubes),
        hull_space_9: (0.0011, TorpedoMountType::SubmergedReloads),
    }

    // Test crew_max {{{3
    macro_rules! test_crew_max {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let mut ship = Ship::default();

                    let (expected, d) = $value;
                    ship.hull.set_d(d);

                    assert_eq!(expected, ship.crew_max());
                }
            )*
        }
    }

    test_crew_max! {
        // name:            (crew, d)
        crew_max_d_eq_zero: (0, 0.0),
        crew_max_d_eq_1000: (115, 1000.0),
    }

    // Test crew_min {{{3
    macro_rules! test_crew_min {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let mut ship = Ship::default();

                    let (expected, d) = $value;
                    ship.hull.set_d(d);

                    assert_eq!(expected, ship.crew_min());
                }
            )*
        }
    }

    test_crew_min! {
        // name:            (crew, d)
        crew_min_d_eq_zero: (0, 0.0),
        crew_min_d_eq_1000: (88, 1000.0),
    }

    // Test wgt_engine {{{3
    macro_rules! test_wgt_engine {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, d) = $value;

                    let mut ship = test_ship();
                    ship.hull.set_d(d);

                    assert_eq!(expected, to_place(ship.wgt_engine(), 2));
                }
            )*
        }
    }

    test_wgt_engine! {
        // name:           (wgt_engine, d)
        wgt_engine_d_599:  (371.6,      599.0),
        wgt_engine_d_600:  (372.23,     600.0),
        wgt_engine_d_4999: (11_339.23,  4999.0),
        wgt_engine_d_5000: (11_344.2,   5_000.0),
    }

    // Test ship_save_load_roundtrip {{{3
    #[test]
    fn ship_save_load_roundtrip() {
        let path = NamedTempFile::new()
            .unwrap()
            .path()
            .to_str()
            .unwrap()
            .to_string();

        test_ship().save(path.clone()).unwrap();
        assert!(Ship::load(path).is_ok());
    }

    // Test ship_convert_roundtrip {{{3
    #[test]
    fn ship_convert_roundtrip() {
        let from = concat!(env!("CARGO_MANIFEST_DIR"), "/test.sship");
        let to   = NamedTempFile::new()
            .unwrap()
            .path()
            .to_str()
            .unwrap()
            .to_string();

        Ship::convert(from.into())
            .unwrap()
            .save(to.clone())
            .unwrap();

        assert!(Ship::load(to).is_ok());
    }

    // Test ship_load_missing {{{3
    #[test]
    fn ship_load_missing() {
        let tmp  = NamedTempFile::new().unwrap();
        let path = tmp
            .path()
            .to_str()
            .unwrap()
            .to_string();
        drop(tmp);

        assert!(Ship::load(path).is_err());
    }

    // Test ship_convert_missing {{{3
    #[test]
    fn ship_convert_missing() {
        let tmp  = NamedTempFile::new().unwrap();
        let path = tmp
            .path()
            .to_str()
            .unwrap()
            .to_string();
        drop(tmp);

        assert!(Ship::convert(path).is_err());
    }
}

// SeaType {{{1
/// Levels of seakeeping ability.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum SeaType {
    #[default]
    BadSea,
    PoorSea,
    FineSea,
    GoodSea,
    Error, // This is an...error if it shows up anywhere
}
