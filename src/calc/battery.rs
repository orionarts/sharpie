use crate::calc::{Armor, Hull, Measurement, plural, Ship, UnitType, Units};
use crate::choice_enum;

use serde::{Deserialize, Serialize};

use std::f64::consts::PI;
use std::fmt;

// Battery {{{1
/// A battery of one type of gun.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Battery {
    /// Units
    pub units: Units,

    /// Number of guns in the battery.
    pub num: u32,

    /// Gun barrel diameter in inches.
    pub diam: Measurement,
    /// Gun barrel length in calibers.
    pub len: f64,

    /// Year gun was designed.
    pub year: u32,

    /// Number of shells in the magazine
    pub shells: u32,
    /// Weight of each shell.
    shell_wgt: Option<Measurement>,

    /// Type of gun.
    pub kind: GunType,

    /// Number of mounts in the battery.
    pub mount_num: u32,
    /// Kind of mounts.
    pub mount_kind: MountType,

    /// Armor thickness on mount face.
    pub armor_face: Measurement,
    /// Armor thickness elsewhere.
    // TODO: This should have a better name (other?)
    pub armor_back: Measurement,
    /// Armor thickness on barbette.
    pub armor_barb: Measurement,

    /// Separate groups of guns within the Battery
    pub groups: Vec<SubBattery>,
}

impl Default for Battery { // {{{2
    fn default() -> Self {
        Self {
            units: Units::Imperial,

            num: 0,
            diam: Measurement::new(0.0, UnitType::LengthSmall, Units::Imperial),
            len: 45.0,
            year: 0,
            shells: 0,
            shell_wgt: None,
            kind: GunType::default(),

            mount_num: 0,
            mount_kind: MountType::default(),
            armor_face: Measurement::new(0.0, UnitType::LengthSmall, Units::Imperial),
            armor_back: Measurement::new(0.0, UnitType::LengthSmall, Units::Imperial),
            armor_barb: Measurement::new(0.0, UnitType::LengthSmall, Units::Imperial),

            groups: vec![
                SubBattery::default(),
                SubBattery::default(),
            ],
        }
    }
}

impl Battery { // {{{2
    /// Factor to account for powder, etc. when calculating the magazine weight.
    ///
    const CORDITE_FACTOR: f64 = 0.2444444;

    // broad_and_below {{{3
    /// Returns true if the battery has Broadside mounts
    /// and any guns are mounted below the waterline.
    ///
    pub fn broad_and_below(&self) -> bool {
        if self.mount_kind == MountType::Broadside {
            for g in self.groups.iter() {
                if g.below != 0 { return true; }
            }
        }
        false
    }

    // concentration {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn concentration(&self, wgt_broad: f64) -> f64 {
        // Catch divide by zero
        if self.mount_num == 0 || wgt_broad == 0.0 { return 0.0; }

        (self.shell_wgt().imp() * self.num as f64 / wgt_broad) *
            if self.mount_kind.wgt_adj() > 0.6 {
                (4.0 / self.mount_num as f64).powf(0.25) - 1.0
            } else {
                -0.1
            }
    }

    // super_ {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn super_(&self, hull: Hull) -> f64 {
        if self.num == 0 { return 0.0 } // catch divide by zero

        let mut super_ = 0;
        for g in self.groups.iter() {
            super_ += g.super_()
        }

        match self.free(hull) {
            0.0 => 0.0, // Catch divide by zero
            free => ((super_ as f64 / self.num as f64) * (self.diam.imp() * 0.6).max(7.5) + free) / free,
        }
    }

    // free {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn free(&self, hull: Hull) -> f64 {
        if self.mount_num == 0 { return 0.0 } // Catch divide by zero

        let mut f = 0.0;
        for b in self.groups.iter() {
            f += b.free(hull.clone());
        }

        f / self.mount_num as f64
    }

    // armor_face_wgt {{{3
    /// Weight of battery face armor.
    ///
    pub fn armor_face_wgt(&self) -> f64 {
        let wgt = self.mount_kind.armor_face_wgt(self.armor_back.imp());

        let mut diameter_calc = 0.0;
        for g in self.groups.iter() {
            diameter_calc += g.diameter_calc(self.diam.imp()) * g.num_mounts() as f64;
        }

        let wgt = wgt * diameter_calc * self.house_hgt() * self.armor_face.imp() * Armor::INCH;

        wgt * self.kind.armor_face_wgt(self.armor_back.imp())
    }

    // house_hgt {{{3
    /// XXX: I do not know what this does.
    ///
    fn house_hgt(&self) -> f64 {
        f64::max(
            7.5,
            0.625 * self.diam.imp() * self.mount_kind.gunhouse_hgt_factor(),
        )
    }

    // armor_back_wgt {{{3
    /// Weight of battery back armor.
    ///
    pub fn armor_back_wgt(&self) -> f64 {
        let (shell_k, base_k) = self.mount_kind.armor_back_wgt();

        // Compute cylindrical shell
        let mut shell = 0.0;
        for g in self.groups.iter() {
            shell += g.diameter_calc(self.diam.imp()) * g.num_mounts() as f64;
        }
        shell *= self.house_hgt() * shell_k;

        // Compute circular base
        let mut base = 0.0;
        for g in self.groups.iter() {
            base += (g.diameter_calc(self.diam.imp()) / 2.0).powf(2.0) * g.num_mounts() as f64;
        }
        base *= PI * base_k;

        (shell + base) * self.armor_back.imp() * Armor::INCH
    }
    // armor_barb_wgt {{{3
    /// Weight of battery barbette armor
    ///
    pub fn armor_barb_wgt(&self, hull: Hull) -> f64 {
        let mut guns = 0;
        for g in self.groups.iter() {
            guns += g.layout.guns_per() * g.num_mounts();
        }

        if self.mount_num == 0 { return 0.0; } // catch divide by zero

        let a =
            if self.mount_kind.wgt_adj() > 0.5 {
                u32::min(4, guns / self.mount_num)
            } else {
                // TODO: This replicates what is most likely a SpringSharp bug in armWeightCalc()
                guns / self.mount_num
            };

        // This is **probably** what the code **should** be:
        // let a = u32::min(
            // if self.mount_kind.wgt_adj() > 0.5 { 4 } else { 5 },
            // guns / self.mount_num,
        // );

        let b = self.mount_kind.armor_barb_wgt();

        if self.free(hull.clone()) <= 0.0 {
            0.0
        } else {
            (1.0 - (a as f64 - 2.0) / 6.0) *
                 self.armor_barb.imp() *
                 self.num as f64 *
                 self.diam.imp().powf(1.2) *
                 b *
                 self.free(hull.clone()) / 16.0 *
                 self.super_(hull.clone()) *
                 b *
                 2.0 *
                 self.date_factor().sqrt()
        }
    }
    // armor_wgt {{{3
    /// Total weight of the battery's armor.
    ///
    pub fn armor_wgt(&self, hull: Hull) -> f64 {
        self.armor_face_wgt() + self.armor_back_wgt() + self.armor_barb_wgt(hull)
    }

    // wgt_adj {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn wgt_adj(&self) -> f64 {
        if self.mount_num == 0 { return 0.0; } // Catch divide by zero

        let mut v = 0.0;
        for b in self.groups.iter() {
            v += b.wgt_adj();
        }

        v / self.mount_num as f64
    }

    // date_factor {{{3
    /// Factor used to adjust shell weight based on year.
    ///
    fn date_factor(&self) -> f64 {
        Ship::year_adj(self.year).sqrt()
    }

    // set_shell_wgt {{{3
    /// Set the shell weight.
    ///
    pub fn set_shell_wgt(&mut self, wgt: f64, units: Units) -> f64 {
        self.shell_wgt = Some(Measurement::new(wgt, UnitType::Weight, units));

        wgt
    }

    // shell_wgt {{{3
    /// Get the shell weight.
    ///
    /// Return the value previously set by set_shell_wgt()
    /// or the estimated shell weight if unset.
    ///
    pub fn shell_wgt(&self) -> Measurement {
        match self.shell_wgt {
            Some(wgt) => wgt,
            None      => Measurement::new(self.shell_wgt_est(), UnitType::Weight, Units::Imperial),
        }
    }

    // shell_wgt_est {{{3
    /// Estimated shell weight.
    ///
    pub fn shell_wgt_est(&self) -> f64 {
        self.diam.imp().powf(3.0) / 1.9830943211886 * self.date_factor() *
            ( 1.0 + if self.len < 45.0 { -1.0 } else { 1.0 } * (45.0 - self.len).abs().sqrt() / 45.0 )
    }

    // gun_wgt {{{3
    /// Weight of the barrels in the battery.
    ///
    pub fn gun_wgt(&self) -> f64 {
        if self.diam.imp() == 0.0 { return 0.0; }

        self.shell_wgt_est() * (self.len / 812.389434917877 *
            (1.0 + (1.0 / self.diam.imp()).powf(2.3297949327695))
            ) * self.num as f64
    }

    // mount_wgt {{{3
    /// Weight of a single gun mount.
    ///
    pub fn mount_wgt(&self) -> f64 {
        if self.diam.imp() == 0.0 { return 0.0; } // Catch divide by zero

        let wgt = self.mount_kind.wgt() *
            if self.mount_kind.wgt_adj() < 0.6 {
                self.kind.wgt_sm()
            } else {
                self.kind.wgt_lg()
            };

        let wgt = (wgt + 1.0 / self.diam.imp().powf(0.313068808543972)) * self.gun_wgt();

        let wgt =
            if self.diam.imp() > 10.0 {
                wgt * (1.0 - 2.1623769 * self.diam.imp() / 100.0)
            } else if self.diam.imp() <= 1.0 {
                self.gun_wgt()
            } else {
                wgt
            };

        wgt * self.wgt_adj()
    }

    // broadside_wgt {{{3
    /// Weight of shells if each barrel fires a single shell.
    ///
    pub fn broadside_wgt(&self) -> f64 {
        self.num as f64 * self.shell_wgt().imp()
    }

    // mag_wgt {{{3
    /// Weight of the battery magazine.
    ///
    pub fn mag_wgt(&self) -> f64 {
        (self.num * self.shells) as f64 * self.shell_wgt().imp() / Ship::POUND2TON * (1.0 + Self::CORDITE_FACTOR)
    }
}

// Internals Output {{{2
#[cfg(debug_assertions)]
impl Battery {
    pub fn internals(&self, hull: Hull, wgt_broad: f64) {
        eprintln!("units = {:?}", self.units);
        eprintln!("num = {}", self.num);
        eprintln!("diam = {}", self.diam.imp());
        eprintln!("len = {}", self.len);
        eprintln!("year = {}", self.year);
        eprintln!("shells = {}", self.shells);
        eprintln!("kind = {}", self.kind);
        eprintln!("mount_num = {}", self.mount_num);
        eprintln!("mount_kind = {}", self.mount_kind);
        eprintln!("armor_face = {}", self.armor_face.imp());
        eprintln!("armor_back = {}", self.armor_back.imp());
        eprintln!("armor_barb = {}", self.armor_barb.imp());

        eprintln!("broad_and_below() = {}", self.broad_and_below());
        eprintln!("concentration() = {}", self.concentration(wgt_broad));
        eprintln!("super_() = {}", self.super_(hull.clone()));
        eprintln!("free() = {}", self.free(hull.clone()));
        eprintln!("house_hgt() = {}", self.house_hgt());
        eprintln!("armor_face_wgt() = {}", self.armor_face_wgt());
        eprintln!("armor_back_wgt() = {}", self.armor_back_wgt());
        eprintln!("armor_barb_wgt() = {}", self.armor_barb_wgt(hull.clone()));
        eprintln!("armor_wgt() = {}", self.armor_wgt(hull.clone()));
        eprintln!("wgt_adj() = {}", self.wgt_adj());
        eprintln!("date_factor() = {}", self.date_factor());
        eprintln!("shell_wgt() = {}", self.shell_wgt().imp());
        eprintln!("shell_wgt_est() = {}", self.shell_wgt_est());
        eprintln!("gun_wgt() = {}", self.gun_wgt());
        eprintln!("mount_wgt() = {}", self.mount_wgt());
        eprintln!("broadside_wgt() = {}", self.broadside_wgt());
        eprintln!("mag_wgt() = {}", self.mag_wgt());
        eprintln!();

        for (i, g) in self.groups.iter().enumerate() {
            eprintln!("Group {}", i);
            eprintln!("--------");
            g.internals(hull.clone(), self.diam.imp());
        }
    }
}

// GunType {{{1
/// Type of gun
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum GunType {
    MuzzleLoading,
    #[default]
    BreechLoading,
    QuickFiring,
    AntiAir,
    DualPurpose,
    RapidFire,
    MachineGun,
}

choice_enum!(GunType {
    MuzzleLoading => ("Muzzle loading gun",  "Muzzle loading"),
    BreechLoading => ("Breech loading gun",  "Breech loading"),
    QuickFiring   => ("Quick firing gun",    "Quick firing"),
    AntiAir       => ("Anti-air gun",        "Anti-air"),
    DualPurpose   => ("Dual purpose gun",    "Dual purpose"),
    RapidFire     => ("Auto rapid fire gun", "Auto rapid fire"),
    MachineGun    => ("Machine gun",         "Machine"),
});

impl GunType { // {{{2
    // armor_face_wgt {{{3
    /// Multiplier for determining the weight of a mount's face armor.
    ///
    pub fn armor_face_wgt(&self, armor_back: f64) -> f64 {
        let mut wgt =
            match self {
                Self::MuzzleLoading => 1.0,
                Self::BreechLoading => 1.0,
                Self::QuickFiring   => 1.0,
                Self::AntiAir       => 0.333,
                Self::DualPurpose   => 1.0,
                Self::RapidFire     => 1.0,
                Self::MachineGun    => 1.0,
            };

        if armor_back == 0.0 {
            wgt *=
                match self {
                    Self::MuzzleLoading => 1.0,
                    Self::BreechLoading => 1.0,
                    Self::QuickFiring   => 1.0,
                    Self::AntiAir       => 1.0,
                    Self::DualPurpose   => 1.0,
                    Self::RapidFire     => 1.0,
                    Self::MachineGun    => 0.333,
                };
        }

        wgt
    }

    // wgt_sm {{{3
    /// Multiplier to adjust mount weight for small mounts.
    ///
    pub fn wgt_sm(&self) -> f64 {
        match self {
            Self::MuzzleLoading => 0.9,
            Self::BreechLoading => 1.0,
            Self::QuickFiring   => 1.35,
            Self::AntiAir       => 1.44,
            Self::DualPurpose   => 1.57,
            Self::RapidFire     => 2.16,
            Self::MachineGun    => 1.0,
        }
    }

    // wgt_lg {{{3
    /// Multiplier to adjust mount weight for large mounts.
    ///
    pub fn wgt_lg(&self) -> f64 {
        match self {
            Self::MuzzleLoading => 0.98,
            Self::BreechLoading => 1.0,
            Self::QuickFiring   => 1.0,
            Self::AntiAir       => 1.0,
            Self::DualPurpose   => 1.1,
            Self::RapidFire     => 1.5,
            Self::MachineGun    => 1.0,
        }
    }
}



// MountType {{{1
/// Type of gun mount.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum MountType {
    Broadside,
    ColesTurret,
    OpenBarbette,
    ClosedBarbette,
    DeckAndHoist,
    #[default]
    Deck,
    Casemate,
}

choice_enum!(MountType {
    Broadside      => ("in broadside mount",             "broadside"),
    ColesTurret    => ("in Coles/Ericsson turret mount", "Coles/Ericsson turret"),
    OpenBarbette   => ("in open barbette mount",         "open barbette"),
    ClosedBarbette => ("in turret on barbette mount",    "turret on barbette"),
    DeckAndHoist   => ("in deck and hoist mount",        "deck and hoist"),
    Deck           => ("in deck mount",                  "deck"),
    Casemate       => ("in casemate mount",              "casemate"),
});
impl MountType { // {{{2
    // gunhouse_hgt_factor {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn gunhouse_hgt_factor(&self) -> f64 {
        match self {
            Self::Broadside      => 1.0,
            Self::ColesTurret    => 2.0,
            Self::OpenBarbette   => 1.0,
            Self::ClosedBarbette => 1.0,
            Self::DeckAndHoist   => 1.0,
            Self::Deck           => 1.0,
            Self::Casemate       => 1.0,
        }
    }

    // armor_face_wgt {{{3
    /// Multiplier for determining the weight of a mount's face armor.
    ///
    pub fn armor_face_wgt(&self, armor_back: f64) -> f64 {
        let mut wgt = match self {
            Self::Broadside      => 1.0,
            Self::ColesTurret    => PI / 2.0,
            Self::OpenBarbette   => 0.0,
            Self::ClosedBarbette => 0.5,
            Self::DeckAndHoist   => 0.5,
            Self::Deck           => 0.5,
            Self::Casemate       => 1.0,
        };

        if armor_back == 0.0 {
            wgt += match self {
                Self::Broadside      => 0.0,
                Self::ColesTurret    => 0.0,
                Self::OpenBarbette   => 0.0,
                Self::ClosedBarbette => 1.0,
                Self::DeckAndHoist   => 1.0,
                Self::Deck           => 1.0,
                Self::Casemate       => 0.0,
            }
        }

        wgt
    }

    // armor_back_wgt {{{3
    /// Multipliers needed to determine back armor weight for the mount.
    ///
    pub fn armor_back_wgt(&self) -> (f64, f64) {
        let a = match self {
            Self::Broadside      => 0.0,
            Self::ColesTurret    => 0.0,
            Self::OpenBarbette   => 0.0,
            Self::ClosedBarbette => 2.5,
            Self::DeckAndHoist   => 2.5,
            Self::Deck           => 2.5,
            Self::Casemate       => 0.0,
        };

        let b = match self {
            Self::Broadside      => 0.75,
            Self::ColesTurret    => 1.0,
            Self::OpenBarbette   => 0.75,
            Self::ClosedBarbette => 0.75,
            Self::DeckAndHoist   => 0.75,
            Self::Deck           => 0.75,
            Self::Casemate       => 0.75,
        };

        (a, b)
    }

    // armor_barb_wgt {{{3
    /// Multiplier to determine barbette armor weight.
    ///
    pub fn armor_barb_wgt(&self) -> f64 {
        match self {
            Self::Broadside      => 0.0,
            Self::ColesTurret    => 0.0,
            Self::OpenBarbette   => 0.6416,
            Self::ClosedBarbette => 0.5,
            Self::DeckAndHoist   => 0.1,
            Self::Deck           => 0.0,
            Self::Casemate       => 0.1,
        }
    }

    // wgt {{{3
    /// Multiplier for weight calculations.
    ///
    pub fn wgt(&self) -> f64 {
        match self {
            Self::Broadside      =>0.83,
            Self::ColesTurret    =>3.5,
            Self::OpenBarbette   =>3.33,
            Self::ClosedBarbette =>3.5,
            Self::DeckAndHoist   =>3.15,
            Self::Deck           =>1.08,
            Self::Casemate       =>1.08,
        }
    }
    // wgt_adj {{{3
    /// Multiplier for weight calculations.
    ///
    pub fn wgt_adj(&self) -> f64 {
        match self {
            Self::Broadside      =>0.5,
            Self::ColesTurret    =>1.0,
            Self::OpenBarbette   =>0.7,
            Self::ClosedBarbette =>1.0,
            Self::DeckAndHoist   =>1.0,
            Self::Deck           =>0.5,
            Self::Casemate       =>0.5,
        }
    }
}



// SubBattery {{{1
/// Gun grouping within a battery.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SubBattery {
    /// Layout of guns within a turret.
    pub layout: GunLayoutType,
    /// Placement of guns on the ship.
    pub distribution: GunDistributionType,

    /// Number of mounts above the waterline.
    pub above: u32,
    /// Number of mounts on the waterline.
    pub on: u32,
    /// Number of mounts below the waterline.
    pub below: u32,

    /// If mounts above the deck are superfiring
    pub two_mounts_up: bool,
    /// If mounts below the waterline are on the lower deck
    pub lower_deck: bool,
}

// Internals Output {{{2
#[cfg(debug_assertions)]
impl SubBattery {
    pub fn internals(&self, hull: Hull, diam: f64) {
        eprintln!("layout = {}", self.layout);
        eprintln!("distribution = {}", self.distribution);
        eprintln!("above = {}", self.above);
        eprintln!("on = {}", self.on);
        eprintln!("below = {}", self.below);
        eprintln!("two_mounts_up = {}", self.two_mounts_up);
        eprintln!("lower_deck = {}", self.lower_deck);
        eprintln!("super_() = {}", self.super_());
        eprintln!("num_mounts() = {}", self.num_mounts());
        eprintln!("diameter_calc() = {}", self.diameter_calc(diam));
        eprintln!("wgt_adj() = {}", self.wgt_adj());
        eprintln!("free() = {}", self.free(hull.clone()));
        eprintln!();
    }
}

impl SubBattery { // {{{2
    // super_ {{{3
    /// Number of barrels above the waterline, reduced by the number of barrels
    /// below the waterline. Superfiring and lower deck barrels count double.
    ///
    pub fn super_(&self) -> i32 {
        let above: i32 = (self.above * if self.two_mounts_up { 2 } else { 1 }) as i32;
        let below: i32 = (self.below * if self.lower_deck    { 2 } else { 1 }) as i32;

        (above - below) * self.layout.guns_per() as i32
    }

    // num_mounts {{{3
    /// Total number of gun mounts.
    ///
    pub fn num_mounts(&self) -> u32 {
        self.above + self.on + self.below
    }

    // diameter_calc {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn diameter_calc(&self, diam: f64) -> f64 {
        if diam == 0.0 { return 0.0; } // Catch divide by zero

        let (factor, power) = self.layout.diameter_calc_nums();

        let mut calc = factor * diam * (1.0 + (1.0 / diam).powf(power));

        if diam < 12.0                               { calc += 12.0 / diam; }
        if diam > 1.0 && self.layout.wgt_adj() < 1.0 { calc *= 0.9; }

        calc
    }

    // wgt_adj {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn wgt_adj(&self) -> f64 {
        self.layout.wgt_adj() * self.num_mounts() as f64
    }

    // free {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn free(&self, hull: Hull) -> f64 {
        let free = self.distribution.free(self.num_mounts(), hull);

        free * self.num_mounts() as f64
    }
}



// GunDistributionType {{{1
/// Distribution of gun mounts on the deck.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum GunDistributionType {
    #[default]
    None,
    CenterlineEven,
    CenterlineEndsFD,
    CenterlineEndsAD,
    CenterlineFDFwd,
    CenterlineFD,
    CenterlineFDAft,
    CenterlineADFwd,
    CenterlineAD,
    CenterlineADAft,
    SidesEven,
    SidesEndsFD,
    SidesEndsAD,
    SidesFDFwd,
    SidesFD,
    SidesFDAft,
    SidesADFwd,
    SidesAD,
    SidesADAft,
}

choice_enum!(GunDistributionType {
    CenterlineEven   => ("Centreline - distributed"),
    CenterlineEndsFD => ("Centreline - ends (fore >= aft)"),
    CenterlineEndsAD => ("Centreline - ends (aft >= fore)"),
    CenterlineFDFwd  => ("Centreline - fore deck forward"),
    CenterlineFD     => ("Centreline - fore deck"),
    CenterlineFDAft  => ("Centreline - fore deck aft"),
    CenterlineADFwd  => ("Centreline - aft deck forward"),
    CenterlineAD     => ("Centreline - aft deck"),
    CenterlineADAft  => ("Centreline - aft deck aft"),
    SidesEven        => ("Sides - distributed"),
    SidesEndsFD      => ("Sides - ends (fore >= aft)"),
    SidesEndsAD      => ("Sides - ends (aft >= fore)"),
    SidesFDFwd       => ("Sides - fore deck forward"),
    SidesFD          => ("Sides - fore deck"),
    SidesFDAft       => ("Sides - fore deck aft"),
    SidesADFwd       => ("Sides - aft deck forward"),
    SidesAD          => ("Sides - aft deck"),
    SidesADAft       => ("Sides - aft deck aft"),
    None             => ("None"),
});

impl GunDistributionType { // {{{2
    // desc {{{3
    /// Description of type based on number of mounts and length of decks.
    ///
    pub fn desc(&self, mounts: u32, fwd_len: f64) -> String {
        let s = match self {
            Self::None => "layout not set",
            Self::CenterlineEven => {
                if mounts == 1 {
                    if fwd_len >= 0.5 {
                        "centreline amidships (forward deck)"
                    } else {
                        "centreline amidships (aft deck)"
                    }
                } else {
                    "centreline, evenly spread"
                }
            }
            Self::CenterlineEndsFD => {
                if mounts == 1 {
                    "centreline forward"
                } else if mounts.is_multiple_of(2) {
                    "centreline ends, evenly spread"
                } else {
                    "centreline ends, majority forward"
                }
            }
            Self::CenterlineEndsAD => {
                if mounts == 1 {
                    "centreline aft"
                } else if mounts.is_multiple_of(2) {
                    "centreline ends, evenly spread"
                } else {
                    "centreline ends, majority aft"
                }
            }
            Self::CenterlineFDFwd => "centreline, forward deck forward",
            Self::CenterlineFD => {
                if mounts == 1 {
                    "centreline, forward deck centre"
                } else {
                    "centreline, forward evenly spread"
                }
            }
            Self::CenterlineFDAft => "centreline, forward deck aft",
            Self::CenterlineADFwd => "centreline, aft deck forward",
            Self::CenterlineAD => {
                if mounts == 1 {
                    "centreline, aft deck centre"
                } else {
                    "centreline, aft evenly spread"
                }
            }
            Self::CenterlineADAft => "centreline, aft deck aft",
            Self::SidesEven => {
                if mounts < 3 {
                    "sides amidships"
                } else {
                    "sides, evenly spread"
                }
            }
            Self::SidesEndsFD => {
                if mounts < 3 {
                    "sides, forward"
                } else if mounts.is_multiple_of(4) {
                    "side ends, evenly spread"
                } else {
                    "side ends, majority forward"
                }
            }
            Self::SidesEndsAD => {
                if mounts < 3 {
                    "sides aft"
                } else if mounts.is_multiple_of(4) {
                    "side ends, evenly spread"
                } else {
                    "side ends, majority aft"
                }
            }
            Self::SidesFDFwd => "sides, forward deck forward",
            Self::SidesFD => {
                if mounts < 3 {
                    "sides, forward deck centre"
                } else {
                    "sides, forward evenly spread"
                }
            }
            Self::SidesFDAft => "sides, forward deck aft",
            Self::SidesADFwd => "sides, aft deck forward",
            Self::SidesAD => {
                if mounts < 3 {
                    "sides, aft deck centre"
                } else {
                    "sides, aft evenly spread"
                }
            }
            Self::SidesADAft => "sides, aft deck aft",
        };

        s.into()
    }

    // super_aft {{{3
    /// True if the type would place guns aft.
    ///
    pub fn super_aft(&self) -> bool {
        matches!(self,
            Self::CenterlineEndsAD |
            Self::CenterlineADFwd |
            Self::CenterlineAD |
            Self::CenterlineADAft |
            Self::SidesEndsAD |
            Self::SidesADFwd |
            Self::SidesAD |
            Self::SidesADAft
        )
    }

    // mounts_fwd {{{3
    /// Number of mounts that are placed forward.
    ///
    fn mounts_fwd(&self, tot: u32, fwd_len: f64) -> u32 {
        // Divide n by 2 and round
        fn half(n: u32) -> u32 {
            f64::round(n as f64 / 2.0) as u32
        }

        match self {
            Self::None             => 0,
            Self::CenterlineFDFwd  => tot,
            Self::CenterlineFD     => tot,
            Self::CenterlineFDAft  => tot,
            Self::CenterlineADFwd  => tot,
            Self::SidesFDFwd       => tot,
            Self::SidesFD          => tot,
            Self::SidesFDAft       => tot,

            Self::CenterlineAD     => 0,
            Self::CenterlineADAft  => 0,
            Self::SidesADFwd       => 0,
            Self::SidesAD          => 0,
            Self::SidesADAft       => 0,

            Self::CenterlineEndsFD | Self::SidesEndsFD =>
                if tot == 1 { tot } else { half(tot) },

            Self::CenterlineEndsAD | Self::SidesEndsAD =>
                if tot == 1 { 0 } else { tot - half(tot) },

            Self::CenterlineEven | Self::SidesEven => {
                if tot == 1 && fwd_len >= 0.5 {
                    tot
                } else if fwd_len >= 0.5 {
                    half(tot)
                } else if tot == 1 && fwd_len < 0.5 {
                    0
                } else {
                    tot - half(tot)
                }
            }
        }
    }

    // free {{{3
    /// XXX: I do not know what this does
    ///
    pub fn free(&self, num_mounts: u32, hull: Hull) -> f64 {
        if num_mounts == 0 { return 0.0; } // catch divide by zero

        // Get these as floats to avoid casts later
        let fwd = self.mounts_fwd(num_mounts, hull.freeboard.fc_len + hull.freeboard.fd_len) as f64;
        let tot = num_mounts as f64;

        let fd     = hull.freeboard.fd();
        let ad     = hull.freeboard.ad();
        let fd_fwd = hull.freeboard.fd_fwd.imp();
        let fd_aft = hull.freeboard.fd_aft.imp();
        let ad_fwd = hull.freeboard.ad_fwd.imp();
        let ad_aft = hull.freeboard.ad_aft.imp();

        match self {
            Self::None             => 0.0,

            Self::CenterlineEven | Self::SidesEven =>
                (fwd * fd + (tot - fwd) * ad) / tot,

            Self::CenterlineEndsFD |
            Self::CenterlineEndsAD |
            Self::SidesEndsFD |
            Self::SidesEndsAD => {
                (if fwd > 0.0 {
                    fwd * ((fd_fwd - fd) / fwd * 0.5 + (fd_fwd + fd) * 0.5)
                } else {
                    0.0
                } + (tot - fwd) * ((ad_aft - ad) * 1.0 / (tot - fwd) * 0.5 + (ad_aft + ad) * 0.5)) / tot
            }

            Self::CenterlineFDFwd | Self::SidesFDFwd => {
                if fwd > 0.0 {
                    (fd_fwd - fd) / fwd * 0.5 + (fd_fwd + fd) * 0.5
                } else {
                    0.0
                }
            }

            Self::CenterlineFD | Self::SidesFD => fd,

            Self::CenterlineFDAft | Self::SidesFDAft => {
                if fwd > 0.0 {
                    (fd_aft - fd) / fwd * 0.5 + (fd_aft + fd) * 0.5
                } else {
                    0.0
                }
            }

            Self::CenterlineADFwd | Self::SidesADFwd => {
                if (tot - fwd) > 0.0 {
                    (ad_fwd - ad) / (tot - fwd) * 0.5 + (ad_fwd + ad) * 0.5
                } else {
                    0.0
                }
            }

            Self::CenterlineAD | Self::SidesAD => ad,

            Self::CenterlineADAft | Self::SidesADAft => {
                if (tot - fwd) > 0.0 {
                    (ad_aft - ad) / (tot - fwd) * 0.5 + (ad_aft + ad) * 0.5
                } else {
                    0.0
                }
            }
        }
    }

    // gun_position {{{3
    /// XXX: I do not know what this does.
    ///
    fn gun_position(&self, fd_len: f64, ad_len: f64) -> f64 {
        match self {
            Self::CenterlineFDFwd  => 0.25 * fd_len,
            Self::CenterlineFD     => 0.5  * fd_len,
            Self::CenterlineFDAft  => 0.75 * fd_len,
            Self::CenterlineADFwd  => 0.25 * ad_len,
            Self::CenterlineAD     => 0.5  * ad_len,
            Self::CenterlineADAft  => 0.75 * ad_len,
            Self::SidesFDFwd       => 0.25 * fd_len,
            Self::SidesFD          => 0.5  * fd_len,
            Self::SidesFDAft       => 0.75 * fd_len,
            Self::SidesADFwd       => 0.25 * ad_len,
            Self::SidesAD          => 0.5  * ad_len,
            Self::SidesADAft       => 0.75 * ad_len,
            _                      => 0.0 // It is an error if we get here
        }
    }

    // g1_gun_position {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn g1_gun_position(&self, fd_len: f64, ad_len: f64) -> f64 {
        match self {
            Self::CenterlineEven |
            Self::CenterlineEndsFD |
            Self::CenterlineEndsAD |
            Self::SidesEven |
            Self::SidesEndsFD |
            Self::SidesEndsAD => 1.0,
            _ => self.gun_position(fd_len, ad_len),
        }
    }
    // g2_gun_position {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn g2_gun_position(&self, fd_len: f64, ad_len: f64) -> f64 {
        match self {
            Self::CenterlineEven |
            Self::CenterlineEndsFD |
            Self::CenterlineEndsAD |
            Self::SidesEven |
            Self::SidesEndsFD |
            Self::SidesEndsAD => 0.0,
            _ => self.gun_position(fd_len, ad_len),
        }
    }

    // super_factor_long {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn super_factor_long(&self) -> bool {
        match self {
            Self::None             => false,
            Self::CenterlineEven   => false,
            Self::CenterlineEndsFD => false,
            Self::CenterlineEndsAD => true,
            Self::CenterlineFDFwd  => true,
            Self::CenterlineFD     => true,
            Self::CenterlineFDAft  => true,
            Self::CenterlineADFwd  => true,
            Self::CenterlineAD     => true,
            Self::CenterlineADAft  => true,
            Self::SidesEven        => false,
            Self::SidesEndsFD      => false,
            Self::SidesEndsAD      => false,
            Self::SidesFDFwd       => true,
            Self::SidesFD          => true,
            Self::SidesFDAft       => true,
            Self::SidesADFwd       => true,
            Self::SidesAD          => true,
            Self::SidesADAft       => true,
        }
    }
}



// GunLayoutType {{{1
/// Layout of guns within a mount.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum GunLayoutType {
    #[default]
    Single,
    Twin2Row,
    Quad4Row,
    Twin,
    TwoGun,
    Quad2Row,
    Triple,
    ThreeGun,
    Sex2Row,
    Quad,
    FourGun,
    Oct2Row,
    Quint,
    FiveGun,
    Dec2Row,
}

choice_enum!(GunLayoutType {
    Single   => ("Single mount",          "Single"),
    Twin2Row => ("2 row twin mount",      "2 row, twin"),
    Quad4Row => ("4 row quad mount",      "4 row, quad"),
    Twin     => ("Twin mount",            "Twin"),
    TwoGun   => ("2-gun mount",           "2-gun"),
    Quad2Row => ("2 row quad mount",      "2 row, quad"),
    Triple   => ("Triple mount",          "Triple"),
    ThreeGun => ("3-gun mount",           "3-gun"),
    Sex2Row  => ("2 row sextuple mount",  "2 row, sextuple"),
    Quad     => ("Quad mount",            "quad"),
    FourGun  => ("4-gun mount",           "4-gun"),
    Oct2Row  => ("2 row octuple mount",   "2 row, octuple"),
    Quint    => ("Quintuple mount",       "quintuple"),
    FiveGun  => ("5-gun mount",           "5-gun"),
    Dec2Row  => ("2 row decuple mount",   "2 row, decuple"),
});

impl GunLayoutType { // {{{2
    // num_guns {{{3
    /// Number of guns per mount.
    ///
    pub fn guns_per(&self) -> u32 {
        match self {
            Self::Single   => 1,
            Self::Twin2Row => 2,
            Self::Twin     => 2,
            Self::TwoGun   => 2,
            Self::Triple   => 3,
            Self::ThreeGun => 3,
            Self::Quad2Row => 4,
            Self::Quad4Row => 4,
            Self::Quad     => 4,
            Self::FourGun  => 4,
            Self::Quint    => 5,
            Self::FiveGun  => 5,
            Self::Sex2Row  => 6,
            Self::Oct2Row  => 8,
            Self::Dec2Row  => 10,
        }
    }

    // diameter_calc_nums {{{3
    /// Return values needed for SubBattery::diameter_calc().
    ///
    pub fn diameter_calc_nums(&self) -> (f64, f64) {
        match self {
            Self::Single   => (1.44, 0.609725),
            Self::Twin2Row => (1.44, 0.609725),
            Self::Quad4Row => (1.44, 0.609725),
            Self::Twin     => (1.52, 0.4205),
            Self::TwoGun   => (1.52, 0.4205),
            Self::Quad2Row => (1.52, 0.4205),
            Self::Triple   => (1.64, 0.29),
            Self::ThreeGun => (1.64, 0.29),
            Self::Sex2Row  => (1.64, 0.29),
            Self::Quad     => (1.8, 0.2),
            Self::FourGun  => (1.8, 0.2),
            Self::Oct2Row  => (1.8, 0.2),
            Self::Quint    => (2.0, 0.14),
            Self::FiveGun  => (2.0, 0.14),
            Self::Dec2Row  => (2.0, 0.14),
        }
    }

    // wgt_adj {{{3
    /// Return values needed by SubBattery::wgt_adj().
    ///
    pub fn wgt_adj(&self) -> f64 {
        match self {
            Self::Single   => 1.0,
            Self::Twin2Row => 1.0,
            Self::Quad4Row => 1.0,
            Self::Twin     => 0.75,
            Self::TwoGun   => 1.0,
            Self::Quad2Row => 1.0,
            Self::Triple   => 0.75,
            Self::ThreeGun => 1.0,
            Self::Sex2Row  => 1.0,
            Self::Quad     => 0.75,
            Self::FourGun  => 1.0,
            Self::Oct2Row  => 1.0,
            Self::Quint    => 0.75,
            Self::FiveGun  => 1.0,
            Self::Dec2Row  => 1.0,
        }
    }
}

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;

    // Battery {{{2
    mod battery {
        use super::*;

        // Test broad_and_below {{{3
        macro_rules! test_broad_and_below {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount_kind, guns_below) = $value;

                        let mut btry = Battery::default();
                        btry.mount_kind = mount_kind;
                        btry.groups[0].below = guns_below;

                        assert_eq!(expected, btry.broad_and_below());
                    }
                )*
            }
        }
        test_broad_and_below! {
            // name:                             (broad_and_below, mount_kind, guns_below)
            broad_and_below_not_broadside:       (false, MountType::Deck, 0),
            broad_and_below_broadside_not_below: (false, MountType::Broadside, 0),
            broad_and_below_broadside_below:     (true, MountType::Broadside, 1),
        }

        // Test concentration {{{3
        macro_rules! test_concentration {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, shell_wgt, mount_kind, mount_num) = $value;

                        let mut btry = Battery::default();
                        btry.set_shell_wgt(shell_wgt, Units::Imperial);
                        btry.mount_kind = mount_kind;
                        btry.mount_num = mount_num;
                        btry.num = 10;

                        let wgt_broadside = 1000.0;

                        assert_eq!(expected, to_place(btry.concentration(wgt_broadside), 5));
                    }
                )*
            }
        }
        test_concentration! {
            // name: (concentration, shell_wgt, mount_kind, mount_num)
            concentration_chk_div_by_0: (0.0, 0.0, MountType::Broadside, 0),
            concentration_sm_mount:     (-0.01, 10.0, MountType::Broadside, 1),
            concentration_lg_mount:     (0.04142, 10.0, MountType::ColesTurret, 1),
        }

        // Test super_ {{{3
        macro_rules! test_super_ {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, group_1_mounts, group_2_mounts) = $value;

                        let mut btry = Battery::default();

                        // Assume they are all single mounts
                        btry.num = group_1_mounts + group_2_mounts;
                        btry.mount_num = group_1_mounts + group_2_mounts;

                        btry.groups[0].above = group_1_mounts;
                        btry.groups[1].on = group_2_mounts;

                        btry.groups[0].distribution = GunDistributionType::CenterlineEven;
                        btry.groups[1].distribution = GunDistributionType::CenterlineEven;

                        let mut hull = Hull::default();
                        hull.freeboard.fc_len = 0.2;

                        hull.freeboard.fd_len = 0.3;
                        hull.freeboard.fd_fwd = Measurement::new(10.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.fd_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.ad_fwd = Measurement::new(20.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.ad_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.qd_len = 0.15;

                        assert_eq!(expected, to_place(btry.super_(hull), 5));
                    }
                )*
            }
        }
        test_super_! {
            // name: (super_, group_1_mounts, group_2_mounts)
            super_test_1: (1.3, 2, 5),
            super_test_2: (1.75, 5, 2),
        }

        // Test free {{{3
        macro_rules! test_free {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, group_1_mounts, group_2_mounts) = $value;

                        let mut btry = Battery::default();

                        btry.mount_num = group_1_mounts + group_2_mounts;

                        btry.groups[0].on = group_1_mounts;
                        btry.groups[1].on = group_2_mounts;

                        btry.groups[0].distribution = GunDistributionType::CenterlineEven;
                        btry.groups[1].distribution = GunDistributionType::CenterlineEven;

                        let mut hull = Hull::default();
                        hull.freeboard.fc_len = 0.2;

                        hull.freeboard.fd_len = 0.3;
                        hull.freeboard.fd_fwd = Measurement::new(10.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.fd_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.ad_fwd = Measurement::new(20.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.ad_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.qd_len = 0.15;

                        assert_eq!(expected, to_place(btry.free(hull), 3));
                    }
                )*
            }
        }
        test_free! {
            // name: (free, group_1_mounts, group_2_mounts)
            free_test_1: (7.143, 2, 5),
            free_test_2: (7.5, 2, 0),
            free_test_3: (7.0, 0, 5),
        }

        // Test armor_face_wgt {{{3
        macro_rules! test_armor_face_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun_kind, mount_kind, armor_face, armor_back) = $value;

                        let mut btry = Battery::default();

                        btry.kind = gun_kind;
                        btry.mount_kind = mount_kind;
                        btry.armor_face = Measurement::new(armor_face, UnitType::LengthSmall, Units::Imperial);
                        btry.armor_back = Measurement::new(armor_back, UnitType::LengthSmall, Units::Imperial);
                        btry.diam = Measurement::new(10.0, UnitType::LengthSmall, Units::Imperial);

                        btry.groups[0].on = 2;
                        btry.groups[1].on = 0;

                        btry.groups[0].layout = GunLayoutType::Single;

                        assert_eq!(expected, to_place(btry.armor_face_wgt(), 2));
                    }
                )*
            }
        }
        test_armor_face_wgt! {
            // name: (armor_face_wgt, gun_kind, mount_kind, armor_face, armor_back)
            armor_face_wgt_no_back: (7.97, GunType::BreechLoading, MountType::DeckAndHoist, 1.0, 0.0),
            armor_face_wgt_back: (2.66, GunType::BreechLoading, MountType::DeckAndHoist, 1.0, 1.0),
        }

        // Test house_hgt {{{3
        macro_rules! test_house_hgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, diam) = $value;

                        let mut btry = Battery::default();
                        btry.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                        btry.mount_kind = MountType::Broadside;

                        assert_eq!(expected, to_place(btry.house_hgt(), 5));
                    }
                )*
            }
        }
        test_house_hgt! {
            // name: (house_hgt, diam)
            house_hgt_1: (8.75, 14.0),
            house_hgt_2: (7.5, 10.0),
        }

        // Test armor_back_wgt {{{3
        macro_rules! test_armor_back_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun_kind, mount_kind, armor_back) = $value;

                        let mut btry = Battery::default();

                        btry.kind = gun_kind;
                        btry.mount_kind = mount_kind;
                        btry.armor_back = Measurement::new(armor_back, UnitType::LengthSmall, Units::Imperial);
                        btry.diam = Measurement::new(10.0, UnitType::LengthSmall, Units::Imperial);

                        btry.groups[0].on = 2;
                        btry.groups[1].on = 0;

                        btry.groups[0].layout = GunLayoutType::Single;

                        assert_eq!(expected, to_place(btry.armor_back_wgt(), 2));
                    }
                )*
            }
        }
        test_armor_back_wgt! {
            // name: (armor_back_wgt, gun_kind, mount_kind, armor_back)
            armor_back_wgt_1: (21.26, GunType::BreechLoading, MountType::DeckAndHoist, 1.0),
        }

        // Test armor_barb_wgt {{{3
        macro_rules! test_armor_barb_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun_kind, mount_kind, armor_barb) = $value;

                        let mut btry = Battery::default();

                        btry.kind = gun_kind;
                        btry.mount_kind = mount_kind;
                        btry.armor_barb = Measurement::new(armor_barb, UnitType::LengthSmall, Units::Imperial);
                        btry.diam = Measurement::new(10.0, UnitType::LengthSmall, Units::Imperial);
                        btry.year = 1920;
                        btry.num = 2;

                        // Assume they are all single mounts
                        btry.mount_num = btry.num;
                        btry.groups[0].on = btry.num;
                        btry.groups[1].on = 0;

                        btry.groups[0].layout = GunLayoutType::Single;
                        btry.groups[0].distribution = GunDistributionType::CenterlineEven;

                        let mut hull = Hull::default();
                        hull.freeboard.fc_len = 0.2;

                        hull.freeboard.fd_len = 0.3;
                        hull.freeboard.fd_fwd = Measurement::new(10.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.fd_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.ad_fwd = Measurement::new(20.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.ad_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.qd_len = 0.15;

                        assert_eq!(expected, to_place(btry.armor_barb_wgt(hull), 2));
                    }
                )*
            }
        }
        test_armor_barb_wgt! {
            // name: (armor_barb_wgt, gun_kind, mount_kind, armor_barb)
            armor_barb_wgt_1: (0.35, GunType::BreechLoading, MountType::DeckAndHoist, 1.0),
        }

        // Test wgt_adj {{{3
        macro_rules! test_wgt_adj {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, g0_mounts, g1_mounts) = $value;

                        let mut btry = Battery::default();
                        btry.mount_num = g0_mounts + g1_mounts;

                        btry.groups[0].on = g0_mounts;
                        btry.groups[1].on = g1_mounts;
                        btry.groups[0].layout = GunLayoutType::Twin;
                        btry.groups[1].layout = GunLayoutType::Twin;

                        assert_eq!(expected, to_place(btry.wgt_adj(), 5));
                    }
                )*
            }
        }
        test_wgt_adj! {
            // name: (wgt_adj, g0_mounts, g1_mounts)
            wgt_adj_no_mounts: (0.0, 0, 0),
            wgt_adj_test: (0.75, 1, 2),
        }

        // Test date_factor {{{3
        macro_rules! test_date_factor {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, year) = $value;

                        let mut btry = Battery::default();
                        btry.year = year;

                        assert_eq!(expected, to_place(btry.date_factor(), 5));
                    }
                )*
            }
        }
        test_date_factor! {
            // name: (date_factor, year)
            date_factor_sm: (0.99247, 1889),
        }

        // Test shell_wgt_est {{{3
        macro_rules! test_shell_wgt_est {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, len) = $value;

                        let mut btry = Battery::default();
                        btry.len = len;
                        btry.diam = Measurement::new(10.0, UnitType::LengthSmall, Units::Imperial);
                        btry.year = 1920;

                        assert_eq!(expected, to_place(btry.shell_wgt_est(), 2));
                    }
                )*
            }
        }
        test_shell_wgt_est! {
            // name: (shell_wgt_est, len)
            shell_wgt_est_sm: (493.06, 44.0),
            shell_wgt_est_45: (504.26, 45.0),
            shell_wgt_est_lg: (515.47, 46.0),
        }

        // Test gun_wgt {{{3
        macro_rules! test_gun_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, diam, len) = $value;

                        let mut btry = Battery::default();
                        btry.len = len;
                        btry.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                        btry.num = 1;
                        btry.year = 1920;

                        assert_eq!(expected, to_place(btry.gun_wgt(), 2));
                    }
                )*
            }
        }
        test_gun_wgt! {
            // name: (gun_wgt, diam, len)
            gun_wgt_cal_eq_0: (0.0, 0.0, 0.0),
            gun_wgt_test: (28.06, 10.0, 45.0),
        }

        // Test mount_wgt {{{3
        macro_rules! test_mount_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount_kind, diam) = $value;

                        let mut btry = Battery::default();
                        btry.mount_kind = mount_kind;
                        btry.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                        btry.len = 45.0;
                        btry.num = 1;
                        btry.year = 1920;
                        btry.kind = GunType::AntiAir;

                        btry.groups[0].on = 1;
                        btry.groups[1].on = 0;
                        btry.groups[0].layout = GunLayoutType::Single;
                        btry.groups[1].layout = GunLayoutType::Single;

                        btry.mount_num = btry.groups[0].num_mounts() +
                            btry.groups[1].num_mounts();

                        assert_eq!(expected, to_place(btry.mount_wgt(), 2));
                    }
                )*
            }
        }
        test_mount_wgt! {
            // name: (mount_wgt, mount_kind, diam)
            mount_wgt_cal_eq_0: (0.0, MountType::Broadside, 0.0),
            mount_wgt_sm_mount: (47.19, MountType::Broadside, 10.0),
            mount_wgt_lg_mount: (111.87, MountType::ColesTurret, 10.0),
            mount_wgt_lg_cal: (112.97, MountType::ColesTurret, 11.0),
            mount_wgt_sm_cal: (0.06, MountType::ColesTurret, 1.0),
        }

        // Test broadside_wgt {{{3
        macro_rules! test_broadside_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, num) = $value;

                        let mut btry = Battery::default();
                        btry.set_shell_wgt(10.0, Units::Imperial);
                        btry.num = num;

                        assert_eq!(expected, btry.broadside_wgt());
                    }
                )*
            }
        }
        test_broadside_wgt! {
            // name: (broadside_wgt, num)
            broadside_wgt_test: (100.0, 10),
        }

        // Test mag_wgt {{{3
        macro_rules! test_mag_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, num, shells, shell_wgt) = $value;

                        let mut btry = Battery::default();
                        btry.num = num;
                        btry.shells = shells;
                        btry.set_shell_wgt(shell_wgt, Units::Imperial);

                        assert_eq!(to_place(expected, 2), to_place(btry.mag_wgt(), 2));
                    }
                )*
            }
        }
        test_mag_wgt! {
            // name: (mag_wgt, num, shells, shell_wgt)
            mag_wgt_test_1: ((10 * 10 * 100) as f64 / Ship::POUND2TON * (1.0 + Battery::CORDITE_FACTOR), 10, 10, 100.0),
            mag_wgt_test_2: (1.0 + Battery::CORDITE_FACTOR, 1, 1, Ship::POUND2TON),
        }
    }

    // GunType {{{2
    mod gun_type {
        use super::*;

        // Test armor_face_wgt {{{3
        macro_rules! test_armor_face_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun, back_armor) = $value;

                        assert_eq!(expected, gun.armor_face_wgt(back_armor));
                    }
                )*
            }
        }

        test_armor_face_wgt! {
            // name:         (factor, mount, back_armor)
            face_wgt_muzzle: (1.0, GunType::MuzzleLoading, 1.0),
            face_wgt_breech: (1.0, GunType::BreechLoading, 1.0),
            face_wgt_qf:     (1.0, GunType::QuickFiring, 1.0),
            face_wgt_aa:     (0.333, GunType::AntiAir, 1.0),
            face_wgt_dp:     (1.0, GunType::DualPurpose, 1.0),
            face_wgt_rapdi:  (1.0, GunType::RapidFire, 1.0),
            face_wgt_mg:     (1.0, GunType::MachineGun, 1.0),

            // name:                 (factor, mount, back_armor)
            face_wgt_muzzle_no_back: (1.0, GunType::MuzzleLoading, 0.0),
            face_wgt_breech_no_back: (1.0, GunType::BreechLoading, 0.0),
            face_wgt_qf_no_back:     (1.0, GunType::QuickFiring, 0.0),
            face_wgt_aa_no_back:     (0.333, GunType::AntiAir, 0.0),
            face_wgt_dp_no_back:     (1.0, GunType::DualPurpose, 0.0),
            face_wgt_rapdi_no_back:  (1.0, GunType::RapidFire, 0.0),
            face_wgt_mg_no_back:     (0.333, GunType::MachineGun, 0.0),
        }
        // Test wgt_sm {{{3
        macro_rules! test_wgt_sm {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun) = $value;

                        assert_eq!(expected, gun.wgt_sm());
                    }
                )*
            }
        }

        test_wgt_sm! {
            // name:       (factor, gun)
            wgt_sm_muzzle: (0.9, GunType::MuzzleLoading),
            wgt_sm_breech: (1.0, GunType::BreechLoading),
            wgt_sm_qf:     (1.35, GunType::QuickFiring),
            wgt_sm_aa:     (1.44, GunType::AntiAir),
            wgt_sm_dp:     (1.57, GunType::DualPurpose),
            wgt_sm_rf:     (2.16, GunType::RapidFire),
            wgt_sm_mg:     (1.0, GunType::MachineGun),
        }

        // Test wgt_lg {{{3
        macro_rules! test_wgt_lg {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun) = $value;

                        assert_eq!(expected, gun.wgt_lg());
                    }
                )*
            }
        }

        test_wgt_lg! {
            // name:       (factor, gun)
            wgt_lg_muzzle: (0.98, GunType::MuzzleLoading),
            wgt_lg_breech: (1.0, GunType::BreechLoading),
            wgt_lg_qf:     (1.0, GunType::QuickFiring),
            wgt_lg_aa:     (1.0, GunType::AntiAir),
            wgt_lg_dp:     (1.1, GunType::DualPurpose),
            wgt_lg_rf:     (1.5, GunType::RapidFire),
            wgt_lg_mg:     (1.0, GunType::MachineGun),
        }

        // Test Display {{{3
        macro_rules! test_display {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, gun) = $value;

                        assert_eq!(expected, format!("{}", gun));
                    }
                )*
            }
        }

        test_display! {
            // name:               (display, gun)
            display_muzzle:        ("Muzzle loading", GunType::MuzzleLoading),
            display_breech:        ("Breech loading", GunType::BreechLoading),
            display_qf:            ("Quick firing", GunType::QuickFiring),
            display_aa:            ("Anti-air", GunType::AntiAir),
            display_dp:            ("Dual purpose", GunType::DualPurpose),
            display_rf:            ("Auto rapid fire", GunType::RapidFire),
            display_mg:            ("Machine", GunType::MachineGun),
        }

        // Test From<&str> {{{3
        macro_rules! test_from_str {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, index) = $value;

                        assert_eq!(expected, index.into());
                    }
                )*
            }
        }

        test_from_str! {
            // name:            (gun, index)
            from_str_muzzle:   (GunType::MuzzleLoading, "0"),
            from_str_breech:   (GunType::BreechLoading, "1"),
            from_str_qf:       (GunType::QuickFiring, "2"),
            from_str_aa:       (GunType::AntiAir, "3"),
            from_str_dp:       (GunType::DualPurpose, "4"),
            from_str_rf:       (GunType::RapidFire, "5"),
            from_str_mg:       (GunType::MachineGun, "6"),
            from_str_default:  (GunType::BreechLoading, "9"),
        }

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(GunType::from("0"), GunType::MuzzleLoading);
            assert_eq!(GunType::from("1"), GunType::BreechLoading);
            assert_eq!(GunType::from("2"), GunType::QuickFiring);
            assert_eq!(GunType::from("3"), GunType::AntiAir);
            assert_eq!(GunType::from("4"), GunType::DualPurpose);
            assert_eq!(GunType::from("5"), GunType::RapidFire);
            assert_eq!(GunType::from("6"), GunType::MachineGun);
        }

        #[test]
        fn index_roundtrip() {
            for v in GunType::ALL {
                assert_eq!(GunType::from_index(v.index()), *v);
                assert_eq!(GunType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(GunType::from("99"), GunType::default());
            assert_eq!(GunType::from("abc"), GunType::default());
            assert_eq!(GunType::from(""), GunType::default());
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = GunType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["Muzzle loading gun", "Breech loading gun", "Quick firing gun",
                 "Anti-air gun", "Dual purpose gun", "Auto rapid fire gun", "Machine gun"]
            );
        }
    }

    // MountType {{{2
    mod mount_type {
        use super::*;

        // Test armor_wgt_adj {{{3
        macro_rules! test_armor_wgt_adj {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount) = $value;

                        assert_eq!(expected, mount.wgt_adj());
                    }
                )*
            }
        }

        test_armor_wgt_adj! {
            // name:             (factor, mount)
            wgt_adj_broad:       (0.5, MountType::Broadside),
            wgt_adj_coles:       (1.0, MountType::ColesTurret),
            wgt_adj_open_barb:   (0.7, MountType::OpenBarbette),
            wgt_adj_closed_barb: (1.0, MountType::ClosedBarbette),
            wgt_adj_deckhoist:   (1.0, MountType::DeckAndHoist),
            wgt_adj_deck:        (0.5, MountType::Deck),
            wgt_adj_casemate:    (0.5, MountType::Casemate),
        }

        // Test armor_wgt {{{3
        macro_rules! test_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount) = $value;

                        assert_eq!(expected, mount.wgt());
                    }
                )*
            }
        }

        test_wgt! {
            // name:         (factor, mount)
            wgt_broad:       (0.83, MountType::Broadside),
            wgt_coles:       (3.5, MountType::ColesTurret),
            wgt_open_barb:   (3.33, MountType::OpenBarbette),
            wgt_closed_barb: (3.5, MountType::ClosedBarbette),
            wgt_deckhoist:   (3.15, MountType::DeckAndHoist),
            wgt_deck:        (1.08, MountType::Deck),
            wgt_casemate:    (1.08, MountType::Casemate),
        }

        // Test armor_face_wgt {{{3
        macro_rules! test_armor_face_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount, back_armor) = $value;

                        assert_eq!(expected, mount.armor_face_wgt(back_armor));
                    }
                )*
            }
        }

        test_armor_face_wgt! {
            // name:              (factor, mount, back_armor)
            face_wgt_broad:       (1.0, MountType::Broadside, 1.0),
            face_wgt_coles:       (PI / 2.0, MountType::ColesTurret, 1.0),
            face_wgt_open_barb:   (0.0, MountType::OpenBarbette, 1.0),
            face_wgt_closed_barb: (0.5, MountType::ClosedBarbette, 1.0),
            face_wgt_deckhoist:   (0.5, MountType::DeckAndHoist, 1.0),
            face_wgt_deck:        (0.5, MountType::Deck, 1.0),
            face_wgt_casemate:    (1.0, MountType::Casemate, 1.0),

            // name:                      (factor, mount, back_armor)
            face_wgt_broad_no_back:       (1.0, MountType::Broadside, 0.0),
            face_wgt_coles_no_back:       (PI / 2.0, MountType::ColesTurret, 0.0),
            face_wgt_open_barb_no_back:   (0.0, MountType::OpenBarbette, 0.0),
            face_wgt_closed_barb_no_back: (1.5, MountType::ClosedBarbette, 0.0),
            face_wgt_deckhoist_no_back:   (1.5, MountType::DeckAndHoist, 0.0),
            face_wgt_deck_no_back:        (1.5, MountType::Deck, 0.0),
            face_wgt_casemate_no_back:    (1.0, MountType::Casemate, 0.0),
        }

        // Test Display {{{3
        macro_rules! test_display {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mount) = $value;

                        assert_eq!(expected, format!("{}", mount));
                    }
                )*
            }
        }

        test_display! {
            // name:               (display, mount)
            display_broad:        ("broadside", MountType::Broadside),
            display_coles:        ("Coles/Ericsson turret", MountType::ColesTurret),
            display_open_barb:    ("open barbette", MountType::OpenBarbette),
            display_closed_barb:  ("turret on barbette", MountType::ClosedBarbette),
            display_deckhoist:    ("deck and hoist", MountType::DeckAndHoist),
            display_deck:         ("deck", MountType::Deck),
            display_casemate:     ("casemate", MountType::Casemate),
        }

        // Test From<&str> {{{3
        macro_rules! test_from_str {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, index) = $value;

                        assert_eq!(expected, index.into());
                    }
                )*
            }
        }

        test_from_str! {
            // name:            (mount, index)
            from_str_broad:    (MountType::Broadside, "0"),
            from_str_coles:    (MountType::ColesTurret, "1"),
            from_str_open_barb: (MountType::OpenBarbette, "2"),
            from_str_closed_barb: (MountType::ClosedBarbette, "3"),
            from_str_deckhoist: (MountType::DeckAndHoist, "4"),
            from_str_deck:     (MountType::Deck, "5"),
            from_str_casemate: (MountType::Casemate, "6"),
            from_str_default:  (MountType::Deck, "9"),
        }

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(MountType::from("0"), MountType::Broadside);
            assert_eq!(MountType::from("1"), MountType::ColesTurret);
            assert_eq!(MountType::from("2"), MountType::OpenBarbette);
            assert_eq!(MountType::from("3"), MountType::ClosedBarbette);
            assert_eq!(MountType::from("4"), MountType::DeckAndHoist);
            assert_eq!(MountType::from("5"), MountType::Deck);
            assert_eq!(MountType::from("6"), MountType::Casemate);
        }

        #[test]
        fn index_roundtrip() {
            for v in MountType::ALL {
                assert_eq!(MountType::from_index(v.index()), *v);
                assert_eq!(MountType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(MountType::from("99"), MountType::default());
            assert_eq!(MountType::from("abc"), MountType::default());
            assert_eq!(MountType::from(""), MountType::default());
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = MountType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["in broadside mount", "in Coles/Ericsson turret mount",
                 "in open barbette mount", "in turret on barbette mount",
                 "in deck and hoist mount", "in deck mount", "in casemate mount"]
            );
        }
    }

    // SubBattery {{{2
    mod sub_battery {
        use super::*;

        // Test super_ {{{3
        macro_rules! test_super_ {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, above, two_mounts_up, below, lower_deck) = $value;

                        let mut sub_btry = SubBattery::default();
                        sub_btry.layout = GunLayoutType::Single;

                        sub_btry.above = above;
                        sub_btry.below = below;
                        sub_btry.two_mounts_up = two_mounts_up;
                        sub_btry.lower_deck = lower_deck;

                        assert_eq!(expected, sub_btry.super_());
                    }
                )*
            }
        }
        test_super_! {
            // name:      (super_, above, two_mounts_up, below, lower_deck)
            super_test_1: ( 1, 1, false, 0, false),
            super_test_2: (-1, 0, false, 1, false),
            super_test_3: ( 2, 1, true, 0, true),
            super_test_4: (-2, 0, true, 1, true),
            super_test_5: ( 0, 1, false, 1, false),
            super_test_6: ( 0, 1, true, 1, true),
            super_test_7: (-1, 1, false, 1, true),
            super_test_8: ( 1, 1, true, 1, false),
        }

        // Test diameter_calc {{{3
        macro_rules! test_diameter_calc {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, diam) = $value;

                        let mut sub_btry = SubBattery::default();
                        sub_btry.layout = GunLayoutType::Single;

                        assert_eq!(expected, to_place(sub_btry.diameter_calc(diam), 2));
                    }
                )*
            }
        }
        test_diameter_calc! {
            // name:      (diameter_calc, diam)
            diameter_calc_cal_eq_0: (0.0, 0.0),
            diameter_calc_cal_lt_12: (19.14, 10.0),
            diameter_calc_cal_gt_1:  (12.30, 5.0),
            diameter_calc_cal_sm:  (25.82, 0.5),
        }

        // Test wgt_adj {{{3
        macro_rules! test_wgt_adj {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, num_mounts) = $value;

                        let mut sub_btry = SubBattery::default();
                        sub_btry.layout = GunLayoutType::Single;
                        sub_btry.above = num_mounts;
                        sub_btry.on = 0;
                        sub_btry.below = 0;

                        assert_eq!(expected, to_place(sub_btry.wgt_adj(), 2));
                    }
                )*
            }
        }
        test_wgt_adj! {
            // name:      (wgt_adj, num_mounts)
            wgt_adj_test: (10.0, 10),
        }

        // Test free {{{3
        macro_rules! test_free {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, num_mounts) = $value;

                        let mut sub_btry = SubBattery::default();
                        sub_btry.distribution = GunDistributionType::CenterlineEven;
                        sub_btry.above = num_mounts;
                        sub_btry.on = 0;
                        sub_btry.below = 0;

                        let mut hull = Hull::default();
                        hull.freeboard.fc_len = 0.2;

                        hull.freeboard.fd_len = 0.3;
                        hull.freeboard.fd_fwd = Measurement::new(10.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.fd_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.ad_fwd = Measurement::new(20.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.ad_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.qd_len = 0.15;

                        assert_eq!(expected, to_place(sub_btry.free(hull), 2));
                    }
                )*
            }
        }
        test_free! {
            // name:   (free, num_mounts)
            free_test: (35.0, 5),
        }
    }

    // GunDistributionType {{{2
    mod gun_dist_type {
        use super::*;

        // Test g1_gun_position {{{3
        macro_rules! test_gun_position {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, dist) = $value;
                        let fd_len = 0.2; let ad_len = 0.2;

                        assert_eq!(expected, (
                                to_place(dist.g1_gun_position(fd_len, ad_len), 2),
                                to_place(dist.g2_gun_position(fd_len, ad_len), 2)
                                ));
                    }
                )*
            }
        }

        test_gun_position! {
            // name:                       ((g1_pos, g2_pos), dist)
            gun_position_center:        ((1.0, 0.0), GunDistributionType::CenterlineEven),
            gun_position_center_end_fd: ((1.0, 0.0), GunDistributionType::CenterlineEndsFD),
            gun_position_center_end_ad: ((1.0, 0.0), GunDistributionType::CenterlineEndsAD),
            gun_position_sides:         ((1.0, 0.0), GunDistributionType::SidesEven),
            gun_position_sides_end_fd:  ((1.0, 0.0), GunDistributionType::SidesEndsFD),
            gun_position_sides_end_ad:  ((1.0, 0.0), GunDistributionType::SidesEndsAD),

            gun_position_center_fd_fwd: ((0.05, 0.05), GunDistributionType::CenterlineFDFwd),
            gun_position_center_fd:     ((0.1, 0.1), GunDistributionType::CenterlineFD),
            gun_position_center_fd_aft: ((0.15, 0.15), GunDistributionType::CenterlineFDAft),
            gun_position_center_ad_fwd: ((0.05, 0.05), GunDistributionType::CenterlineADFwd),
            gun_position_center_ad:     ((0.1, 0.1), GunDistributionType::CenterlineAD),
            gun_position_center_ad_aft: ((0.15, 0.15), GunDistributionType::CenterlineADAft),

            gun_position_sides_fd_fwd:  ((0.05, 0.05), GunDistributionType::SidesFDFwd),
            gun_position_sides_fd:      ((0.1, 0.1), GunDistributionType::SidesFD),
            gun_position_sides_fd_aft:  ((0.15, 0.15), GunDistributionType::SidesFDAft),
            gun_position_sides_ad_fwd:  ((0.05, 0.05), GunDistributionType::SidesADFwd),
            gun_position_sides_ad:      ((0.1, 0.1), GunDistributionType::SidesAD),
            gun_position_sides_ad_aft:  ((0.15, 0.15), GunDistributionType::SidesADAft),
        }

        // Test mounts_fwd {{{3
        macro_rules! test_mounts_fwd {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, tot, fwd_len, dist) = $value;

                        assert_eq!(expected, dist.mounts_fwd(tot, fwd_len));
                    }
                )*
            }
        }

        test_mounts_fwd! {
            // name:                    (fwd, tot, fwd_len, mount)
            mounts_fwd_center_1:        (1, 1, 0.5, GunDistributionType::CenterlineEven),
            mounts_fwd_center_2:        (2, 3, 0.5, GunDistributionType::CenterlineEven),
            mounts_fwd_center_3:        (0, 1, 0.4, GunDistributionType::CenterlineEven),
            mounts_fwd_center_4:        (1, 3, 0.4, GunDistributionType::CenterlineEven),
            mounts_fwd_center_end_fd_1: (1, 1, 0.0, GunDistributionType::CenterlineEndsFD),
            mounts_fwd_center_end_fd_2: (2, 3, 0.0, GunDistributionType::CenterlineEndsFD),
            mounts_fwd_center_end_ad_1: (0, 1, 0.0, GunDistributionType::CenterlineEndsAD),
            mounts_fwd_center_end_ad_2: (1, 3, 0.0, GunDistributionType::CenterlineEndsAD),
            mounts_fwd_center_fd_fwd:   (3, 3, 0.0, GunDistributionType::CenterlineFDFwd),
            mounts_fwd_center_fd:       (3, 3, 0.0, GunDistributionType::CenterlineFD),
            mounts_fwd_center_fd_aft:   (3, 3, 0.0, GunDistributionType::CenterlineFDAft),
            mounts_fwd_center_ad_fwd:   (3, 3, 0.0, GunDistributionType::CenterlineADFwd),
            mounts_fwd_center_ad:       (0, 3, 0.0, GunDistributionType::CenterlineAD),
            mounts_fwd_center_ad_aft:   (0, 3, 0.0, GunDistributionType::CenterlineADAft),

            mounts_fwd_sides_1:         (1, 1, 0.5, GunDistributionType::SidesEven),
            mounts_fwd_sides_2:         (2, 3, 0.5, GunDistributionType::SidesEven),
            mounts_fwd_sides_3:         (0, 1, 0.4, GunDistributionType::SidesEven),
            mounts_fwd_sides_4:         (1, 3, 0.4, GunDistributionType::SidesEven),
            mounts_fwd_sides_end_fd_1:  (1, 1, 0.0, GunDistributionType::SidesEndsFD),
            mounts_fwd_sides_end_fd_2:  (2, 3, 0.0, GunDistributionType::SidesEndsFD),
            mounts_fwd_sides_end_ad_1:  (0, 1, 0.0, GunDistributionType::SidesEndsAD),
            mounts_fwd_sides_end_ad_2:  (1, 3, 0.0, GunDistributionType::SidesEndsAD),
            mounts_fwd_sides_fd_fwd:    (3, 3, 0.0, GunDistributionType::SidesFDFwd),
            mounts_fwd_sides_fd:        (3, 3, 0.0, GunDistributionType::SidesFD),
            mounts_fwd_sides_fd_aft:    (3, 3, 0.0, GunDistributionType::SidesFDAft),
            mounts_fwd_sides_ad_fwd:    (0, 3, 0.0, GunDistributionType::SidesADFwd),
            mounts_fwd_sides_ad:        (0, 3, 0.0, GunDistributionType::SidesAD),
            mounts_fwd_sides_ad_aft:    (0, 3, 0.0, GunDistributionType::SidesADAft),

            mounts_fwd_none:            (0, 5, 0.5, GunDistributionType::None),
        }

        // Test free {{{3
        macro_rules! test_free {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, num, dist) = $value;
                        let mut hull = Hull::default();
                        hull.freeboard.fc_len = 0.2;

                        hull.freeboard.fd_len = 0.3;
                        hull.freeboard.fd_fwd = Measurement::new(10.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.fd_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.ad_fwd = Measurement::new(20.0, UnitType::LengthLong, Units::Imperial);
                        hull.freeboard.ad_aft = Measurement::new(0.0, UnitType::LengthLong, Units::Imperial);

                        hull.freeboard.qd_len = 0.15;

                        assert_eq!(expected, to_place(dist.free(num, hull), 3));
                    }
                )*
            }
        }

        test_free! {
            // name:       (free, mounts, fd, ad, fd_fwd, fd_aft, ad_fwd, ad_aft, dist)
            free_tot_eq_0: (0.0, 0, GunDistributionType::CenterlineEven),
            free_case_1_1: (7.0, 5, GunDistributionType::CenterlineEven),
            free_case_1_2: (7.0, 5, GunDistributionType::SidesEven),
            free_case_2_1: (6.0, 5, GunDistributionType::CenterlineEndsFD),
            free_case_2_2: (5.5, 5, GunDistributionType::CenterlineEndsAD),
            free_case_2_3: (6.0, 5, GunDistributionType::SidesEndsFD),
            free_case_2_4: (5.5, 5, GunDistributionType::SidesEndsAD),
            free_case_3_1: (8.0, 5, GunDistributionType::CenterlineFDFwd),
            free_case_3_2: (8.0, 5, GunDistributionType::SidesFDFwd),
            free_case_4_1: (5.0, 5, GunDistributionType::CenterlineFD),
            free_case_4_2: (5.0, 5, GunDistributionType::SidesFD),
            free_case_5_1: (2.0, 5, GunDistributionType::CenterlineFDAft),
            free_case_5_2: (2.0, 5, GunDistributionType::SidesFDAft),
            free_case_6_1: (0.0, 5, GunDistributionType::CenterlineADFwd),
            free_case_6_2: (16.0, 5, GunDistributionType::SidesADFwd),
            free_case_7_1: (10.0, 5, GunDistributionType::CenterlineAD),
            free_case_7_2: (10.0, 5, GunDistributionType::SidesAD),
            free_case_8_1: (4.0, 5, GunDistributionType::CenterlineADAft),
            free_case_8_2: (4.0, 5, GunDistributionType::SidesADAft),
            free_none:     (0.0, 5, GunDistributionType::None),
        }

        // Test desc {{{3
        macro_rules! test_desc {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, mounts, fwd_len, dist) = $value;

                        assert_eq!(expected, dist.desc(mounts, fwd_len));
                    }
                )*
            }
        }

        test_desc! {
            // name:                     (desc, mounts, fwd_len, dist)
            desc_even_1_fwd:             ("centreline amidships (forward deck)", 1, 0.5, GunDistributionType::CenterlineEven),
            desc_even_1_aft:             ("centreline amidships (aft deck)", 1, 0.4, GunDistributionType::CenterlineEven),
            desc_even_multi:             ("centreline, evenly spread", 2, 0.5, GunDistributionType::CenterlineEven),
            desc_end_fd_1:               ("centreline forward", 1, 0.5, GunDistributionType::CenterlineEndsFD),
            desc_end_fd_even:            ("centreline ends, evenly spread", 2, 0.5, GunDistributionType::CenterlineEndsFD),
            desc_end_fd_odd:             ("centreline ends, majority forward", 3, 0.5, GunDistributionType::CenterlineEndsFD),
            desc_end_ad_1:               ("centreline aft", 1, 0.5, GunDistributionType::CenterlineEndsAD),
            desc_end_ad_even:            ("centreline ends, evenly spread", 2, 0.5, GunDistributionType::CenterlineEndsAD),
            desc_end_ad_odd:             ("centreline ends, majority aft", 3, 0.5, GunDistributionType::CenterlineEndsAD),
            desc_fd_fwd:                 ("centreline, forward deck forward", 2, 0.5, GunDistributionType::CenterlineFDFwd),
            desc_fd_1:                   ("centreline, forward deck centre", 1, 0.5, GunDistributionType::CenterlineFD),
            desc_fd_multi:               ("centreline, forward evenly spread", 2, 0.5, GunDistributionType::CenterlineFD),
            desc_fd_aft:                 ("centreline, forward deck aft", 2, 0.5, GunDistributionType::CenterlineFDAft),
            desc_ad_fwd:                 ("centreline, aft deck forward", 2, 0.5, GunDistributionType::CenterlineADFwd),
            desc_ad_1:                   ("centreline, aft deck centre", 1, 0.5, GunDistributionType::CenterlineAD),
            desc_ad_multi:               ("centreline, aft evenly spread", 2, 0.5, GunDistributionType::CenterlineAD),
            desc_ad_aft:                 ("centreline, aft deck aft", 2, 0.5, GunDistributionType::CenterlineADAft),
            desc_sides_few:              ("sides amidships", 2, 0.5, GunDistributionType::SidesEven),
            desc_sides_multi:            ("sides, evenly spread", 3, 0.5, GunDistributionType::SidesEven),
            desc_sides_end_fd_few:       ("sides, forward", 2, 0.5, GunDistributionType::SidesEndsFD),
            desc_sides_end_fd_even:      ("side ends, evenly spread", 4, 0.5, GunDistributionType::SidesEndsFD),
            desc_sides_end_fd_odd:       ("side ends, majority forward", 6, 0.5, GunDistributionType::SidesEndsFD),
            desc_sides_end_ad_few:       ("sides aft", 2, 0.5, GunDistributionType::SidesEndsAD),
            desc_sides_end_ad_even:      ("side ends, evenly spread", 4, 0.5, GunDistributionType::SidesEndsAD),
            desc_sides_end_ad_odd:       ("side ends, majority aft", 6, 0.5, GunDistributionType::SidesEndsAD),
            desc_sides_fd_fwd:           ("sides, forward deck forward", 2, 0.5, GunDistributionType::SidesFDFwd),
            desc_sides_fd_few:           ("sides, forward deck centre", 2, 0.5, GunDistributionType::SidesFD),
            desc_sides_fd_multi:         ("sides, forward evenly spread", 3, 0.5, GunDistributionType::SidesFD),
            desc_sides_fd_aft:           ("sides, forward deck aft", 2, 0.5, GunDistributionType::SidesFDAft),
            desc_sides_ad_fwd:           ("sides, aft deck forward", 2, 0.5, GunDistributionType::SidesADFwd),
            desc_sides_ad_few:           ("sides, aft deck centre", 2, 0.5, GunDistributionType::SidesAD),
            desc_sides_ad_multi:         ("sides, aft evenly spread", 3, 0.5, GunDistributionType::SidesAD),
            desc_sides_ad_aft:           ("sides, aft deck aft", 2, 0.5, GunDistributionType::SidesADAft),
            desc_none:                   ("layout not set", 0, 0.5, GunDistributionType::None),
        }

        // Test Display {{{3
        macro_rules! test_display {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, dist) = $value;

                        assert_eq!(expected, format!("{}", dist));
                    }
                )*
            }
        }

        test_display! {
            // name:                  (display, dist)
            display_even:             ("Centreline - distributed", GunDistributionType::CenterlineEven),
            display_end_fd:           ("Centreline - ends (fore >= aft)", GunDistributionType::CenterlineEndsFD),
            display_end_ad:           ("Centreline - ends (aft >= fore)", GunDistributionType::CenterlineEndsAD),
            display_fd_fwd:           ("Centreline - fore deck forward", GunDistributionType::CenterlineFDFwd),
            display_fd:               ("Centreline - fore deck", GunDistributionType::CenterlineFD),
            display_fd_aft:           ("Centreline - fore deck aft", GunDistributionType::CenterlineFDAft),
            display_ad_fwd:           ("Centreline - aft deck forward", GunDistributionType::CenterlineADFwd),
            display_ad:               ("Centreline - aft deck", GunDistributionType::CenterlineAD),
            display_ad_aft:           ("Centreline - aft deck aft", GunDistributionType::CenterlineADAft),
            display_sides:            ("Sides - distributed", GunDistributionType::SidesEven),
            display_sides_end_fd:     ("Sides - ends (fore >= aft)", GunDistributionType::SidesEndsFD),
            display_sides_end_ad:     ("Sides - ends (aft >= fore)", GunDistributionType::SidesEndsAD),
            display_sides_fd_fwd:     ("Sides - fore deck forward", GunDistributionType::SidesFDFwd),
            display_sides_fd:         ("Sides - fore deck", GunDistributionType::SidesFD),
            display_sides_fd_aft:     ("Sides - fore deck aft", GunDistributionType::SidesFDAft),
            display_sides_ad_fwd:     ("Sides - aft deck forward", GunDistributionType::SidesADFwd),
            display_sides_ad:         ("Sides - aft deck", GunDistributionType::SidesAD),
            display_sides_ad_aft:     ("Sides - aft deck aft", GunDistributionType::SidesADAft),
            display_none:             ("None", GunDistributionType::None),
        }

        // Test From<&str> {{{3
        macro_rules! test_from_str {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, index) = $value;

                        assert_eq!(expected, index.into());
                    }
                )*
            }
        }

        test_from_str! {
            // name:               (dist, index)
            from_str_even:         (GunDistributionType::CenterlineEven, "0"),
            from_str_end_fd:       (GunDistributionType::CenterlineEndsFD, "1"),
            from_str_end_ad:       (GunDistributionType::CenterlineEndsAD, "2"),
            from_str_fd_fwd:       (GunDistributionType::CenterlineFDFwd, "3"),
            from_str_fd:           (GunDistributionType::CenterlineFD, "4"),
            from_str_fd_aft:       (GunDistributionType::CenterlineFDAft, "5"),
            from_str_ad_fwd:       (GunDistributionType::CenterlineADFwd, "6"),
            from_str_ad:           (GunDistributionType::CenterlineAD, "7"),
            from_str_ad_aft:       (GunDistributionType::CenterlineADAft, "8"),
            from_str_sides:        (GunDistributionType::SidesEven, "9"),
            from_str_sides_end_fd: (GunDistributionType::SidesEndsFD, "10"),
            from_str_sides_end_ad: (GunDistributionType::SidesEndsAD, "11"),
            from_str_sides_fd_fwd: (GunDistributionType::SidesFDFwd, "12"),
            from_str_sides_fd:     (GunDistributionType::SidesFD, "13"),
            from_str_sides_fd_aft: (GunDistributionType::SidesFDAft, "14"),
            from_str_sides_ad_fwd: (GunDistributionType::SidesADFwd, "15"),
            from_str_sides_ad:     (GunDistributionType::SidesAD, "16"),
            from_str_sides_ad_aft: (GunDistributionType::SidesADAft, "17"),
            from_str_default:      (GunDistributionType::None, "99"),
        }

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(GunDistributionType::from("0"),  GunDistributionType::CenterlineEven);
            assert_eq!(GunDistributionType::from("9"),  GunDistributionType::SidesEven);
            assert_eq!(GunDistributionType::from("17"), GunDistributionType::SidesADAft);
        }

        #[test]
        fn index_roundtrip() {
            for v in GunDistributionType::ALL {
                assert_eq!(GunDistributionType::from_index(v.index()), *v);
                assert_eq!(GunDistributionType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(GunDistributionType::from("99"), GunDistributionType::default());
            assert_eq!(GunDistributionType::from("abc"), GunDistributionType::default());
            assert_eq!(GunDistributionType::from(""), GunDistributionType::default());
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = GunDistributionType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["Centreline - distributed", "Centreline - ends (fore >= aft)",
                 "Centreline - ends (aft >= fore)", "Centreline - fore deck forward",
                 "Centreline - fore deck", "Centreline - fore deck aft",
                 "Centreline - aft deck forward", "Centreline - aft deck",
                 "Centreline - aft deck aft", "Sides - distributed",
                 "Sides - ends (fore >= aft)", "Sides - ends (aft >= fore)",
                 "Sides - fore deck forward", "Sides - fore deck",
                 "Sides - fore deck aft", "Sides - aft deck forward",
                 "Sides - aft deck", "Sides - aft deck aft", "None"]
            );
        }
    }

    // GunLayoutType {{{2
    mod gunlayouttype {
        use super::*;

        // Test Display {{{3
        macro_rules! test_display {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, layout) = $value;

                        assert_eq!(expected, format!("{}", layout));
                    }
                )*
            }
        }

        test_display! {
            // name:            (display, layout)
            display_single:    ("Single",          GunLayoutType::Single),
            display_twin_2row: ("2 row, twin",     GunLayoutType::Twin2Row),
            display_quad_4row: ("4 row, quad",     GunLayoutType::Quad4Row),
            display_twin:      ("Twin",            GunLayoutType::Twin),
            display_two_gun:   ("2-gun",           GunLayoutType::TwoGun),
            display_quad_2row: ("2 row, quad",     GunLayoutType::Quad2Row),
            display_triple:    ("Triple",          GunLayoutType::Triple),
            display_three_gun: ("3-gun",           GunLayoutType::ThreeGun),
            display_sex_2row:  ("2 row, sextuple", GunLayoutType::Sex2Row),
            display_quad:      ("quad",            GunLayoutType::Quad),
            display_four_gun:  ("4-gun",           GunLayoutType::FourGun),
            display_oct_2row:  ("2 row, octuple",  GunLayoutType::Oct2Row),
            display_quint:     ("quintuple",       GunLayoutType::Quint),
            display_five_gun:  ("5-gun",           GunLayoutType::FiveGun),
            display_dec_2row:  ("2 row, decuple",  GunLayoutType::Dec2Row),
        }

        // Test From<&str> {{{3
        macro_rules! test_from_str {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, index) = $value;

                        assert_eq!(expected, index.into());
                    }
                )*
            }
        }

        test_from_str! {
            // name:              (layout, index)
            from_str_single:     (GunLayoutType::Single, "0"),
            from_str_twin_2row:  (GunLayoutType::Twin2Row, "1"),
            from_str_quad_4row:  (GunLayoutType::Quad4Row, "2"),
            from_str_twin:       (GunLayoutType::Twin, "3"),
            from_str_two_gun:    (GunLayoutType::TwoGun, "4"),
            from_str_quad_2row:  (GunLayoutType::Quad2Row, "5"),
            from_str_triple:     (GunLayoutType::Triple, "6"),
            from_str_three_gun:  (GunLayoutType::ThreeGun, "7"),
            from_str_sex_2row:   (GunLayoutType::Sex2Row, "8"),
            from_str_quad:       (GunLayoutType::Quad, "9"),
            from_str_four_gun:   (GunLayoutType::FourGun, "10"),
            from_str_oct_2row:   (GunLayoutType::Oct2Row, "11"),
            from_str_quint:      (GunLayoutType::Quint, "12"),
            from_str_five_gun:   (GunLayoutType::FiveGun, "13"),
            from_str_dec_2row:   (GunLayoutType::Dec2Row, "14"),
            from_str_default:    (GunLayoutType::Single, "99"),
        }

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(GunLayoutType::from("0"),  GunLayoutType::Single);
            assert_eq!(GunLayoutType::from("8"),  GunLayoutType::Sex2Row);
            assert_eq!(GunLayoutType::from("14"), GunLayoutType::Dec2Row);
        }

        #[test]
        fn index_roundtrip() {
            for v in GunLayoutType::ALL {
                assert_eq!(GunLayoutType::from_index(v.index()), *v);
                assert_eq!(GunLayoutType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(GunLayoutType::from("99"), GunLayoutType::default());
            assert_eq!(GunLayoutType::from("abc"), GunLayoutType::default());
            assert_eq!(GunLayoutType::from(""), GunLayoutType::default());
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = GunLayoutType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["Single mount", "2 row twin mount", "4 row quad mount", "Twin mount",
                 "2-gun mount", "2 row quad mount", "Triple mount", "3-gun mount",
                 "2 row sextuple mount", "Quad mount", "4-gun mount",
                 "2 row octuple mount", "Quintuple mount", "5-gun mount",
                 "2 row decuple mount"]
            );
        }
    }
}
