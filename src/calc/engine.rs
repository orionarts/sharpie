use crate::calc::{Hull, YEAR_MAX};

use bitflags::{bitflags, bitflags_match};
use serde::{Deserialize, Serialize};

use std::fmt;

// Engine {{{1
/// The ship's engine and speed and range characteristics.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Engine {
    /// Year engine built.
    pub year: u32,

    /// Type of fuel.
    pub fuel: FuelType,
    /// Type of steam boilers.
    pub boiler: BoilerType,
    /// Type of engine drive.
    pub drive: DriveType,

    /// TODO: Unimplemented
    pub factor: u32,

    /// Maximum speed (not maximum trial speed).
    pub vmax: f64,
    /// Cruising speed.
    pub vcruise: f64,
    /// Maximum range at cruising speed.
    pub range: u32,

    /// Number of propeller shafts.
    ///
    shafts: u32,

    /// Percentage of bunker weight devoted to coal.
    pub pct_coal: f64,

    /// Installed horsepower override in imperial horsepower.
    ///
    /// While `Some`, [`Engine::hp_max`] reports this value instead of the
    /// horsepower required at [`Engine::vmax`], pinning the machinery weight
    /// and derived engine readouts to the locked power (SpringSharp's
    /// "Lock Power"). Managed by the GUI; never persisted (`#[serde(skip)]`).
    ///
    #[serde(skip)]
    pub power_lock: Option<f64>,
}

impl Engine { // {{{2
    /// XXX: I do not know what this is.
    const RANGE: f64 = 7000.0;

    // set_shafts {{{3
    /// Set the number of shafts in the engine and set any
    /// Hull parameters that depend on the number of shafts.
    ///
    pub fn set_shafts(&mut self, shafts: u32, hull: &mut Hull) -> u32 {
        hull.set_shafts(shafts);

        self.shafts = shafts;
        shafts
    }

    // shafts {{{3
    /// Return number of shafts in the engine.
    ///
    pub fn shafts(&self) -> u32 {
        self.shafts
    }

    // hp {{{3
    /// Horsepower required to achieve a given speed.
    ///
    pub(crate) fn hp(&self, v: f64, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        let len_hp =
            if v <= 15.0 {
                lwl - (leff - lwl)
            } else if v >= 25.0 {
                leff
            } else {
                (leff - lwl) * ((v - 20.0) / 5.0) + lwl
            };

        if len_hp == 0.0 { return 0.0; }

        let hp = (d.powf(2.0/3.0) / len_hp * cs * v.powf(4.0) + 0.01 * ws * v.powf(1.83)) *
            v / 184.1666667;

        hp * if self.year < 1890 {
                1.0 + (1890 - self.year) as f64 / 100.0
            } else {
                1.0
            }
    }

    // hp_max {{{3
    /// Horsepower required to achieve maximum speed.
    ///
    /// When the power is locked the installed horsepower override is
    /// returned instead, matching SpringSharp's frozen `hpMax`.
    ///
    pub fn hp_max(&self, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        match self.power_lock {
            Some(hp) => hp,
            None     => self.hp(self.vmax, d, lwl, leff, cs, ws),
        }
    }

    // hp_cruise {{{3
    /// Horsepower required to achieve cruising speed.
    ///
    // XXX: Should vcruise be set to a minimum somewhere else?
    pub fn hp_cruise(&self, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        self.hp(self.vcruise.min(self.vmax), d, lwl, leff, cs, ws)
    }

    // rf {{{3
    /// Friction resistance at a given speed.
    ///
    fn rf(v: f64, ws: f64) -> f64 {
        0.01 * ws * v.powf(1.83)
    }

    // rf_max {{{3
    /// Friction resistance at maximum speed.
    ///
    pub fn rf_max(&self, ws: f64) -> f64 {
        Self::rf(self.vmax, ws)
    }

    // rf_cruise {{{3
    /// Friction resistance at cruising speed.
    ///
    pub fn rf_cruise(&self, ws: f64) -> f64 {
        Self::rf(self.vcruise, ws)
    }

    // rw {{{3
    /// Wave resistance at a given speed.
    ///
    fn rw(v: f64, d: f64, lwl: f64, cs: f64) -> f64 {
        if lwl == 0.0 { return 0.0; }
        d.powf(2.0 / 3.0) / lwl * cs * v.powf(4.0)
    }

    // rw_max {{{3
    /// Wave resistance at maximum speed.
    ///
    pub fn rw_max(&self, d: f64, lwl: f64, cs: f64) -> f64 {
        Self::rw(self.vmax, d, lwl, cs)
    }

    // rw_cruise {{{3
    /// Wave resistance at cruising speed.
    ///
    pub fn rw_cruise(&self, d: f64, lwl: f64, cs: f64) -> f64 {
        Self::rw(self.vcruise, d, lwl, cs)
    }

    // pw {{{3
    /// Power to wave ratio.
    ///
    fn pw(rw: f64, rf: f64) -> f64 {
        match rw + rf {
            0.0 => 0.0, // Protect against divide by 0
            _   => rw / (rw + rf),
        }
    }

    // pw_max {{{3
    /// Power to wave ratio at maximum speed.
    ///
    pub fn pw_max(&self, d: f64, lwl: f64, cs: f64, ws: f64) -> f64 {
        Self::pw(self.rw_max(d, lwl, cs), self.rf_max(ws))
    }

    // pw_cruise {{{3
    /// Power to wave ratio at cruising speed.
    ///
    pub fn pw_cruise(&self, d: f64, lwl: f64, cs: f64, ws: f64) -> f64 {
        Self::pw(self.rw_cruise(d, lwl, cs), self.rf_cruise(ws))
    }

    // bunker {{{3
    /// Bunkerage weight.
    ///
    pub fn bunker(&self, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        self.bunker_for_range(self.range as f64, d, lwl, leff, cs, ws)
    }

    // bunker_for_range {{{3
    /// Bunkerage weight for a given range in nautical miles.
    ///
    /// Split out of `bunker` so the power-lock Recalc can solve the range
    /// whose bunkerage matches a stored target.
    ///
    pub(crate) fn bunker_for_range(&self, range: f64, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        if self.vcruise == 0.0 { return 0.0; } // catch divide by zero
        if self.boiler.bunker_factor(self.year) == 0.0 { return 0.0 };
        if self.hp_cruise(d, lwl, leff, cs, ws) == 0.0 { return 0.0 };

        let bunker = range / (1.0 + 0.4 * (1.0 - self.pct_coal));
        let bunker = bunker / self.boiler.bunker_factor(self.year);

        bunker /
            (1.8 / self.hp_cruise(d, lwl, leff, cs, ws) * Self::RANGE * self.vcruise * 0.1) +
            d * 0.005
    }

    // bunker_max {{{3
    /// Bunkerage weight at maximum displacement.
    ///
    pub fn bunker_max(&self, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        self.bunker(d, lwl, leff, cs, ws) * 1.8
    }

    // num_engines {{{3
    /// Number of steam engines.
    ///
    pub fn num_engines(&self) -> u32 {
        self.boiler.num_engines()
    }

    // d_engine {{{3
    /// Displacement of the engine.
    ///
    pub fn d_engine(&self, d: f64, lwl: f64, leff: f64, cs: f64, ws: f64) -> f64 {
        let factor = self.boiler.d_engine_factor(self.year, self.fuel.clone());
        let early =
            if self.year <= 1889 {
                1.0 + (1890 - self.year) as f64 / 100.0
            } else {
                1.0
            };

        if early == 0.0 { return 0.0; }

        (
            self.hp_max(d, lwl, leff, cs, ws) /
            (factor / self.num_engines() as f64 * (1.1 - self.pct_coal / 10.0))
        ) / early
    }
}

// FuelType {{{1
//
bitflags! {
    /// Types of fuel used by the engine.
    ///
    #[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
    pub struct FuelType: u8 {
        const Coal     = 1 << 0;
        const Oil      = 1 << 1;
        const Diesel   = 1 << 2;
        const Gasoline = 1 << 3;
        const Battery  = 1 << 4;
    }
}

impl fmt::Display for FuelType { // {{{2
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
            bitflags_match!(*self, {
                Self::Coal        => "Coal fired boilers",
                Self::Oil         => "Oil fired boilers",
                Self::Coal |
                    Self::Oil     => "Coal and oil fired boilers",

                Self::Coal |
                    Self::Diesel  => "Coal fired boilers plus diesel motors",
                Self::Oil |
                    Self::Diesel  => "Oil fired boilers plus diesel motors",
                Self::Coal |
                    Self::Oil |
                    Self::Diesel  => "Coal and oil fired boilers plus diesel motors",

                Self::Diesel      => "Diesel internal combustion motors",
                Self::Diesel |
                    Self::Battery => "Diesel internal combustion engines plus batteries",

                Self::Gasoline    => "Gasoline internal combustion motors",
                Self::Gasoline |
                    Self::Battery => "Gasoline internal combustion motors plus batteries",

                Self::Battery     => "Battery powered",

                _                 => "ERROR: Revise fuels",
            })
        )
    }
}

impl FuelType { // {{{2
    // is_steam {{{3
    /// Return true if the fuel indicates a steam engine.
    ///
    pub fn is_steam(&self) -> bool {
        self.contains(Self::Coal) || self.contains(Self::Oil)
    }
}

// BoilerType {{{1
//
bitflags! {
    /// Types of boilers used by the engine.
    ///
    #[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
    pub struct BoilerType: u8 {
        /// Simple, reciprocating engines.
        const Simple  = 1 << 0;
        /// Complex, reciprocating engines.
        const Complex = 1 << 1;
        /// Steam turbine engines.
        const Turbine = 1 << 2;
    }
}

impl fmt::Display for BoilerType { // {{{2
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
            bitflags_match!(*self, {
                Self::Simple => "simple reciprocating steam engines",
                Self::Complex => "complex reciprocating steam engines",
                Self::Turbine => "steam turbines",

                Self::Simple |
                    Self::Complex => "reciprocating steam engines",

                Self::Simple |
                    Self::Turbine => "reciprocating cruising steam engines and steam turbines",

                Self::Simple |
                    Self::Complex |
                    Self::Turbine => "ERROR: Too many types of steam engines",

                _ => "ERROR: No steam engines",
            })
        )
    }
}

// BoilerType Implementation {{{2
impl BoilerType {
    // hp_type {{{3
    /// Return the string for the type of
    /// horsepower used with the boiler type.
    ///
    pub fn hp_type(&self) -> String {
        match self.is_reciprocating() {
            true => "ihp".into(),
            false => "shp".into(),
        }
    }

    // num_engines {{{3
    /// Number of steam engines with each
    /// boiler type representing one engine.
    ///
    pub fn num_engines(&self) -> u32 {
        u8::count_ones(self.bits())
    }

    // is_simple {{{3
    /// Return true if the boiler has simple reciprocating engines.
    ///
    pub fn is_simple(&self) -> bool {
        self.contains(Self::Simple)
    }

    // is_complex {{{3
    /// Return true if the boiler has complex reciprocating engines.
    ///
    pub fn is_complex(&self) -> bool {
        self.contains(Self::Complex)
    }

    // is_reciprocating {{{3
    /// Return true if the boiler has any type of reciprocating engines.
    ///
    pub fn is_reciprocating(&self) -> bool {
        self.is_simple() || self.is_complex()
    }

    // is_turbine {{{3
    /// Return true if the boiler has steam turbines.
    ///
    pub fn is_turbine(&self) -> bool {
        self.contains(Self::Turbine)
    }

    // d_engine_factor {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn d_engine_factor(&self, year: u32, fuel: FuelType) -> f64 {
        if year < 1860 { return 0.0 };

        let a = if self.is_simple() {
                         if year <= 1884 { 1.2  + (year - 1860) as f64 * 0.05 }
                    else if year <= 1949 { 2.45 + (year - 1885) as f64 * 0.025 }
                    else                 { 4.075 }
                } else { 0.0 };

        let b = if self.is_complex() {
                         if year <= 1905 { 1.2 + (year - 1860) as f64 * 0.05 }
                    else if year <= 1910 { 3.5 + (year - 1906) as f64 }
                    else if year <= 1949 { 7.5 + (year - 1910) as f64 * 0.025 }
                    else                 { 8.5 }
                } else { 0.0 };

        let c = if self.is_turbine() || ! fuel.is_steam() {
                         if year <= 1897 { 1.2 + (year - 1860) as f64 * 0.05 }
                    else if year <= 1902 { 1.0 + (year - 1898) as f64 * 0.5 }
                    else if year <= 1909 { 4.0 + (year - 1903) as f64 }
                    else if year <= 1949 { 11.0 + (year - 1910) as f64 * 0.2 }
                    else                 { 19.0 }
                } else { 0.0 };

        a + b + c
    }

    // bunker_factor {{{3
    /// XXX: I do not know what this does.
    ///
    pub fn bunker_factor(&self, year: u32) -> f64 {
        let year = year as f64;
        if self.is_reciprocating() || year < 1898.0 {
            1.0 - (1910.0 - year) / 70.0
        } else if year < 1920.0 {
            1.0 + (year - 1910.0) / 20.0
        } else if year < YEAR_MAX as f64 {
            1.5 + (year - 1920.0) / 60.0
        } else {
            2.0
        }
    }
}

// DriveType {{{1
//
bitflags! {
    /// Type of drive used by the engine.
    ///
    #[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
    pub struct DriveType: u8 {
        const Direct    = 1 << 0;
        const Geared    = 1 << 1;
        const Electric  = 1 << 2;
        const Hydraulic = 1 << 3;
    }
}

impl fmt::Display for DriveType { // {{{2
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",
            bitflags_match!(*self, {
                Self::Direct    => "Direct drive",
                Self::Geared    => "Geared drive",
                Self::Electric  => "Electric motors",
                Self::Hydraulic => "Hydraulic drive",

                Self::Geared |
                    Self::Electric => "Electric cruising motors plus geared drives",

                Self::empty()   => "ERROR: No drive to shaft",
                _               => "ERROR: Revise drives",
            })
        )
    }
}

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;

    // Testing Engine {{{2
    mod engine {
        use super::*;

        // Test hp {{{4
        macro_rules! test_hp {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let d = 5000.0; let lwl = 500.0; let leff = 500.0; let cs = 0.2576602375; let ws = 30050.0;
                        let (expected, v, year) = $value;
                        let mut eng = Engine::default();
                        eng.year = year;

                        assert_eq!(expected, to_place(eng.hp(v, d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_hp! {
            // name:                     (hp, v, year)
            hp_v_le_15:                  (4096.44, 15.0, 1900),
            hp_v_ge_25:                  (22740.39, 25.0, 1900),
            hp_v_other_low:              (5029.43, 16.0, 1900),
            hp_year_early:               (10566.98, 20.0, 1889),
            hp_year_late:                (10462.36, 20.0, 1891),
            hp_v_other_hi_year_boundary: (19655.91, 24.0, 1890),
        }

        // Test hp_max {{{3
        macro_rules! test_hp_max {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let d = 5000.0; let lwl = 500.0; let leff = 500.0; let cs = 0.2576602375; let ws = 30050.0;

                        let (expected, vmax) = $value;
                        let mut eng = Engine::default();
                        eng.vmax = vmax; eng.year = 1920;

                        assert_eq!(expected, to_place(eng.hp_max(d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_hp_max! {
            // name:     (hp, vmax)
            hp_max_test: (10462.36, 20.0),
        }

        // Test hp_cruise {{{3
        macro_rules! test_hp_cruise {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let d = 5000.0; let lwl = 500.0; let leff = 500.0; let cs = 0.2576602375; let ws = 30050.0;

                        let (expected, vmax, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.vmax = vmax; eng.vcruise = vcruise; eng.year = 1920;

                        assert_eq!(expected, to_place(eng.hp_cruise(d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_hp_cruise! {
            // name:                  (hp, vmax, vcruise)
            hp_cruise_cruise_is_less: (1184.96, 20.0, 10.0),
            hp_cruise_max_is_less:    (1184.96, 10.0, 20.0),
        }

        // Test rf {{{3
        macro_rules! test_rf {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, v, ws) = $value;

                        assert_eq!(expected, to_place(Engine::rf(v, ws), 2));
                    }
                )*
            }
        }
        test_rf! {
            // name:    (rf, v, ws)
            rf_test:    (2028.25, 10.0, 3000.0),
        }

        // Test rf_max {{{3
        macro_rules! test_rf_max {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vmax) = $value;
                        let mut eng = Engine::default();
                        eng.vmax = vmax;
                        let ws = 3000.0;

                        assert_eq!(expected, to_place(eng.rf_max(ws), 2));
                    }
                )*
            }
        }
        test_rf_max! {
            // name:     (rf, vmax)
            rf_max_test: (7211.18, 20.0),
        }

        // Test rf_cruise {{{3
        macro_rules! test_rf_cruise {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.vcruise = vcruise;
                        let ws = 3000.0;

                        assert_eq!(expected, to_place(eng.rf_cruise(ws), 2));
                    }
                )*
            }
        }
        test_rf_cruise! {
            // name:        (rf, vcruise)
            rf_cruise_test: (2028.25, 10.0),
        }

        // Test rw {{{3
        macro_rules! test_rw {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, lwl) = $value;
                        let v = 10.0; let d = 1000.0; let cs = 0.1234;

                        assert_eq!(expected, to_place(Engine::rw(v, d, lwl, cs), 2));
                    }
                )*
            }
        }
        test_rw! {
            // name:     (rw, lwl)
            rw_lwl_zero: (0.0, 0.0),
            rw_test:     (1234.0, 100.0),
        }

        // Test rw_max {{{3
        macro_rules! test_rw_max {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vmax) = $value;
                        let mut eng = Engine::default();
                        eng.vmax = vmax;
                        let d = 5000.0; let lwl = 500.0; let cs = 0.4;

                        assert_eq!(expected, to_place(eng.rw_max(d, lwl, cs), 2));
                    }
                )*
            }
        }
        test_rw_max! {
            // name:     (rw, vmax)
            rw_max_test: (37427.43, 20.0),
        }

        // Test rw_cruise {{{3
        macro_rules! test_rw_cruise {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.vcruise = vcruise;
                        let d = 5000.0; let lwl = 500.0; let cs = 0.4;

                        assert_eq!(expected, to_place(eng.rw_cruise(d, lwl, cs), 2));
                    }
                )*
            }
        }
        test_rw_cruise! {
            // name:        (rw, vcruise)
            rw_cruise_test: (2339.21, 10.0),
        }

        // Test pw {{{3
        macro_rules! test_pw {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, rw, rf) = $value;

                        assert_eq!(expected, to_place(Engine::pw(rw, rf), 2));
                    }
                )*
            }
        }
        test_pw! {
            // name:    (pw, rw, rf)
            pw_both_eq_0: (0.0, 0.0, 0.0),
            pw_test:    (0.29, 800.0, 2000.0),
        }

        // Test pw_max {{{3
        macro_rules! test_pw_max {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vmax) = $value;
                        let mut eng = Engine::default();
                        eng.vmax = vmax;
                        let d = 5000.0; let lwl = 500.0; let cs = 0.4; let ws = 3000.0;

                        assert_eq!(expected, to_place(eng.pw_max(d, lwl, cs, ws), 2));
                    }
                )*
            }
        }
        test_pw_max! {
            // name:     (pw, vmax)
            pw_max_test: (0.84, 20.0),
        }

        // Test pw_cruise {{{3
        macro_rules! test_pw_cruise {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.vcruise = vcruise;
                        let d = 5000.0; let lwl = 500.0; let cs = 0.4; let ws = 3000.0;

                        assert_eq!(expected, to_place(eng.pw_cruise(d, lwl, cs, ws), 2));
                    }
                )*
            }
        }
        test_pw_cruise! {
            // name:        (pw, vcruise)
            pw_cruise_test: (0.54, 10.0),
        }

        // Test bunker {{{3
        macro_rules! test_bunker {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, range, pct_coal, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.range = range;
                        eng.pct_coal = pct_coal;
                        eng.vcruise = vcruise;
                        eng.vmax = vcruise; // vmax must be >= vcruise or hp_cruise will fail

                        eng.boiler = BoilerType::Turbine;
                        eng.year = 1920;
                        let lwl = 500.0; let leff = 500.0;
                        let cs = 0.2563; let ws = 12000.0; let d = 1000.0;

                        assert_eq!(expected, to_place(eng.bunker(d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_bunker! {
            // name:           (bunker, range, pct_coal, vcruise)
            bunker_pct_coal_0: (29.78, 1000, 1.0, 10.0),
            bunker_pct_coal_1: (22.70, 1000, 0.0, 10.0),
            bunker_range_0:    (5.0, 0, 0.5, 10.0),
            bunker_vcruise_0:  (0.0, 1000, 0.5, 0.0),
        }

        // Test bunker_max {{{3
        macro_rules! test_bunker_max {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, range, pct_coal, vcruise) = $value;
                        let mut eng = Engine::default();
                        eng.range = range;
                        eng.pct_coal = pct_coal;
                        eng.vcruise = vcruise;
                        eng.vmax = vcruise; // vmax must be >= vcruise or hp_cruise will fail

                        eng.boiler = BoilerType::Turbine;
                        eng.year = 1920;
                        let lwl = 500.0; let leff = 500.0;
                        let cs = 0.2563; let ws = 12000.0; let d = 1000.0;

                        assert_eq!(expected, to_place(eng.bunker_max(d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_bunker_max! {
            // name:     (bunker, range, pct_coal, vcruise)
            bunker_max: (40.86, 1000, 0.0, 10.0),
        }

        // Test d_engine {{{3
        macro_rules! test_d_engine {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, year) = $value;
                        let mut eng = Engine::default();
                        eng.year = year;

                        eng.pct_coal = 0.5;
                        eng.vmax = 10.0;
                        eng.boiler = BoilerType::Turbine;
                        eng.fuel = FuelType::Oil;
                        let lwl = 500.0; let leff = 500.0;
                        let cs = 0.2563; let ws = 12000.0; let d = 1000.0;

                        assert_eq!(expected, to_place(eng.d_engine(d, lwl, leff, cs, ws), 2));
                    }
                )*
            }
        }
        test_d_engine! {
            // name:     (d_engine, year)
            d_engine_early: (168.32, 1889),
            d_engine_late: (165.21, 1890),
        }
    } // mod engine

    // Testing FuelType {{{2
    mod fuel_type {
        use super::*;

        // Test is_steam {{{4
        macro_rules! test_is_steam {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, fuel) = $value;

                        assert_eq!(expected, fuel.is_steam());
                    }
                )*
            }
        }

        test_is_steam! {
            // name:           (is_steam, fuel)
            is_steam_coal:     (true, FuelType::Coal),
            is_steam_oil:      (true, FuelType::Oil),
            is_steam_diesel:   (false, FuelType::Diesel),
            is_steam_gas:      (false, FuelType::Gasoline),
            is_steam_battery:  (false, FuelType::Battery),
            is_steam_multiple: (true, FuelType::Coal | FuelType::Diesel),
        }
    } // mod fuel_type

    // Testing BoilerType {{{2
    mod boiler_type {
        use super::*;

        // Test d_engine_factor {{{4
        macro_rules! test_d_engine_factor {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, boiler, fuel, year) = $value;

                        assert_eq!(expected, to_place(boiler.d_engine_factor(year, fuel), 3));
                    }
                )*
            }
        }

        test_d_engine_factor! {
            // name:                  (d_engine_factor, boiler, fuel, year)
            // Test the years
            d_engine_factor_simple_1: (2.4, BoilerType::Simple, FuelType::Oil, 1884),
            d_engine_factor_simple_2: (4.05, BoilerType::Simple, FuelType::Oil, 1949),
            d_engine_factor_simple_3: (4.075, BoilerType::Simple, FuelType::Oil, 1950),

            d_engine_factor_complex_1: (3.45, BoilerType::Complex, FuelType::Oil, 1905),
            d_engine_factor_complex_2: (7.5, BoilerType::Complex, FuelType::Oil, 1910),
            d_engine_factor_complex_3: (8.475, BoilerType::Complex, FuelType::Oil, 1949),
            d_engine_factor_complex_4: (8.5, BoilerType::Complex, FuelType::Oil, 1950),

            d_engine_factor_other_1: (3.05, BoilerType::Turbine, FuelType::Oil, 1897),
            d_engine_factor_other_2: (3.0, BoilerType::Turbine, FuelType::Oil, 1902),
            d_engine_factor_other_3: (10.0, BoilerType::Turbine, FuelType::Oil, 1909),
            d_engine_factor_other_4: (18.8, BoilerType::Turbine, FuelType::Oil, 1949),
            d_engine_factor_other_5: (19.0, BoilerType::Turbine, FuelType::Oil, 1950),

            // Test ! fuel.is_steam()
            d_engine_factor_not_steam: (4.825, BoilerType::Simple, FuelType::Gasoline, 1900),

            // Test sum of the three checks
            d_engine_factor_recip:           (6.025, BoilerType::Simple | BoilerType::Complex, FuelType::Oil, 1900),
            d_engine_factor_simple_turbine:  (4.825, BoilerType::Simple | BoilerType::Turbine, FuelType::Oil, 1900),
            d_engine_factor_complex_turbine: (5.2, BoilerType::Complex | BoilerType::Turbine, FuelType::Oil, 1900),
        }
    } // mod boiler_type
}
