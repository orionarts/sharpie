use crate::calc::{Hull, Measurement, UnitType::{LengthSmall, LengthLong}, Units};
use crate::choice_enum;

use serde::{Deserialize, Serialize};

use std::fmt;

// Armor {{{1
/// The ship's armor, excluding gun armor.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Armor {
    /// Units
    pub units: Units,

    /// Main belt armor.
    pub main: Belt,
    /// End belt armor.
    pub end: Belt,
    /// Upper belt armor.
    pub upper: Belt,
    /// Incline of belt armor.
    pub incline: f64,

    /// Torpedo bulge armor.
    ///
    pub bulge: Belt,
    /// Bulkhead armor.
    pub bulkhead: Belt,
    /// What it says on the tin.
    pub bh_kind: BulkheadType,
    /// Beam between outer and inner bulkheads.
    pub bh_beam: Measurement,

    /// Deck armor.
    pub deck: Deck,

    /// Forward conning tower armor.
    pub ct_fwd: CT,
    /// Aft conning tower armor.
    pub ct_aft: CT,
}

impl Default for Armor { // {{{2
    fn default() -> Self {
        Armor {
            units: Units::Imperial,

            main:     Belt::new(BeltType::Main),
            end:      Belt::new(BeltType::End),
            upper:    Belt::new(BeltType::Upper),
            bulge:    Belt::new(BeltType::Bulge),
            bulkhead: Belt::new(BeltType::Bulkhead),

            bh_kind: BulkheadType::Additional,
            incline: 0.0,
            bh_beam: Measurement::new(0.0, LengthLong, Units::Imperial),

            deck: Deck::default(),

            ct_fwd: CT::default(),
            ct_aft: CT::default(),
        }
    }
}

impl Armor { // {{{2
    // Factor to calculate the weight of X ft²*in of armor
    // 1 ft²*in armor => INCH tons
    pub const INCH: f64 = 0.0185;

    // wgt {{{3
    /// Total weight of armor.
    ///
    pub fn wgt(&self, hull: Hull, wgt_mag: f64, wgt_engine: f64) -> f64 {
        let lwl = hull.lwl().imp();
        let cwp = hull.cwp();
        let b   = hull.b.imp();
        let d   = hull.d();

        self.main    .wgt(lwl, cwp, b) +
        self.end     .wgt(lwl, cwp, b) +
        self.upper   .wgt(lwl, cwp, b) +
        self.bulge   .wgt(lwl, cwp, b) +
        self.bulkhead.wgt(lwl, cwp, b) +

        self.deck    .wgt(hull.clone(), wgt_mag, wgt_engine) +

        self.ct_fwd  .wgt(d) +
        self.ct_aft  .wgt(d)
    }

    // belt_coverage {{{3
    /// Percentage of the "vital areas" covered by the main belt.
    ///
    pub fn belt_coverage(&self, lwl: f64) -> f64 {
        if lwl == 0.0 { return 0.0 }
        self.main.len.imp() / (lwl * 0.65)
    }

    // max_hgt {{{3
    /// Maximum allowable belt height.
    ///
    #[allow(dead_code)]
    pub fn max_belt_hgt(&self, t: f64, dist: f64) -> f64 {
        use std::f64::consts::PI;

        let radians = self.incline * PI / 180.0;
        if radians.abs().cos() == 0.0 { return 0.0 }

        (t + dist) * (1.0 / radians.abs().cos()) + 0.02
    }
}

// Belt {{{1
/// Belt, bulkhead and torpedo bulge armor.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Belt {
    /// Belt thickness.
    pub thick: Measurement,
    /// Belt length.
    pub len: Measurement,
    /// Belt height.
    pub hgt: Measurement,

    /// Type of belt.
    ///
    /// Using this private "set once" field allows Belt to represent the
    /// multiple types that differ only in how their weight is calculated.
    kind: BeltType, // kind should not be changed after creation
}

impl Belt { // {{{2
    // wgt {{{3
    /// Belt weight.
    ///
    pub fn wgt(&self, lwl: f64, cwp: f64, b: f64) -> f64 {
        let len   = self.len.imp();
        let hgt   = self.hgt.imp();
        let thick = self.thick.imp();

        if lwl == 0.0 { return 0.0; }

        // Calculate the area of one bulkhead across the beam
        let beam_bulkhead = match self.kind {
            BeltType::Main | BeltType::Upper =>
                (1.0 - len / lwl).powf(1.0 - cwp) * b,
            _ => 0.0
        };

        // Calculate the weight of one belt and one bulkhead across the beam
        let wgt = (len + beam_bulkhead) * hgt * thick * Armor::INCH;

        // Double the weight to account for two belts and two beam bulkheads
        wgt * 2.0
    }

    // new {{{3
    /// Create a Belt of type "kind".
    ///
    pub fn new(kind: BeltType) -> Belt {
        Belt {
            thick: Measurement::new(0.0, LengthSmall, Units::Imperial),
            len:   Measurement::new(0.0, LengthLong, Units::Imperial),
            hgt:   Measurement::new(0.0, LengthLong, Units::Imperial),
            kind,
        }
    }
}

// BulkheadType {{{1
/// Values for Armor::bh_kind
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum BulkheadType {
    /// Simpler and thinner.
    Strengthened,
    /// Modern, multilayered and thicker.
    #[default]
    Additional,
}

choice_enum!(BulkheadType {
    Additional   => ("Additional bulkheads",   "Additional damage containing bulkheads"),
    Strengthened => ("Strengthened bulkheads", "Strengthened structural bulkheads"),
});

// BeltType {{{1
/// Values for Belt::kind
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BeltType {
    /// Main belt.
    Main,
    /// End belt.
    End,
    /// Upper belt.
    Upper,
    /// Torpedo bulges.
    Bulge,
    /// Bulkhead.
    Bulkhead,
}

// CT {{{1
/// Conning tower armor.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CT {
    /// Armor thickness.
    pub thick: Measurement,
}

impl Default for CT {
    fn default() -> Self {
        CT {
            thick: Measurement::new(0.0, LengthSmall, Units::Imperial),
        }
    }
}

impl CT { // {{{2
    // wgt {{{3
    /// Weight of armor.
    ///
    pub fn wgt(&self, d: f64) -> f64 {
        10.0 * (d / 10_000.0).powf(2.0 / 3.0) * self.thick.imp()
    }
}

// Deck {{{1
/// Deck armor.
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Deck {
    /// Forecastle deck thickness.
    pub fc: Measurement,
    /// Main deck thickness.
    pub md: Measurement,
    /// Quarterdeck deck thickness.
    pub qd: Measurement,

    /// Deck armor configuration.
    pub kind: DeckType,
}

impl Default for Deck {
    fn default() -> Self {
        Deck {
            fc:   Measurement::new(0.0, LengthSmall, Units::Imperial),
            md:   Measurement::new(0.0, LengthSmall, Units::Imperial),
            qd:   Measurement::new(0.0, LengthSmall, Units::Imperial),
            kind: DeckType::MultipleArmored,
        }
    }
}

impl Deck { // {{{2
    // wgt {{{3
    /// Weight of deck armor.
    ///
    pub fn wgt(&self, hull: Hull, wgt_mag: f64, wgt_engine: f64) -> f64 {
        let d      = hull.d();
        let lwl    = hull.lwl().imp();
        let b      = hull.b.imp();
        let fc_len = hull.freeboard.fc_len;
        let qd_len = hull.freeboard.qd_len;
        let cwp    = hull.cwp();
        let wp     = hull.wp().imp();

        let main_deck = self.kind.wgt_factor(
            d, lwl, b, fc_len, qd_len, wp, cwp, wgt_engine, wgt_mag
        );

        let fc_deck = (fc_len * 2.0).powf(1.0 - cwp.powf(2.0)) *
            b * lwl * fc_len * 0.5;

        let qd_deck = qd_len.powf(1.0 - cwp) * b * lwl * qd_len / 4.0 *
            (2.0 + 2.0_f64.powf(1.0 - cwp));

        (main_deck * self.md.imp() + fc_deck * self.fc.imp() + qd_deck * self.qd.imp()) * Armor::INCH
    }
}

// DeckType {{{1
/// Deck armor configuration types.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum DeckType {
    #[default]
    MultipleArmored,
    SingleArmored,
    MultipleProtected,
    SingleProtected,
    BoxOverMachinery,
    BoxOverMagazine,
    BoxOverBoth,
}

impl DeckType { // {{{2
    // wgt_factor {{{3
    /// Main deck weight factor for each deck type.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn wgt_factor(&self,
        d: f64, lwl: f64, b: f64,
        fc_len: f64, qd_len:f64,
        wp: f64, cwp: f64,
        wgt_engine: f64, wgt_mag: f64) -> f64 {

        if d == 0.0 { return 0.0; }

        match self {
            Self::MultipleArmored |
            Self::SingleArmored |
            Self::MultipleProtected |
            Self::SingleProtected => {
                (
                    wp - (fc_len * 2.0).powf(1.0 - cwp.powf(2.0)) * b * lwl * fc_len / 2.0 -
                    (
                        qd_len.powf(1.0 - cwp) * b * lwl * qd_len * 0.25 +
                        (
                            qd_len.powf(1.0 - cwp) +
                            (qd_len * 2.0).powf(1.0 - cwp)
                        ) * b * lwl * qd_len * 0.25
                    )
                ) * 1.01
            },

            Self::BoxOverMachinery =>
                (wgt_engine * 3.0 / (d * 0.94) * 0.65 * lwl + 16.0) * (b + 16.0) - 256.0,

            Self::BoxOverMagazine =>
                (wgt_mag / (d * 0.94) * 0.65 * lwl + 16.0) * (b + 16.0) - 256.0,

            Self::BoxOverBoth =>
                ((wgt_engine * 3.0 + wgt_mag) / (d * 0.94) * 0.65 * lwl + 16.0) * (b + 16.0) - 256.0,

        }
    }
}

choice_enum!(DeckType {
    MultipleArmored   => ("Armoured deck - multiple decks",  "Armoured deck - multiple decks"),
    SingleArmored     => ("Armoured deck - single deck",     "Armoured deck - single deck"),
    MultipleProtected => ("Protected deck - multiple decks", "Protected deck - multiple decks"),
    SingleProtected   => ("Protected deck - single deck",    "Protected deck - single deck"),
    BoxOverMachinery  => ("Box over machinery",              "Box over machinery"),
    BoxOverMagazine   => ("Box over magazines",              "Box over magazines"),
    BoxOverBoth       => ("Box over machinery & magazines",  "Box over machinery & magazines"),
});

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;

    // Testing Armor {{{2
    mod armor {
        use super::*;

        // Test belt_coverage {{{3
        macro_rules! test_belt_coverage {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, belt_len, lwl) = $value;

                        let mut armor = Armor::default();
                        armor.main.len = Measurement::new(belt_len, LengthLong, Units::Imperial);

                        assert_eq!(expected, to_place(armor.belt_coverage(lwl), 2));
                    }
                )*
            }
        }
        test_belt_coverage! {
            // name:       (belt_coverage, belt_len, lwl)
            belt_coverage: (1.0, 0.65, 1.0),
        }

        // Test max_hgt {{{3
        macro_rules! test_max_hgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, incline) = $value;

                        let t = 10.0;

                        let mut ship = test_ship();
                        ship.armor.incline = incline;

                        assert_eq!(
                            to_place(ship.armor.max_belt_hgt(t, ship.hull.freeboard.distributed()), 2),
                            expected
                        );
                    }
                )*
            }
        }
        test_max_hgt! {
            // name:            (max_hgt, incline)
            max_belt_hgt_0:     (20.02, 0.0),
            max_belt_hgt_45:     (28.3, 45.0),
            max_belt_hgt_neg_45: (28.3, -45.0),
        }

        // Test wgt {{{3

        // All-zero armor weighs nothing.
        #[test]
        fn wgt_zero() {
            let ship = test_ship();

            assert_eq!(to_place(ship.armor.wgt(ship.hull, 100.0, 100.0), 2), 0.0);
        }

        // The aggregate wgt() is the sum of every belt, the deck and both
        // conning towers.
        #[test]
        fn wgt_all() {
            let ship = test_ship();
            let mut armor = ship.armor;

            armor.main.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.main.len   = Measurement::new(20.0, LengthLong, Units::Imperial);
            armor.main.hgt   = Measurement::new(5.0, LengthLong, Units::Imperial);

            armor.end.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.end.len   = Measurement::new(20.0, LengthLong, Units::Imperial);
            armor.end.hgt   = Measurement::new(5.0, LengthLong, Units::Imperial);

            armor.upper.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.upper.len   = Measurement::new(20.0, LengthLong, Units::Imperial);
            armor.upper.hgt   = Measurement::new(5.0, LengthLong, Units::Imperial);

            armor.bulge.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.bulge.len   = Measurement::new(20.0, LengthLong, Units::Imperial);
            armor.bulge.hgt   = Measurement::new(5.0, LengthLong, Units::Imperial);

            armor.bulkhead.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.bulkhead.len   = Measurement::new(20.0, LengthLong, Units::Imperial);
            armor.bulkhead.hgt   = Measurement::new(5.0, LengthLong, Units::Imperial);

            armor.deck.fc = Measurement::new(0.5, LengthSmall, Units::Imperial);
            armor.deck.md = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.deck.qd = Measurement::new(0.5, LengthSmall, Units::Imperial);

            armor.ct_fwd.thick = Measurement::new(1.0, LengthSmall, Units::Imperial);
            armor.ct_aft.thick = Measurement::new(2.0, LengthSmall, Units::Imperial);

            assert_eq!(to_place(armor.wgt(ship.hull, 100.0, 100.0), 2), 110.32);
        }
    }

    // Testing Belt {{{2
    mod belt {
        use super::*;

        // Test wgt {{{3
        macro_rules! test_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let lwl = 500.0;
                        let cwp = 0.5;
                        let b = 10.0;

                        let (expected, thick, len, hgt, kind) = $value;
                        let mut belt = Belt::new(kind);
                        belt.thick = Measurement::new(thick, LengthSmall, Units::Imperial);
                        belt.len   = Measurement::new(len, LengthLong, Units::Imperial);
                        belt.hgt   = Measurement::new(hgt, LengthLong, Units::Imperial);

                        assert_eq!(expected, to_place(belt.wgt(lwl, cwp, b), 2));
                    }
                )*
            }
        }
        test_wgt! {
            // name:      (wgt, thick, len, hgt, kind)
            wgt_zero:     (0.0, 0.0, 0.0, 0.0, BeltType::Main),
            wgt_main:     (40.31, 1.0, 100.0, 10.0, BeltType::Main),
            wgt_end:      (37.0, 1.0, 100.0, 10.0, BeltType::End),
            wgt_upper:    (40.31, 1.0, 100.0, 10.0, BeltType::Upper),
            wgt_bulge:    (37.0, 1.0, 100.0, 10.0, BeltType::Bulge),
            wgt_bulkhead: (37.0, 1.0, 100.0, 10.0, BeltType::Bulkhead),
        }
    }

    // Testing CT {{{2
    mod ct {
        use super::*;

        // Test wgt {{{3
        macro_rules! test_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let d = 1000.0;
                        let (expected, thick) = $value;
                        let mut ct = CT::default();
                        ct.thick = Measurement::new(thick, LengthSmall, Units::Imperial);

                        assert_eq!(expected, to_place(ct.wgt(d), 2));
                    }
                )*
            }
        }
        test_wgt! {
            //  name: (wgt, thick)
            wgt_zero: (0.0, 0.0),
            wgt_test: (2.15, 1.0),
        }
    }

    // Testing Deck {{{2
    mod deck {
        use super::*;

        // Test wgt {{{3
        macro_rules! test_wgt {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let (expected, kind, fc, md, qd) = $value;

                        let mut ship = test_ship();

                        ship.armor.deck.kind = kind;
                        ship.armor.deck.fc = Measurement::new(fc, LengthSmall, Units::Imperial);
                        ship.armor.deck.md = Measurement::new(md, LengthSmall, Units::Imperial);
                        ship.armor.deck.qd = Measurement::new(qd, LengthSmall, Units::Imperial);

                        let wgt_mag    = 100.0;
                        let wgt_engine = ship.wgt_engine();

                        assert_eq!(
                            to_place(ship.armor.deck.wgt(ship.hull, wgt_mag, wgt_engine), 2),
                            expected
                        );
                    }
                )*
            }
        }

        test_wgt! {
            //  name:             (wgt, deck, fc, md, qd)
            wgt_mult_arm_fc:      (6.67,   DeckType::MultipleArmored,    1.0, 0.0, 0.0),
            wgt_mult_arm_md:      (60.58,  DeckType::MultipleArmored,    0.0, 1.0, 0.0),
            wgt_mult_arm_qd:      (7.49,   DeckType::MultipleArmored,    0.0, 0.0, 1.0),
            wgt_mult_arm:         (74.74,  DeckType::MultipleArmored,    1.0, 1.0, 1.0),

            wgt_one_arm_fc:       (6.67,   DeckType::SingleArmored,      1.0, 0.0, 0.0),
            wgt_mult_prot_fc:     (6.67,   DeckType::MultipleProtected,  1.0, 0.0, 0.0),
            wgt_one_prot_fc:      (6.67,   DeckType::SingleProtected,    1.0, 0.0, 0.0),
            wgt_box_machinery_md: (194.5,  DeckType::BoxOverMachinery,   0.0, 1.0, 0.0),
            wgt_box_magazine_md:  (23.24,  DeckType::BoxOverMagazine,    0.0, 1.0, 0.0),
            // XXX: springsheet gives 202.77
            wgt_box_both_md:      (202.95, DeckType::BoxOverBoth,        0.0, 1.0, 0.0),
        }
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
            display_multiple_armored: ("Armoured deck - multiple decks", DeckType::MultipleArmored),
            display_single_armored: ("Armoured deck - single deck", DeckType::SingleArmored),
            display_multiple_protected: ("Protected deck - multiple decks", DeckType::MultipleProtected),
            display_single_protected: ("Protected deck - single deck", DeckType::SingleProtected),
            display_box_machinery: ("Box over machinery", DeckType::BoxOverMachinery),
            display_box_magazine: ("Box over magazines", DeckType::BoxOverMagazine),
            display_box_both: ("Box over machinery & magazines", DeckType::BoxOverBoth),
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
            // name: (type, index)
            from_str_default: (DeckType::MultipleArmored, "default"),
            from_str_zero:    (DeckType::MultipleArmored, "0"),
            from_str_one:     (DeckType::SingleArmored, "1"),
            from_str_two:     (DeckType::MultipleProtected, "2"),
            from_str_three:   (DeckType::SingleProtected, "3"),
            from_str_four:    (DeckType::BoxOverMachinery, "4"),
            from_str_five:    (DeckType::BoxOverMagazine, "5"),
            from_str_six:     (DeckType::BoxOverBoth, "6"),
        }

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(DeckType::from("0"), DeckType::MultipleArmored);
            assert_eq!(DeckType::from("3"), DeckType::SingleProtected);
            assert_eq!(DeckType::from("6"), DeckType::BoxOverBoth);
        }

        #[test]
        fn index_roundtrip() {
            for v in DeckType::ALL {
                assert_eq!(DeckType::from_index(v.index()), *v);
                assert_eq!(DeckType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(DeckType::from("99"), DeckType::default());
            assert_eq!(DeckType::from("abc"), DeckType::default());
            assert_eq!(DeckType::from(""), DeckType::default());
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = DeckType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["Armoured deck - multiple decks", "Armoured deck - single deck",
                 "Protected deck - multiple decks", "Protected deck - single deck",
                 "Box over machinery", "Box over magazines",
                 "Box over machinery & magazines"]
            );
        }
    }

    // Testing BulkheadType {{{2
    mod bulkhead_type {
        use super::*;

        // Test from/index round-trip {{{3
        #[test]
        fn from_matches_sship_codes() {
            assert_eq!(BulkheadType::from("0"), BulkheadType::Additional);
            assert_eq!(BulkheadType::from("1"), BulkheadType::Strengthened);
        }

        #[test]
        fn index_roundtrip() {
            for v in BulkheadType::ALL {
                assert_eq!(BulkheadType::from_index(v.index()), *v);
                assert_eq!(BulkheadType::from(v.index().to_string()), *v);
            }
        }

        #[test]
        fn from_unknown_falls_back_to_default() {
            assert_eq!(BulkheadType::from("99"), BulkheadType::default());
            assert_eq!(BulkheadType::from("abc"), BulkheadType::default());
            assert_eq!(BulkheadType::from(""), BulkheadType::default());
        }

        #[test]
        fn default_is_additional() {
            assert_eq!(BulkheadType::default(), BulkheadType::Additional);
        }

        #[test]
        fn labels_match_dropdown_order() {
            let labels: Vec<&str> = BulkheadType::ALL.iter().map(|v| v.label()).collect();
            assert_eq!(
                labels,
                ["Additional bulkheads", "Strengthened bulkheads"]
            );
        }
    }
}
