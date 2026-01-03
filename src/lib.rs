mod hull;
use hull::{Hull, BowType};

mod armor;
use armor::{Armor, BulkheadType};

mod engine;
use engine::{Engine, FuelType, BoilerType, DriveType};

mod weapons;
use weapons::{Battery, Torpedoes, Mines, ASW};
use weapons::{MountType, GunDistributionType};

mod weights;
use weights::MiscWgts;

mod units;
use units::Units::*;
use units::metric;
use units::UnitType::*;

use format_num::format_num;

use serde::{Serialize, Deserialize};
use serde_json::Value;

use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

/// File extension for sharpie files.
pub const SHIP_FILE_EXT: &str = "ship";
/// File extension for Springsharp files.
pub const SS_SHIP_FILE_EXT: &str = "sship";

/// The Ship file version created by this version of sharpie.
pub const SHIP_FILE_VERSION: u32 = 1;

// Version {{{1
/// Holds Ship file version information.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Version {
    version: u32,
}

// Testing support {{{1
#[cfg(test)]
mod test_support {
    // Round a float to a given number of digits
    //
    // This makes it much easier to test results that
    // are floats.
    pub fn to_place(n: f64, digits: u32) -> f64 {
        let mult = 10_u32.pow(digits) as f64;

        (n * mult).round() / mult
    }
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
}

impl Default for Ship { // {{{2
    fn default() -> Ship {
        Ship {
            name: "".into(),
            country: "".into(),
            kind: "".into(),
            year: 0,

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
        }
    }
}

impl Ship { // {{{2
    /// Pounds in a long ton.
    const POUND2TON: f64 = 2240.0;

    // year_adj {{{3
    /// Year adjustment factor for various calculations.
    ///
    pub fn year_adj(year: u32) -> f64 {
             if year <= 1890 { 1.0 - (1890 - year) as f64 / 66.666664 }
        else if year <= 1950 { 1.0 }
        else                 { 0.0 }
    }

    // deck_space {{{3
    /// Relative measure of hull space based on waterplane area, freeboard and
    /// displacement adjusted for above water torpedoes.
    ///
    pub fn deck_space(&self) -> f64 {
        let mut space = 0.0;
        for w in self.torps.iter() {
            space += w.deck_space(self.hull.b); 
        }

        space / self.hull.wp()
    }

    // hull_space {{{3
    /// Proportional measure of weights of engines, guns, magazines,
    /// miscellaneous weights, ships stores, torpedo bulkheads and hull mounted
    /// torpedoes to displacement to estimate the minimum length of the
    /// "vitalspace" needed to contain these relative to a norm of 65% of water
    /// length.
    ///
    pub fn hull_space(&self) -> f64 {
        let mut space = 0.0;
        for w in self.torps.iter() {
            space += w.hull_space(); 
        }
        space / (self.hull.d() * Hull::FT3_PER_TON_SEA)
    }

    // wgt_bunker {{{3
    /// Convenience function to get bunkerage weight from the engine.
    ///
    fn wgt_bunker(&self) -> f64 {
        self.engine.bunker(
            self.hull.d(),
            self.hull.lwl(),
            self.hull.leff(),
            self.hull.cs(),
            self.hull.ws()
        )
    }

    // wgt_load {{{3
    /// Weight of bunkerage, magazine and stores.
    ///
    fn wgt_load(&self) -> f64 {
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
    pub fn t_max(&self) -> f64 {
        self.hull.t_calc(self.d_max())
    }

    // cb_max {{{3
    /// Block coeficcient at maximum displacement.
    ///
    pub fn cb_max(&self) -> f64 {
        self.hull.cb_calc(self.d_max(), self.t_max())
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
    pub fn vitalspace(&self) -> f64 {
        (1.0 - 0.65 * self.hull_room()) * 50.0 - 0.01
    }

    // vitalspace_length {{{3
    /// Minimum armor belt length to cover
    /// engine and magazine spaces.
    ///
    pub fn vitalspace_length(&self) -> f64 {
        self.hull.lwl() * 0.65 * self.hull_room() + 0.01
    }

    // room {{{3
    /// XXX: I do not know what this does.
    ///
    fn room(&self) -> f64 {
        (
            self.wgt_mag() +
            self.hull.d() * 0.02 +
            self.wgt_borne() * 6.4 +
            self.wgt_engine() * 3.0 +
            self.wgts.vital as f64 +
            self.wgts.hull as f64
        ) / (self.hull.d() * 0.94) / (1.0 - self.hull_space())
    }

    // hull_room {{{3
    /// Ratio of the sum of weights of the engine, magazines, ship's stores, torpedo
    /// bulkheads, hull mounted torpedoes and miscellaneous weights to displacement.
    ///
    pub fn hull_room(&self) -> f64 {
        self.room() * if self.armor.bulkhead.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) > 0.1 {
            self.hull.b / self.armor.bh_beam
        } else { 1.0 }
    }

    // deck_room {{{3
    /// XXX: Deck analog of hull_room()
    ///
    pub fn deck_room(&self) -> f64 {
        self.hull.wp() /
            Hull::FT3_PER_TON_SEA /
            15.0 * (1.0 - self.deck_space()) /
            self.crew_min() as f64 * self.hull.freeboard_dist()
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
        ((self.hull.d()-self.wgt_load())*0.00014+self.wgt_engine()*0.00056+(self.wgt_borne()*8.0)*0.00042)*
            if self.year as f64 +2.0>1914.0 {
                1.0+(self.year as f64 +1.5-1914.0)/5.5
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
        (
            (self.wgt_broad()/self.hull.d() * self.hull.freeboard_dist() * self.gun_super_factor() / self.hull.bb) *

            ( self.hull.d().powf(1.0 / 3.0) / self.hull.bb * 3.0 ).powf(2.0) * 7.0
        ) /
            if self.stability_adj() > 0.0 {
                self.stability_adj() * ((50.0 - self.steadiness()) / 150.0 + 1.0)
            } else { 1.0 }
    }

    // metacenter {{{3
    /// A measure of vertical equilibrium.
    ///
    pub fn metacenter(&self) -> f64 {
        self.hull.b.powf(1.5) * (self.stability_adj() - 0.5) / 0.5 / 200.0
    }

    // seaboat {{{3
    /// Intermediate calculations for seakeeping() and steadiness().
    ///
    fn seaboat(&self) -> f64 {
        let a = (self.hull.free_cap(self.cap_calc_broadside()) / (2.4 * self.hull.d().powf(0.2))).sqrt() *
            (
                (self.stability() * 5.0 * (self.hull.bb / self.hull.lwl())).powf(0.2) *
                (self.hull.free_cap(self.cap_calc_broadside()) / self.hull.lwl() * 20.0).sqrt() *
                (
                    self.hull.d() /
                        (
                            self.hull.d() +
                            self.armor.end.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) * 3.0 +
                            self.wgt_hull_plus() / 3.0 +
                            (
                                self.wgt_borne() +
                                self.wgt_gun_armor()
                            ) * self.super_factor_long()
                        )
                )
            ) * 8.0;

        let b = a * if (self.hull.t / self.hull.bb) < 0.3 {
                (self.hull.t / self.hull.bb / 0.3).sqrt()
            } else {
                1.0
            };

        let c = b *
            if (self.engine.rf_max(self.hull.ws()) / (self.engine.rf_max(self.hull.ws()) + self.engine.rw_max(self.hull.d(), self.hull.lwl(), self.hull.cs()))) < 0.55 &&
                self.engine.vmax > 0.0
            {
                (self.engine.rf_max(self.hull.ws()) / (self.engine.rf_max(self.hull.ws()) + self.engine.rw_max(self.hull.d(), self.hull.lwl(), self.hull.cs()))).powf(2.0)
            } else {
                0.3025
            };

        f64::min(c, 2.0)
    }

    // seakeeping {{{3
    /// The sea keeping ability of the ship.
    ///
    pub fn seakeeping(&self) -> f64 {
        self.seaboat() * f64::min(self.steadiness(), 50.0) / 50.0
    }

    // tender_warn {{{3
    /// If ship has an excessive risk of capsizing.
    ///
    fn tender_warn(&self) -> bool {
        if self.stability_adj() <= 0.995 {
            true
        } else {
            false
        }
    }

    // capsize_warn {{{3
    /// If ship will capsize.
    ///
    fn capsize_warn(&self) -> bool {
        if self.metacenter() <= 0.0 {
            true
        } else {
            false
        }
    }

    // hull_strained {{{3
    /// If hull will be subject to strain in the open sea.
    ///
    fn hull_strained(&self) -> bool {
        if self.str_comp() >= 0.5 && self.str_comp() < 0.885 && (
            self.engine.vmax < 24.0 || self.hull.d() > 4000.0)
        {
            true
        } else {
            false
        }
    }

    // is_steady {{{3
    /// If ship is a steady gun platform.
    ///
    fn is_steady(&self) -> bool {
        if self.steadiness() >= 69.5 {
            true
        } else {
            false
        }
    }

    // is_unsteady {{{3
    /// If ship is not a steady gun platform.
    ///
    fn is_unsteady(&self) -> bool {
        if self.steadiness() < 30.0 {
            true
        } else {
            false
        }
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
            s.push("Ship has slow easy roll, a good steady, gun platform".into());
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
                    }).into(),
            SeaType::Error   => "Invalid SeaType".into(),
        };

        s.push(sea);

        s
    }

    // roll_period {{{3
    /// Roll period of the ship.
    ///
    pub fn roll_period(&self) -> f64 {
        0.42 * self.hull.bb / self.metacenter().sqrt()
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
        let a =
            (self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())) * 5.0 +
            (self.wgt_borne() + self.wgt_gun_armor()) * (2.0 * self.gun_super_factor() - 1.0) * 4.0 +
            self.wgts.hull as f64 * 2.0 +
            self.wgts.on as f64 * 3.0 +
            self.wgts.above as f64 * 4.0 +
            self.armor.upper.wgt(self.hull.d(), self.hull.cwp(), self.hull.b) * 2.0 +
            self.armor.main.wgt(self.hull.d(), self.hull.cwp(), self.hull.b) +
            self.armor.end.wgt(self.hull.d(), self.hull.cwp(), self.hull.b) +
            // TODO: Replace with the following once the circular references are fixed:
            // self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine()) +
            self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), 0.0) +
            (self.wgt_hull_plus() + self.wgt_guns() + self.wgt_gun_mounts() - self.wgt_borne()) * 1.5 * self.hull.freeboard() / self.hull.t;

        let b = a +
            if self.deck_room() < 1.0 {
                (self.wgt_engine() + self.wgts.vital as f64 + self.wgts.void as f64) * (1.0 - self.deck_room().powf(2.0))
            } else { 0.0 };

        if b > 0.0 {
            ((self.hull.d() * (self.hull.bb / self.hull.t) / b) * 0.5).sqrt() *
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
        f64::min(
            self.hull.d() /
            (
                self.engine.d_engine(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()) +
                    8.0 * self.wgt_borne() + self.wgt_armor() + self.wgts.wgt() as f64
            ),
            10.0
        )
    }

    // cap_calc_broadside {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn cap_calc_broadside(&self) -> bool {
        for b in self.batteries.iter() {
            if ! b.broad_and_below() { return false; }
        }

        true
    }

    // flotation {{{3
    /// Estimate of the pounds of non-critical shell
    /// hits required to sink or destroy the ship.
    ///
    pub fn flotation(&self) -> f64 {
        let a = if self.cap_calc_broadside() {
                self.hull.free_cap(self.cap_calc_broadside())
            } else {
                self.hull.freeboard_dist()
            };

        let b = (a * self.hull.wp() / Hull::FT3_PER_TON_SEA + self.hull.d()) / 2.0;

        let c = b * self.stability_adj().powf(
            if self.stability_adj() > 1.0 { 0.5 } else { 4.0 }
            );

        let d = c * if self.str_comp() < 1.0 { self.str_comp() } else { 1.0 };

        let e = d / self.room().powf(if self.room() > 1.0 { 2.0 } else { 1.0 });

        f64::max(e * Self::year_adj(self.year), 0.0)
    }

    // str_cross {{{3
    /// Cross-sectional strength.
    ///
    pub fn str_cross(&self) -> f64 {
        let mut concentration: f64 = 1.0;

        if self.wgt_broad() > 0.0 {
            concentration = 1.0 + self.gun_concentration();
        }

        let mut str_cross = self.wgt_struct() / f64::sqrt(self.hull.bb * (self.hull.t + self.hull.freeboard_dist())) /
            ((self.hull.d() + ((self.wgt_broad() + self.wgt_borne() + self.wgt_gun_armor() + self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())) * (concentration * self.gun_super_factor()) + f64::max(self.engine.hp_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()), 0.0) / 100.0)) / self.hull.d()) * 0.6;

        if self.year < 1900 {
            str_cross *= 1.0 - (1900.0 - self.year as f64) / 100.0;
        }

        str_cross
    }

    // str_long {{{3
    /// Longitudinal strength.
    ///
    pub fn str_long(&self) -> f64 {
        (
            self.wgt_hull_plus() + match self.armor.bh_kind {
                BulkheadType::Strengthened =>
                    self.armor.bulkhead.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b),
                BulkheadType::Additional => 0.0,
            }
        ) /
            (
                (self.hull.lwl() / (self.hull.t + self.hull.free_cap(self.cap_calc_broadside()))).powf(2.0) *
                (
                    self.hull.d() +
                    self.armor.end.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) *
                    3.0 + (
                        self.wgt_borne() +
                        self.wgt_gun_armor()
                        ) * self.super_factor_long() * 2.0
                )
            ) *
            850.0 * if self.year < 1900 { 1 - (1900 - self.year) / 100 } else { 1 } as f64
    }

    // str_comp {{{3
    /// Composite strength.
    ///
    pub fn str_comp(&self) -> f64 {
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
            concentration += b.concentration(self.wgt_broad());
        }
        concentration
    }

    // damage_shell_size {{{3
    /// Size of shells used to calculate flotation().
    ///
    pub fn damage_shell_size(&self) -> f64 {
        if self.batteries[0].diam > 0.0 {
            self.batteries[0].diam
        } else {
            6.0
        }
    }

    // damage_shell_num {{{3
    /// Number of non-critical shell hits of the same caliber as the
    /// main battery or 6" shells if the ship has no main battery.
    ///
    pub fn damage_shell_num(&self) -> f64 {
        self.flotation() / (
            self.damage_shell_size().powf(3.0) /
            2.0 * Self::year_adj(self.year) as f64
            )
    }

    // damage_shell_torp_num {{{3
    /// Number of non-critical 20" torpedo hits required to sink the ship.
    ///
    pub fn damage_torp_num(&self) -> f64 {
        (
            (
                (
                    (self.flotation() / 10_000.0).powf(1.0/3.0) +
                    (self.hull.bb / 75.0).powf(2.0) +
                    (
                        (self.armor.bulkhead.thick / 2.0 * self.armor.bulkhead.len / self.hull.lwl()) /
                        0.65 * self.armor.bulkhead.hgt / self.hull.t
                    ).powf(1.0/3.0) *
                    self.flotation() / 35_000.0 * self.hull.bb / 50.0
                ) / self.room() * self.hull.lwl() / (self.hull.lwl() + self.hull.bb)
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
    fn wgt_engine(&self) -> f64 {

        let p =
            if (self.hull.d() < 5000.0) && (self.hull.d() >= 600.0) && (self.d_factor() < 1.0)
            {
                1.0 - self.hull.d() / 5000.0
            } else if (self.hull.d() < 600.0) && (self.d_factor() < 1.0) {
                    0.88
                } else {
                    0.0
            };

        (self.engine.d_engine(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()) / 2.0) *
            self.d_factor().powf(p)
    }

    // wgt_struct {{{3
    /// Weight per square feet of hull.
    ///
    pub fn wgt_struct(&self) -> f64 {
        (
            self.wgt_hull_plus() +
            match self.armor.bh_kind {
                BulkheadType::Strengthened => 
                    self.armor.bulkhead.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b),
                BulkheadType::Additional => 0.0,
            }
        ) * Self::POUND2TON / (
            self.hull.ws() +
            2.0 * self.hull.lwl() * self.hull.free_cap(self.cap_calc_broadside()) +
            self.hull.wp()
            )
    }

    // wgt_hull {{{3
    /// Weight of the hull.
    ///
    fn wgt_hull(&self) -> f64 {
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
    /// XXX: I do not know what this does
    ///
    fn wgt_borne(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.gun_wgt() * b.mount_kind.wgt_adj();
        }
        wgt * 2.0
    }

    // wgt_weaps {{{3
    /// Weight of torpedos, mines and ASW weapons
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
    fn wgt_guns(&self) -> f64 {
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
    fn wgt_gun_armor(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.armor_wgt(self.hull.clone());
        }
        wgt
    }

    // wgt_mag {{{3
    /// Weight of the ship's magazines.
    ///
    fn wgt_mag(&self) -> f64 {
        let mut wgt = 0.0;
        for b in self.batteries.iter() {
            wgt += b.mag_wgt();
        }
        wgt
    }

    // wgt_broad {{{3
    /// Sum of the broadside weights of all batteries.
    ///
    fn wgt_broad(&self) -> f64 {
        let mut broad = 0.0;
        for b in self.batteries.iter() {
            broad += b.broadside_wgt();
        }
        broad
    }

    // wgt_armor {{{3
    /// Weight of ship and battery armor.
    ///
    fn wgt_armor(&self) -> f64 {
        // TODO: Replace with the following once the circular references are fixed:
        // self.armor.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine()) + self.wgt_gun_armor()
        self.armor.wgt(self.hull.clone(), self.wgt_mag(), 0.0) + self.wgt_gun_armor()
    }

    // gun_wtf {{{3
    /// XXX: I do not know what this does.
    ///
    fn gun_wtf(&self) -> f64 {
        let mut wtf = 0.0;
        for b in self.batteries.iter() {
            if b.diam == 0.0 { continue; }
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
                    (self.batteries[0].groups[0].distribution.g1_gun_position(self.hull.fd_len, self.hull.ad_len()) -
                     self.batteries[0].groups[1].distribution.g2_gun_position(self.hull.fd_len, self.hull.ad_len())).abs() < 0.2
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
        format!("{} tons, {:.1} %", format_num!(",.0", portion),
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
            ()
        } else if line.contains("SpringSharp") {
            Err("SpringSharp file too old")?;
        } else {
            Err("Unknown file format")?;
        }

        ship.name    = lines.next().unwrap();
        ship.country = lines.next().unwrap();
        ship.kind    = lines.next().unwrap();

        ship.hull.units     = lines.next().unwrap().into();
        for b in ship.batteries.iter_mut() { b.units = lines.next().unwrap().into(); }
        ship.torps[0].units = lines.next().unwrap().into();
        ship.armor.units    = lines.next().unwrap().into();

        ship.year = lines.next().unwrap().parse()?;

        ship.wgts.vital = lines.next().unwrap().parse()?;

        ship.hull.set_lwl(lines.next().unwrap().parse()?);
        ship.hull.b          = lines.next().unwrap().parse()?;
        ship.hull.t          = lines.next().unwrap().parse()?;
        ship.hull.stern_type = lines.next().unwrap().into();
        ship.hull.set_cb(lines.next().unwrap().parse()?);

        ship.hull.qd_aft         = lines.next().unwrap().parse()?;
        ship.hull.stern_overhang = lines.next().unwrap().parse()?;
        ship.hull.qd_len         = lines.next().unwrap().parse()?;
        ship.hull.qd_len /= 100.0; // convert from % to decimal
        ship.hull.qd_fwd         = lines.next().unwrap().parse()?;
        ship.hull.ad_aft         = lines.next().unwrap().parse()?;
        ship.hull.fd_len         = lines.next().unwrap().parse()?;
        ship.hull.fd_len /= 100.0; // convert from % to decimal
        ship.hull.ad_fwd         = lines.next().unwrap().parse()?;
        ship.hull.fd_aft         = lines.next().unwrap().parse()?;
        ship.hull.fc_len         = lines.next().unwrap().parse()?;
        ship.hull.fc_len /= 100.0; // convert from % to decimal
        ship.hull.fd_fwd         = lines.next().unwrap().parse()?;
        ship.hull.fc_aft         = lines.next().unwrap().parse()?;
        ship.hull.fc_fwd         = lines.next().unwrap().parse()?;
        ship.hull.bow_angle      = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() {
            b.num             = lines.next().unwrap().parse()?;
            b.diam             = lines.next().unwrap().parse()?;
            b.kind            = lines.next().unwrap().into();
            b.groups[0].above = lines.next().unwrap().parse()?;
            b.groups[0].below = lines.next().unwrap().parse()?;

            // Have to remove the commas from the string or it fails
            // to convert to a float
            b.set_shell_wgt( lines.next().unwrap().replace(",", "").parse()? );
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
        ship.torps[0].diam = lines.next().unwrap().parse()?;

        ship.armor.main.thick = lines.next().unwrap().parse()?;
        ship.armor.main.len   = lines.next().unwrap().parse()?;
        ship.armor.main.hgt   = lines.next().unwrap().parse()?;

        ship.armor.end.thick = lines.next().unwrap().parse()?;
        ship.armor.end.len   = lines.next().unwrap().parse()?;
        ship.armor.end.hgt   = lines.next().unwrap().parse()?;

        ship.armor.upper.thick = lines.next().unwrap().parse()?;
        ship.armor.upper.len   = lines.next().unwrap().parse()?;
        ship.armor.upper.hgt   = lines.next().unwrap().parse()?;

        ship.armor.bulkhead.thick = lines.next().unwrap().parse()?;
        ship.armor.bulkhead.len   = lines.next().unwrap().parse()?;
        ship.armor.bulkhead.hgt   = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() {
            b.armor_face = lines.next().unwrap().parse()?;
            b.armor_back = lines.next().unwrap().parse()?;
            b.armor_barb = lines.next().unwrap().parse()?;
        }

        ship.armor.deck.md      = lines.next().unwrap().parse()?;
        ship.armor.ct_fwd.thick = lines.next().unwrap().parse()?;
        ship.engine.vmax        = lines.next().unwrap().parse()?;
        ship.engine.vcruise     = lines.next().unwrap().parse()?;
        ship.engine.range       = lines.next().unwrap().parse()?;
        ship.engine.set_shafts(lines.next().unwrap().parse()?, &mut ship.hull);
        ship.engine.pct_coal    = lines.next().unwrap().parse()?;
        ship.engine.pct_coal /= 100.0; // convert from % to decimal

        ship.engine.fuel = FuelType::empty();
        match lines.next().unwrap().as_str() { "True" => ship.engine.fuel.toggle(FuelType::Coal), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.fuel.toggle(FuelType::Oil), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.fuel.toggle(FuelType::Diesel), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.fuel.toggle(FuelType::Gasoline), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.fuel.toggle(FuelType::Battery), _ => (), };

        ship.engine.boiler = BoilerType::empty();
        match lines.next().unwrap().as_str() { "True" => ship.engine.boiler.toggle(BoilerType::Simple), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.boiler.toggle(BoilerType::Complex), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.boiler.toggle(BoilerType::Turbine), _ => (), };

        ship.engine.drive = DriveType::empty();
        match lines.next().unwrap().as_str() { "True" => ship.engine.drive.toggle(DriveType::Direct), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.drive.toggle(DriveType::Geared), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.drive.toggle(DriveType::Electric), _ => (), };
        match lines.next().unwrap().as_str() { "True" => ship.engine.drive.toggle(DriveType::Hydraulic), _ => (), };

        ship.trim        = lines.next().unwrap().parse()?;
        ship.hull.bb     = lines.next().unwrap().parse()?;
        ship.engine.year = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() { b.year = lines.next().unwrap().parse()?; }

        ship.hull.bow_type = lines.next().unwrap().into();
        let ram_len        = lines.next().unwrap().parse()?;
        ship.hull.bow_type = match ship.hull.bow_type {
            BowType::Ram(_) => BowType::Ram(ram_len),
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
        for b in ship.batteries.iter_mut() { b.groups[1].two_mounts_up = match lines.next().unwrap().as_str() { "True" => true, _ => false, }; }
        for b in ship.batteries.iter_mut() { b.groups[1].on            = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].below         = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].lower_deck    = match lines.next().unwrap().as_str() { "True" => true, _ => false, }; }

        ship.torps[0].mounts     = lines.next().unwrap().parse()?;
        ship.torps[1].mounts     = lines.next().unwrap().parse()?;
        ship.torps[1].diam       = lines.next().unwrap().parse()?;
        ship.torps[0].len        = lines.next().unwrap().parse()?;
        ship.torps[1].len        = lines.next().unwrap().parse()?;
        ship.torps[0].mount_kind = lines.next().unwrap().into();
        ship.torps[1].mount_kind = lines.next().unwrap().into();

        ship.mines.num        = lines.next().unwrap().parse()?;
        ship.mines.reload     = lines.next().unwrap().parse()?;
        ship.mines.wgt        = lines.next().unwrap().parse()?;
        ship.mines.mount_kind = lines.next().unwrap().into();

        ship.asw[0].num    = lines.next().unwrap().parse()?;
        ship.asw[1].num    = lines.next().unwrap().parse()?;
        ship.asw[0].reload = lines.next().unwrap().parse()?;
        ship.asw[1].reload = lines.next().unwrap().parse()?;
        ship.asw[0].wgt    = lines.next().unwrap().parse()?;
        ship.asw[1].wgt    = lines.next().unwrap().parse()?;
        ship.asw[0].kind   = lines.next().unwrap().into();
        ship.asw[1].kind   = lines.next().unwrap().into();

        ship.wgts.hull  = lines.next().unwrap().parse()?;
        ship.wgts.on    = lines.next().unwrap().parse()?;
        ship.wgts.above = lines.next().unwrap().parse()?;

        ship.armor.incline               = lines.next().unwrap().parse()?;
        ship.armor.bulge.thick           = lines.next().unwrap().parse()?;
        ship.armor.bulge.len             = lines.next().unwrap().parse()?;
        ship.armor.bulge.hgt             = lines.next().unwrap().parse()?;

        ship.armor.bh_kind =
            match lines.next().unwrap().parse()? {
                0 => BulkheadType::Additional,
                1 | _ => BulkheadType::Strengthened,
            };

        ship.armor.bh_beam               = lines.next().unwrap().parse()?;
        ship.armor.deck.fc               = lines.next().unwrap().parse()?;
        ship.armor.deck.qd               = lines.next().unwrap().parse()?;
        ship.armor.deck.kind             = lines.next().unwrap().into();
        ship.armor.ct_aft.thick          = lines.next().unwrap().parse()?;

        for b in ship.batteries.iter_mut() { b.groups[0].above  = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[0].below  = lines.next().unwrap().parse()?; }
        for b in ship.batteries.iter_mut() { b.groups[1].above  = lines.next().unwrap().parse()?; }
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
                b.groups[1].above - b.groups[1].on - b.groups[1].below;
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
        if version.version == 1 { // No special handling required
            ()
        } else { // Cannot open any other versions
            let err = format!("Cannot open ship files of this version: {}!", version.version);
            return Err(err.into())
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
        let _ = OpenOptions::new().write(true).truncate(true).open(&p)?;
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

        let main_no_back = main.armor_face > 0.0;
        let sec_no_back  = sec.armor_face > 0.0;
        let ter_no_back  = ter.armor_face > 0.0;

        let main_broad_no_back = main_broad && main_no_back;
        let sec_broad_no_back  = sec_broad && sec_no_back;
        let ter_broad_no_back  = ter_broad && ter_no_back;

        let has_belt = (
            self.armor.main.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) +
            self.armor.end.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) +
            self.armor.upper.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b)
        ) > 0.0;

        if main_broad || sec_broad || ter_broad {
            if has_belt {
                if main_broad_no_back || sec_broad_no_back || ter_broad_no_back {
                    s.push("Armoured Casemate Ship".into());
                } else if self.hull.fc_len + self.hull.fd_len < 0.5 {
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

// Report {{{2
// addto {{{3
/// Pass arguments to format!() and push to a Vec<String>.
///
macro_rules! addto {
    ($r:ident,$($tts:tt)*) => {
        $r.push(format!($($tts)*))
    };
    ($r:ident) => {
        $r.push("".to_string())
    };
}

// addif {{{3
/// Return a formatted string if the condition is true.
/// Otherwise return an empty string.
///
macro_rules! addif {
    ($cond:expr, $($tts:tt)*) => {
        if $cond {
            format!($($tts)*)
        } else {
            "".into()
        }
    }
}

// num {{{3
/// Format a number with commas and the specified number of
/// significant digits.
///
// This is a macro instead of a function to avoid having to cast
// floats to ints or ints to floats
macro_rules! num {
    ($val:expr, $digits: expr) => {
        format_num!(&*format!(",.{}", $digits), $val)
    }
}

// plural {{{3
/// Return an "s" is num is anything other than 1.
///
fn plural(num: u32) -> String {
    match num { 1 => "".to_string(), _ => "s".to_string() }
}

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
        if self.hull.d() < (self.wgt_broad() / 4.0)
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
            num!(self.d_lite(), 0),
            num!(self.d_std(), 0),
            num!(self.hull.d(), 0),
            num!(self.d_max(), 0)
        );
        addto!(r);

        addto!(r, "Dimensions: Length (overall / waterline) x beam x draught (normal/deep)"); // {{{5
        addto!(r, "    ({:.2} ft / {:.2} ft) x {:.2} ft {}x ({:.2} / {:.2} ft)",
            self.hull.loa(),
            self.hull.lwl(),
            self.hull.b,
            addif!(self.hull.bb > self.hull.b, "(Bulges {:.2} ft) ", self.hull.bb),
            self.hull.t,
            self.t_max()
        );
        addto!(r, "    ({:.2} m / {:.2} m) x {:.2} m {}x ({:.2} / {:.2} m)",
            metric(self.hull.loa(), LengthLong, self.hull.units),
            metric(self.hull.lwl(), LengthLong, self.hull.units),
            metric(self.hull.b, LengthLong, self.hull.units),
            addif!(self.hull.bb > self.hull.b, "(Bulges {:.2} m) ", metric(self.hull.bb, LengthLong, self.hull.units)),
            metric(self.hull.t, LengthLong, self.hull.units),
            metric(self.t_max(), LengthLong, self.hull.units)
        );
        addto!(r);

        addto!(r, "Armament:"); // {{{5
        for (i, b) in self.batteries.iter().enumerate() {
            let main_gun = i == 0;

            if b.num == 0 { continue; }
            addto!(r, "    {} - {:.2}\" / {} mm {:.1} cal gun{} - {}lbs / {}kg shells, {} per gun",
                b.num,
                b.diam,
                num!(metric(b.diam, LengthSmall, b.units), if b.diam * 25.4 < 100.0 { 1 } else { 0 }),
                b.len,
                plural(b.num),
                num!(b.shell_wgt(), 2),
                num!(metric(b.shell_wgt(), Weight, b.units), 2),
                num!(b.shells, 0),
            );
            addto!(r, "        {} gun{} in {} mount{}, {} Model",
                b.kind,
                plural(b.num),
                b.mount_kind,
                plural(b.num),
                b.year
            );

            for (i, sb) in b.groups.iter().enumerate() {
                let sb_super = match i {
                    0 => sb.above < (b.mount_num - b.groups[1].above),
                    // TODO: SpringSharp BUG. Correct line is the below commented line:
                    // 1 => sb.above < (b.mount_num - b.groups[0].above),
                    _ => sb.above < (2 * sb.num_mounts() - sb.above),
                };

                if sb.num_mounts() == 0 { continue; }
                addto!(r, "        {} x {} mount{} on {}",
                    sb.num_mounts(),
                    sb.layout,
                    plural(sb.num_mounts()),
                    sb.distribution.desc(sb.num_mounts(), self.hull.fc_len + self.hull.fd_len)
                );
                if sb.above > 0 {
                    addto!(r, "        {} {}raised mount{}{}",
                        sb.above,
                        match sb.two_mounts_up { true => "double ", false => "", },
                        if sb.above > 1 { "s" } else if sb.distribution.super_aft() && main_gun { " aft" } else { "" },
                        if sb_super {
                            match sb.distribution {
                                GunDistributionType::CenterlineEven |
                                GunDistributionType::CenterlineFD |
                                GunDistributionType::CenterlineAD |
                                GunDistributionType::SidesEven |
                                GunDistributionType::SidesFD |
                                GunDistributionType::SidesAD => "",

                                _ => match b.mount_kind {
                                    MountType::Broadside => "",
                                    MountType::ColesTurret => "",

                                    _ => " - superfiring",
                                    },
                            }
                        } else {
                            ""
                        }
                    );
                }

                if sb.below > 0 {
                    addto!(r, "        {} hull mount{} {}- Limited use in {}",
                        sb.below,
                        if sb.above > 1 { "s" } else if sb.distribution.super_aft() && main_gun { " aft" } else { "" },
                        if b.mount_kind == MountType::Broadside {
                            (match sb.lower_deck { true => "on gundeck", false => "on upperdeck", }).into()
                        } else {
                            format!("in {}casemate{}",
                                addif!(sb.lower_deck, "{}", "lower "),
                                plural(sb.below),
                            )
                        },
                        if b.free(self.hull.clone()) < 12.0 ||
                            (b.free(self.hull.clone()) < 19.0 && sb.lower_deck)
                        {
                            "any sea"
                        } else if b.free(self.hull.clone()) < 16.0 ||
                            (b.free(self.hull.clone()) < 24.0 && sb.lower_deck)
                        {
                            "all but light seas"
                        } else {
                            "heavy seas"
                        }
                    );
                }
            }
        }
        addto!(r, "    Weight of broadside {} lbs / {} kg",
            num!(self.wgt_broad(), 0),
            num!(metric(self.wgt_broad(), Weight, Imperial), 0),
        );

        // Weapons {{{5
        for (i, torp) in self.torps.iter().enumerate() {
            if torp.num == 0 { continue; }

            addto!(r, "{} Torpedoes",
                match i { 0 => "Main", 1 => "2nd", _ => "Other", }
            );
            addto!(r, "{} - {:.1}\" / {:.0} mm, {:.2} ft / {:.2} m torpedo{} {:.3} t total",
                torp.num,
                torp.diam,
                metric(torp.diam, LengthSmall, torp.units),
                torp.len,
                metric(torp.len, LengthLong, torp.units),
                match torp.num {
                    1 => " -".to_string(),
                    _ => format!("es - {:.3} t each,", torp.wgt_weaps() / torp.num as f64),
                },
                torp.wgt_weaps()
            );
            addto!(r, "    {}",
                torp.mount_kind.desc(torp.num, torp.mounts)
            );
        }

        if self.mines.num != 0 {
            addto!(r, "Mines");
            addto!(r, "{} - {:.2} lbs / {:.2} kg mines{} - {:.3} t total",
                self.mines.num,
                self.mines.wgt,
                metric(self.mines.wgt, Weight, self.mines.units),
                addif!(self.mines.reload > 0, " + {} reloads", self.mines.reload),
                self.mines.wgt_weaps()
            );
            addto!(r, "    {}",
                self.mines.mount_kind.desc()
            );
        }

        for (i, asw) in self.asw.iter().enumerate() {
            if asw.num == 0 { continue; }

            addto!(r, "{} DC/AS Mortars",
                match i { 0 => "Main", 1 => "2nd", _ => "Other", }
            );
            addto!(r, "{} - {:.2} lbs / {:.2} kg {}{} - {:.3} t total",
                asw.num,
                asw.wgt,
                metric(asw.wgt, Weight, asw.units),
                asw.kind.desc(),
                addif!(asw.reload > 0, " + {} reloads", asw.reload),
                asw.wgt_weaps()
            );
            if asw.kind.dc_desc() != "" {
                addto!(r, "    {}",
                    asw.kind.dc_desc()
                );
            }
        }

        // Armor {{{5
        addto!(r);
        addto!(r, "Armour:");

        if self.armor.main.thick + self.armor.end.thick + self.armor.upper.thick + self.armor.bulkhead.thick > 0.0 {
            addto!(r, " - Belts:    Width (max)    Length (avg)    Height (avg)");
            if self.armor.main.thick > 0.0 {
                addto!(r, "    Main:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.main.thick, if self.armor.main.thick < 10.0 { 2 } else { 1 }),
                    metric(self.armor.main.thick, LengthSmall, self.armor.units),
                    self.armor.main.len,
                    metric(self.armor.main.len, LengthLong, self.armor.units),
                    self.armor.main.hgt,
                    metric(self.armor.main.hgt, LengthLong, self.armor.units),
                );
            }

            if self.armor.end.thick > 0.0 {
                addto!(r, "    Ends:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.end.thick, if self.armor.end.thick < 10.0 { 2 } else { 1 }),
                    metric(self.armor.end.thick, LengthSmall, self.armor.units),
                    self.armor.end.len,
                    metric(self.armor.end.len, LengthLong, self.armor.units),
                    self.armor.end.hgt,
                    metric(self.armor.end.hgt, LengthLong, self.armor.units),
                );
                if self.armor.main.len + self.armor.end.len < self.hull.lwl() {
                    addto!(r, "    {:.2} ft / {:.2} m Unarmoured ends",
                        self.hull.lwl() - self.armor.main.len - self.armor.end.len,
                        metric(self.hull.lwl() - self.armor.main.len - self.armor.end.len, LengthLong, self.armor.units)
                    );
                }
            } else if self.armor.main.len < self.hull.lwl() {
                addto!(r, "    Ends:    Unarmoured");
            }

            if self.armor.upper.thick > 0.0 {
                addto!(r, "    Upper:    {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.upper.thick, if self.armor.upper.thick < 10.0 { 2 } else { 1 }),
                    metric(self.armor.upper.thick, LengthSmall, self.armor.units),
                    self.armor.upper.len,
                    metric(self.armor.upper.len, LengthLong, self.armor.units),
                    self.armor.upper.hgt,
                    metric(self.armor.upper.hgt, LengthLong, self.armor.units),
                );
            }

            if self.armor.main.thick > 0.0 {
                addto!(r, "    Main Belt covers {:.0} % of normal length",
                    self.armor.belt_coverage(self.hull.lwl())*100.0
                );
                if self.armor.belt_coverage(self.hull.lwl()) < self.hull_room() {
                    addto!(r, "    Main belt does not fully cover magazines and engineering spaces");
                }
            }

            if self.armor.incline != 0.0 {
                addto!(r, "    Main Belt inclined {:.2} degrees (positive = in)",
                    self.armor.incline
                );
            }

            if self.armor.bulkhead.thick > 0.0 {
                addto!(r);
                addto!(r, "- Torpedo Bulkhead - {} bulkheads:",
                    match self.armor.bh_kind {
                        BulkheadType::Strengthened => "Strengthened structural",
                        BulkheadType::Additional   => "Additional damage containing",
                    }
                );
                addto!(r, "        {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.bulkhead.thick, if self.armor.bulkhead.thick < 10.0 { 2 } else { 1 }),
                    metric(self.armor.bulkhead.thick, LengthSmall, self.armor.units),
                    self.armor.bulkhead.len,
                    metric(self.armor.bulkhead.len, LengthLong, self.armor.units),
                    self.armor.bulkhead.hgt,
                    metric(self.armor.bulkhead.hgt, LengthLong, self.armor.units),
                );
                addto!(r, "    Beam between torpedo bulkheads {:.2} ft / {:.2} m",
                    self.armor.bh_beam,
                    metric(self.armor.bh_beam, LengthLong, self.armor.units)
                );
                addto!(r);
            }

            if self.armor.bulge.thick > 0.0 || self.wgts.void > 0 {
                addto!(r, "- Hull {}:",
                    if self.hull.b == self.hull.bb { "void" }
                    else { "Bulges" }
                );
                addto!(r, "        {}\" / {:.0} mm    {:.2} ft / {:.2} m    {:.2} ft / {:.2} m",
                    num!(self.armor.bulge.thick, if self.armor.bulge.thick < 10.0 { 2 } else { 1 }),
                    metric(self.armor.bulge.thick, LengthSmall, self.armor.units),
                    self.armor.bulge.len,
                    metric(self.armor.bulge.len, LengthLong, self.armor.units),
                    self.armor.bulge.hgt,
                    metric(self.armor.bulge.hgt, LengthLong, self.armor.units),
                );
            addto!(r);
            }
        }

        if self.wgt_gun_armor() > 0.0 {
            addto!(r, "- Gun armour:    Face (max)    Other gunhouse (avg)    Barbette/hoist (max)");

            for (i, b) in self.batteries.iter().enumerate() {
                if b.armor_face == 0.0 &&
                b.armor_back == 0.0 &&
                b.armor_barb == 0.0 { continue; }
                addto!(r, "    {}:    {}        {}            {}",
                    match i { 0 => "Main", 1 => "2nd", 2 => "3rd", 3 => "4th", 4 => "5th", _ => "Other", },
                    if b.armor_face == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_face, if b.armor_face >= 10.0 { 1 } else { 2 }), metric(b.armor_face, LengthSmall, b.units)) },
                    if b.armor_back == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_back, if b.armor_back >= 10.0 { 1 } else { 2 }), metric(b.armor_back, LengthSmall, b.units)) },
                    if b.armor_barb == 0.0 { "-".into() } else { format!("{}\" / {:.0} mm", num!(b.armor_barb, if b.armor_barb >= 10.0 { 1 } else { 2 }), metric(b.armor_barb, LengthSmall, b.units)) },
                );
            }
            addto!(r);
        }

        if self.armor.deck.fc + self.armor.deck.md + self.armor.deck.qd > 0.0 {
            addto!(r, "- {}:",
                self.armor.deck.kind
            );
            // TODO: Change spelling to Fore (required to match Springsharp reports)
            addto!(r, "    For and Aft decks: {:.2}\" / {:.0} mm",
                self.armor.deck.md,
                metric(self.armor.deck.md, LengthSmall, self.armor.units)
            );
            // TODO: Change spelling to Quarterdeck (required to match Springsharp reports)
            addto!(r, "    Forecastle: {:.2}\" / {:.0} mm    Quarter deck: {:.2}\" / {:.0} mm",
                self.armor.deck.fc,
                metric(self.armor.deck.fc, LengthSmall, self.armor.units),
                self.armor.deck.qd,
                metric(self.armor.deck.qd, LengthSmall, self.armor.units)
            );
            addto!(r);
        }

        if self.armor.ct_fwd.thick + self.armor.ct_aft.thick > 0.0 {
            // TODO: Remove stray space before comma (required to match Springsharp reports)
            addto!(r, "- Conning towers: Forward {:.2}\" / {:.0} mm, Aft {:.2}\" / {:.0} mm",
                self.armor.ct_fwd.thick,
                metric(self.armor.ct_fwd.thick, LengthSmall, self.armor.units),
                self.armor.ct_aft.thick,
                metric(self.armor.ct_aft.thick, LengthSmall, self.armor.units)
            );
            addto!(r);
        }

        addto!(r, "Machinery:"); // {{{5
        if self.engine.vmax != 0.0 {
            addto!(r, "    {}, {},",
                self.engine.fuel,
                self.engine.boiler
            );
            addto!(r, "    {}, {} shaft{}, {} {} / {} Kw = {:.2} kts",
                self.engine.drive,
                self.engine.shafts(),
                plural(self.engine.shafts()),
                num!(self.engine.hp_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()), 0),
                self.engine.boiler.hp_type(),
                num!(metric(self.engine.hp_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()), Power, Imperial), 0),
                self.engine.vmax
            );
            addto!(r, "    Range {}nm at {:.2} kts",
                num!(self.engine.range, 0),
                self.engine.vcruise
            );
            addto!(r, "    Bunker at max displacement = {} tons{}",
                num!(self.engine.bunker_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()), 0),
                if self.engine.pct_coal > 0.0 { format!(" ({:.0}% coal)", self.engine.pct_coal * 100.0) } else { "".into() }
            );
            let ratio = self.engine.hp_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()) / self.engine.shafts() as f64;

            if ratio > 20_000.0 && self.engine.boiler.is_reciprocating()
                { addto!(r, "    Caution: Too much power for reciprocating engines."); }
            else if ratio > 75_000.0
                { addto!(r, "    Caution: Too much power for number of propellor shafts."); }

            if self.wgt_engine() < self.engine.d_engine(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws()) / 5.0 {
                addto!(r, "    Caution: Delicate, lightweight machinery.");
            }

        } else {
            addto!(r, "    Immobile floating battery");
        }
        addto!(r);

        addto!(r, "Complement:"); // {{{5
        addto!(r, "    {} - {}",
            self.crew_min(),
            self.crew_max()
        );
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

        if self.torps[0].wgt() + self.torps[1].wgt() + self.mines.wgt() + self.asw[0].wgt() + self.asw[1].wgt > 0.0 {
            addto!(r, "    - Weapons: {}",
                self.percent_calc(self.torps[0].wgt() + self.torps[1].wgt() + self.mines.wgt() + self.asw[0].wgt() + self.asw[1].wgt()),
            );
        }

        if self.wgt_armor() > 0.0 {
            addto!(r, "    Armour: {}",
                self.percent_calc(self.wgt_armor()),
            );

            if self.armor.main.thick + self.armor.end.thick + self.armor.upper.thick > 0.0 {
                addto!(r, "    - Belts: {}",
                    self.percent_calc(self.armor.main.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) +
                        self.armor.end.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) +
                        self.armor.upper.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b)),
                );
            }

            if self.armor.bulkhead.thick > 0.0 {
                addto!(r, "    - Torpedo bulkhead: {}",
                    self.percent_calc(self.armor.bulkhead.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b)),
                );
            }

            if self.armor.bulge.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b) > 0.0 {
                addto!(r, "    - {}: {}",
                    if self.hull.b == self.hull.bb { "Void" } else { "Bulges" },
                    self.percent_calc(self.armor.bulge.wgt(self.hull.lwl(), self.hull.cwp(), self.hull.b)),
                );
            }

            if self.wgt_gun_armor() > 0.0 {
                addto!(r, "    - Armament: {}",
                    self.percent_calc(self.wgt_gun_armor()),
                );
            }

            if self.armor.deck.fc + self.armor.deck.md + self.armor.deck.qd > 0.0 {
                addto!(r, "    - Armour Deck: {}",
                    // TODO: Replace with the following once the circular references are fixed:
                    // self.percent_calc(self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine())),
                    self.percent_calc(self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), 0.0)),
                );
            }

            if self.armor.ct_fwd.thick + self.armor.ct_aft.thick > 0.0 {
                addto!(r, "    - Conning Tower{}: {}",
                    if self.armor.ct_fwd.thick > 0.0 && self.armor.ct_aft.thick > 0.0 {
                        "s"
                    } else { "" },
                    self.percent_calc(self.armor.ct_fwd.wgt(self.hull.d()) + self.armor.ct_aft.wgt(self.hull.d())),
                );
            }
        }

        addto!(r, "    Machinery: {}",
            self.percent_calc(self.wgt_engine()),
        );
        addto!(r, "    Hull, fittings & equipment: {}",
            self.percent_calc(self.wgt_hull()),
        );
        addto!(r, "    Fuel, ammunition & stores: {}",
            self.percent_calc(self.wgt_load()),
        );

        if self.wgts.wgt() > 0 {
            addto!(r, "    Miscellaneous weights: {}",
                self.percent_calc(self.wgts.wgt() as f64),
            );
            if self.wgts.vital > 0 { addto!(r, "    - Hull below water: {} tons", 
                    num!(self.wgts.vital, 0)
            ); }
            if self.wgts.void > 0 {
                addto!(r, "    - {} void weights: {} tons",
                    if self.hull.bb > self.hull.b { "Bulge" } else { "Hull" },
                    num!(self.wgts.void, 0),
                );
            }
            if self.wgts.hull > 0  { addto!(r, "    - Hull above water: {:.0} tons", self.wgts.hull) };
            if self.wgts.on > 0    { addto!(r, "    - On freeboard deck: {:.0} tons", self.wgts.on) };
            if self.wgts.above > 0 { addto!(r, "    - Above deck: {:.0} tons", self.wgts.above) };
        }

        addto!(r);

        addto!(r, "Overall survivability and seakeeping ability:"); // {{{5
        addto!(r, "    Survivability (Non-critical penetrating hits needed to sink ship):");
        addto!(r, "    {:.0} lbs / {:.0} Kg = {:.1} x {:.1} \" / {:.0} mm shells or {:.1} torpedoes",
            self.flotation(),
            metric(self.flotation(), Weight, Imperial),
            self.damage_shell_num(),
            self.damage_shell_size(),
            metric(self.damage_shell_size(), LengthSmall, Imperial),
            self.damage_torp_num()
        );
        addto!(r, "    Stability (Unstable if below 1.00): {:.2}",
            self.stability_adj()
        );
        addto!(r, "    Metacentric height {:.1} ft / {:.1} m",
            self.metacenter(),
            metric(self.metacenter(), LengthLong, Imperial)
        );
        addto!(r, "    Roll period: {:.1} seconds",
            self.roll_period()
        );
        addto!(r, "    Steadiness    - As gun platform (Average = 50 %): {:.0} %",
            self.steadiness()
        );
        addto!(r, "        - Recoil effect (Restricted arc if above 1.00): {:.2}",
            self.recoil()
        );
        addto!(r, "    Seaboat quality (Average = 1.00): {:.2}",
            self.seakeeping()
        );
        addto!(r);

        addto!(r, "Hull form characteristics:"); // {{{5
        addto!(r, "    Hull has {},",
            self.hull.freeboard_desc()
        );
        addto!(r, "    {} and {}",
            self.hull.bow_type,
            self.hull.stern_type
        );
        addto!(r, "    Block coefficient (normal/deep): {:.3} / {:.3}",
            self.hull.cb(), self.cb_max()
        );
        addto!(r, "    Length to Beam Ratio: {:.2} : 1",
            self.hull.len2beam()
        );
        addto!(r, "    'Natural speed' for length: {:.2} kts",
            self.hull.vn()
        );
        addto!(r, "    Power going to wave formation at top speed: {:.0} %",
            self.engine.pw_max(self.hull.d(), self.hull.lwl(), self.hull.cs(), self.hull.ws()) * 100.0
        );
        addto!(r, "    Trim (Max stability = 0, Max steadiness = 100): {}",
            self.trim
        );
        addto!(r, "    Bow angle (Positive = bow angles forward): {:.2} degrees",
            self.hull.bow_angle
        );
        addto!(r, "    Stern overhang: {:.2} ft / {:.2} m",
            self.hull.stern_overhang,
            metric(self.hull.stern_overhang, LengthLong, self.hull.units)
        );
        addto!(r, "    Freeboard (% = length of deck as a percentage of waterline length):"
        );
        addto!(r, "            Fore end, Aft end");
        addto!(r, "    - Forecastle:    {:.2} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            self.hull.fc_len*100.0,   self.hull.fc_fwd, metric(self.hull.fc_fwd, LengthLong, self.hull.units), self.hull.fc_aft, metric(self.hull.fc_aft, LengthLong, self.hull.units)
        );
        addto!(r, "    - Forward deck:    {:.2} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            self.hull.fd_len*100.0,   self.hull.fd_fwd, metric(self.hull.fd_fwd, LengthLong, self.hull.units), self.hull.fd_aft, metric(self.hull.fd_aft, LengthLong, self.hull.units)
        );
        addto!(r, "    - Aft deck:    {:.2} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            self.hull.ad_len()*100.0, self.hull.ad_fwd, metric(self.hull.ad_fwd, LengthLong, self.hull.units), self.hull.ad_aft, metric(self.hull.ad_aft, LengthLong, self.hull.units)
        );
        addto!(r, "    - Quarter deck:    {:.2} %, {:.2} ft / {:.2} m, {:.2} ft / {:.2} m",
            self.hull.qd_len*100.0,   self.hull.qd_fwd, metric(self.hull.qd_fwd, LengthLong, self.hull.units), self.hull.qd_aft, metric(self.hull.qd_aft, LengthLong, self.hull.units)
        );
        addto!(r, "    - Average freeboard:        {:.2} ft / {:.2} m",
            self.hull.freeboard(), metric(self.hull.freeboard(), LengthLong, self.hull.units)
        
        );
        if self.hull.is_wet_fwd() {
            addto!(r, "    Ship tends to be wet forward");
        }
        addto!(r);

        addto!(r, "Ship space, strength and comments:"); // {{{5
        addto!(r, "    Space    - Hull below water (magazines/engines, low = better): {:.1} %",
            self.hull_room() * 100.0
        );
        addto!(r, "        - Above water (accommodation/working, high = better): {:.1} %",
            self.deck_room() * 100.0
        );
        addto!(r, "    Waterplane Area: {} Square feet or {} Square metres",
            num!(self.hull.wp(), 0),
            num!(metric(self.hull.wp(), Area, Imperial), 0)
        );
        addto!(r, "    Displacement factor (Displacement / loading): {:.0} %",
            self.d_factor() * 100.0
        );
        addto!(r, "    Structure weight / hull surface area: {:.0} lbs/sq ft or {:.0} Kg/sq metre",
            self.wgt_struct(),
            metric(self.wgt_struct(), WeightPerArea, Imperial)

            
        );
        addto!(r, "Hull strength (Relative):");
        addto!(r, "        - Cross-sectional: {:.2}",
            self.str_cross()
        );
        addto!(r, "        - Longitudinal: {:.2}",
            self.str_long()
        );
        addto!(r, "        - Overall: {:.2}",
            self.str_comp()
        );

        if self.tender_warn() && !self.capsize_warn() {
            addto!(r, "Caution: Poor stability - excessive risk of capsizing");
        }
        if self.hull_strained() {
            addto!(r, "Caution: Hull subject to strain in open-sea");
        }
        addto!(r, "    {} machinery, storage, compartmentation space",
            self.hull_room_quality()
        );
        addto!(r, "    {} accommodation and workspace room",
            self.deck_room_quality()
        );
        for s in self.seakeeping_desc() {
            addto!(r, "    {}", s
            );
        }

        addto!(r);

        // Custom Notes {{{5
        for s in self.notes.iter() {
            addto!(r, "{}", s);
        }

        r.join("\n")
    }
}

// Inernals Output {{{2
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
            b.internals(self.hull.clone(), self.wgt_broad());
            s.push("".to_string());
        }

        s.push(format!("Cs = {}", self.hull.cs()));
        s.push(format!("Cm = {}", Hull::cm(self.hull.cb())));
        s.push(format!("Cp = {}", Hull::cp(self.hull.cb())));
        s.push(format!("Cwp = {}", self.hull.cwp()));
        s.push(format!("WP = {}", self.hull.wp()));
        s.push(format!("WS = {}", self.hull.ws()));
        s.push(format!("Ts = {}", self.hull.ts()));
        s.push("".to_string());
        s.push(format!("Stem length = {}", self.hull.stem_len()));
        if let BowType::Ram(len) = self.hull.bow_type { s.push(format!("Ram length = {}", len)); }
        s.push(format!("Freeboard dist = {}", self.hull.freeboard_dist()));
        s.push(format!("Leff = {}", self.hull.leff()));
        s.push("".to_string());
        s.push(format!("Rf max = {}", self.engine.rf_max(self.hull.ws())));
        s.push(format!("Rf cruise = {}", self.engine.rf_cruise(self.hull.ws())));
        s.push(format!("Rw max = {}", self.engine.rw_max(self.hull.d(), self.hull.lwl(), self.hull.cs())));
        s.push(format!("Rw cruise = {}", self.engine.rw_cruise(self.hull.d(), self.hull.lwl(), self.hull.cs())));
        s.push(format!("Pw max = {}", self.engine.pw_max(self.hull.d(), self.hull.lwl(), self.hull.cs(), self.hull.ws())));
        s.push(format!("Pw cruise = {}", self.engine.pw_cruise(self.hull.d(), self.hull.lwl(), self.hull.cs(), self.hull.ws())));
        s.push("".to_string());
        s.push(format!("hp max = {}", self.engine.hp_max(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push(format!("hp cruise = {}", self.engine.hp_cruise(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push("".to_string());

        s.push(format!("wgt_load = {}", self.wgt_load()));
        s.push(format!("wgt_hull = {}", self.wgt_hull()));
        s.push(format!("wgt_hull_plus = {}", self.wgt_hull_plus()));
        s.push(format!("wgt_misc = {}", self.wgts.wgt()));
        s.push(format!("wgt_armor = {}", self.wgt_armor()));
        s.push("".to_string());

        s.push(format!("main belt = {}", self.armor.main.wgt(self.hull.d(), self.hull.cwp(), self.hull.b)));
        s.push(format!("upper belt = {}", self.armor.upper.wgt(self.hull.d(), self.hull.cwp(), self.hull.b)));
        s.push(format!("end belt = {}", self.armor.end.wgt(self.hull.d(), self.hull.cwp(), self.hull.b)));
        // TODO: Replace with the following once circular references are fixed:
        // s.push(format!("deck = {}", self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), self.wgt_engine())));
        s.push(format!("deck = {}", self.armor.deck.wgt(self.hull.clone(), self.wgt_mag(), 0.0)));
        s.push("".to_string());

        s.push(format!("wgt_engine = {}", self.wgt_engine()));
        s.push(format!("d_engine = {}", self.engine.d_engine(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
        s.push(format!("d_factor = {}", self.d_factor()));
        s.push(format!("bunker (normal) = {}", self.engine.bunker(self.hull.d(), self.hull.lwl(), self.hull.leff(), self.hull.cs(), self.hull.ws())));
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
        s.push(format!("flotation = {}", self.flotation()));

        s.join("\n")
    }
}

// Testing Ship {{{2
#[cfg(test)]
mod ship {
    use super::*;
    use crate::test_support::*;
    use crate::hull::SternType;
    use crate::weapons::TorpedoMountType;

    fn get_hull() -> Hull {

        let mut hull = Hull::default();

        hull.set_d(7000.0);
        hull.set_lwl(500.0);
        hull.b = 50.0;
        hull.bb = hull.b;
        hull.t = 10.0;
        hull.bow_angle = 0.0;
        hull.stern_overhang = 0.0;

        hull.fc_len = 0.20;
        hull.fc_fwd = 10.0;
        hull.fc_aft = 10.0;

        hull.fd_len = 0.30;
        hull.fd_fwd = hull.fc_len;
        hull.fd_aft = hull.fc_len;

        hull.ad_fwd = hull.fc_len;
        hull.ad_aft = hull.fc_len;

        hull.qd_len = 0.15;
        hull.qd_fwd = hull.fc_len;
        hull.qd_aft = hull.fc_len;

        hull.bow_type = BowType::Normal;
        hull.stern_type = SternType::Cruiser;

        hull
    }

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

                    let mut ship = Ship::default();
                    ship.hull = get_hull().clone();

                    ship.torps[0].year = 1920;
                    ship.torps[0].num = 3;
                    ship.torps[0].mounts = 2;
                    ship.torps[0].diam = 20.0;
                    ship.torps[0].len = 10.0;
                    ship.torps[0].mount_kind = kind;

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

                    let mut ship = Ship::default();
                    ship.hull = get_hull().clone();

                    ship.torps[0].year = 1920;
                    ship.torps[0].num = 3;
                    ship.torps[0].mounts = 2;
                    ship.torps[0].diam = 20.0;
                    ship.torps[0].len = 10.0;
                    ship.torps[0].mount_kind = kind;

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

