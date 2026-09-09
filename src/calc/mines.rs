use crate::calc::{Measurement, Ship, Units};
use crate::{addif, addto, choice_enum};

use serde::{Deserialize, Serialize};

use std::fmt;

// Mines {{{1
/// Mines and deployment gear.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Mines {
    /// Units
    pub units: Units,

    /// Year mines were designed.
    pub year: u32,

    /// Number of mines.
    pub num: u32,
    /// Number of mine reloads.
    pub reload: u32,

    /// Weight of a single mine.
    pub wgt: Measurement,

    /// Type of mine deployment system.
    pub kind: MineType,
}

impl Mines { // {{{2
    // wgt {{{3
    /// Weight of mines, reloads and deployment gear.
    ///
    pub fn wgt(&self) -> f64 {
        self.wgt_weaps() + self.wgt_mounts()
    }

    // wgt_weaps {{{3
    /// Weight of mines and reloads.
    ///
    pub fn wgt_weaps(&self) -> f64 {
        (self.num + self.reload) as f64 * self.wgt.imp() / Ship::POUND2TON
    }

    // wgt_mounts {{{3
    /// Weight of deployment gear.
    ///
    pub fn wgt_mounts(&self) -> f64 {
        self.wgt_weaps() * self.kind.wgt_factor()
    }

    // desc {{{3
    /// Print a description of the number and type of mines.
    ///
    pub fn desc(&self) -> String {
        format!("{} - {:.2} lbs / {:.2} kg mines{}",
            self.num,
            self.wgt.imp(),
            self.wgt.metric(),
            addif!(self.reload > 0, " + {} reloads", self.reload),
        )
    }

    // long_desc {{{3
    /// Print a full description of the mines.
    ///
    pub fn long_desc(&self) -> Vec<String> {
        let mut d: Vec<String> = Vec::new();

        addto!(d, "{} - {:.3} t total",
            self.desc(),
            self.wgt_weaps()
        );
        addto!(d, "    {}",
            self.kind.desc()
        );

        d
    }
}

// MineType {{{1
/// Types of mine deployment gear.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum MineType {
    #[default]
    SternRails,
    BowTubes,
    SternTubes,
    SideTubes,
}

choice_enum!(MineType {
    SternRails => ("Above water - Stern racks/rails"),
    BowTubes   => ("Below water - bow tubes"),
    SternTubes => ("Below water - stern tubes"),
    SideTubes  => ("Below water - side tubes"),
});

impl MineType { // {{{2
    // wgt_factor {{{3
    /// Multiplier to determine weight of mine deployment gear.
    ///
    pub fn wgt_factor(&self) -> f64 {
        match self {
            Self::SternRails => 0.25,
            Self::BowTubes   => 1.0,
            Self::SternTubes => 1.0,
            Self::SideTubes  => 1.0,
        }
    }

    // desc {{{3
    /// Description of mine deployment gear type.
    ///
    pub fn desc(&self) -> String {
        match self {
            Self::SternRails => "in Above water - Stern racks/rails",
            Self::BowTubes   => "in Below water - bow tubes",
            Self::SternTubes => "",
            Self::SideTubes  => "",
        }.into()
    }
}

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;
    use crate::calc::UnitType;

    // Test wgt_factor {{{3
    macro_rules! test_wgt_factor {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, mines) = $value;

                    assert_eq!(expected, mines.wgt_factor());
                }
            )*
        }
    }

    test_wgt_factor! {
        // name: (factor, mines)
        rails:   (0.25, MineType::SternRails),
        bow:     (1.0, MineType::BowTubes),
        stern:   (1.0, MineType::SternTubes),
        side:    (1.0, MineType::SideTubes),
    }

    // Test from/index round-trip {{{3
    #[test]
    fn from_matches_sship_codes() {
        assert_eq!(MineType::from("0"), MineType::SternRails);
        assert_eq!(MineType::from("1"), MineType::BowTubes);
        assert_eq!(MineType::from("2"), MineType::SternTubes);
        assert_eq!(MineType::from("3"), MineType::SideTubes);
    }

    #[test]
    fn index_roundtrip() {
        for v in MineType::ALL {
            assert_eq!(MineType::from_index(v.index()), *v);
            assert_eq!(MineType::from(v.index().to_string()), *v);
        }
    }

    #[test]
    fn from_unknown_falls_back_to_default() {
        assert_eq!(MineType::from("99"), MineType::default());
        assert_eq!(MineType::from("abc"), MineType::default());
        assert_eq!(MineType::from(""), MineType::default());
    }

    #[test]
    fn labels_match_dropdown_order() {
        let labels: Vec<&str> = MineType::ALL.iter().map(|v| v.label()).collect();
        assert_eq!(
            labels,
            ["Above water - Stern racks/rails", "Below water - bow tubes",
             "Below water - stern tubes", "Below water - side tubes"]
        );
    }

    // Test mines_wgt_weaps {{{3
    macro_rules! test_mines_wgt_weaps {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut mines = Mines::default();
                    mines.kind = kind;
                    mines.num = num;
                    mines.reload = reload;
                    mines.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(to_place(expected, 3), to_place(mines.wgt_weaps(), 3));
                }
            )*
        }
    }
    test_mines_wgt_weaps! {
        // name:                    (expected, kind, num, reload, wgt)
        wgt_weaps_mines_stern_rails: (200.0 * 10.0 / Ship::POUND2TON, MineType::SternRails, 100, 100, 10.0),
        wgt_weaps_mines_bow_tubes:   (200.0 * 10.0 / Ship::POUND2TON, MineType::BowTubes, 100, 100, 10.0),
        wgt_weaps_mines_stern_tubes: (200.0 * 10.0 / Ship::POUND2TON, MineType::SternTubes, 100, 100, 10.0),
        wgt_weaps_mines_side_tubes:  (200.0 * 10.0 / Ship::POUND2TON, MineType::SideTubes, 100, 100, 10.0),
    }

    // Test mines_wgt_mounts {{{3
    macro_rules! test_mines_wgt_mounts {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut mines = Mines::default();
                    mines.kind = kind;
                    mines.num = num;
                    mines.reload = reload;
                    mines.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(to_place(expected, 3), to_place(mines.wgt_mounts(), 3));
                }
            )*
        }
    }
    test_mines_wgt_mounts! {
        // name:                    (expected, kind, num, reload, wgt)
        wgt_mounts_mines_stern_rails: (200.0 * 10.0 / Ship::POUND2TON * MineType::SternRails.wgt_factor(), MineType::SternRails, 100, 100, 10.0),
        wgt_mounts_mines_bow_tubes:   (200.0 * 10.0 / Ship::POUND2TON * MineType::BowTubes.wgt_factor(), MineType::BowTubes, 100, 100, 10.0),
        wgt_mounts_mines_stern_tubes: (200.0 * 10.0 / Ship::POUND2TON * MineType::SternTubes.wgt_factor(), MineType::SternTubes, 100, 100, 10.0),
        wgt_mounts_mines_side_tubes:  (200.0 * 10.0 / Ship::POUND2TON * MineType::SideTubes.wgt_factor(), MineType::SideTubes, 100, 100, 10.0),
    }

    // Test mines_wgt {{{3
    macro_rules! test_mines_wgt {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut mines = Mines::default();
                    mines.kind = kind;
                    mines.num = num;
                    mines.reload = reload;
                    mines.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(to_place(expected, 3), to_place(mines.wgt(), 3));
                }
            )*
        }
    }
    test_mines_wgt! {
        // name:                    (expected, kind, num, reload, wgt)
        wgt_mines_stern_rails: (200.0 * 10.0 / Ship::POUND2TON * (1.0 + MineType::SternRails.wgt_factor()), MineType::SternRails, 100, 100, 10.0),
        wgt_mines_bow_tubes:   (200.0 * 10.0 / Ship::POUND2TON * (1.0 + MineType::BowTubes.wgt_factor()), MineType::BowTubes, 100, 100, 10.0),
        wgt_mines_stern_tubes: (200.0 * 10.0 / Ship::POUND2TON * (1.0 + MineType::SternTubes.wgt_factor()), MineType::SternTubes, 100, 100, 10.0),
        wgt_mines_side_tubes:  (200.0 * 10.0 / Ship::POUND2TON * (1.0 + MineType::SideTubes.wgt_factor()), MineType::SideTubes, 100, 100, 10.0),
    }
}
