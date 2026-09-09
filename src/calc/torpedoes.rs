use crate::calc::{Measurement, Units};
use crate::{addto, choice_enum};

use serde::{Deserialize, Serialize};

use std::f64::consts::PI;
use std::fmt;

// Torpedoes {{{1
/// A set of torpedo mounts or tubes.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Torpedoes {
    /// Units
    pub units: Units,
    /// Year torpedo was designed.
    pub year: u32,

    /// Number of mounts.
    pub mounts: u32,
    /// Type of mount.
    pub kind: TorpedoMountType,

    /// Number of torpedoes in the set
    pub num: u32,

    /// Torpedo diameter.
    pub diam: Measurement,
    /// Torpedo length.
    pub len: Measurement,
}

impl Torpedoes { // {{{2
    // wgt {{{3
    /// Weight of all torpedoes and mounts in the set.
    ///
    pub fn wgt(&self) -> f64 {
        self.wgt_weaps() + self.wgt_mounts()
    }

    // wgt_weaps {{{3
    /// Weight of torpedoes in the set.
    ///
    pub fn wgt_weaps(&self) -> f64 {
        let divisor = (f64::max(1907.0 - self.year as f64, 0.0) + 25.0) * 937.0;
        if divisor == 0.0 { return 0.0; }

        (
            PI * self.diam.imp().powf(2.0) * self.len.imp() /
                ( divisor) + (self.year as f64 - 1890.0) * 0.004
        ) * self.num as f64
    }

    // wgt_mounts {{{3
    /// Weight of mounts in the set.
    ///
    pub fn wgt_mounts(&self) -> f64 {
        self.kind.wgt_factor() * self.wgt_weaps()
    }

    // hull_space {{{3
    /// Hull space taken up by the set.
    ///
    pub fn hull_space(&self) -> f64 {
        self.kind.hull_space(self.len.imp(), self.diam.imp()) * self.num as f64
    }

    // deck_space {{{3
    /// Deck space taken up by the set.
    ///
    pub fn deck_space(&self, b: f64) -> f64 {
        self.kind.deck_space(b, self.num, self.len.imp(), self.diam.imp(), self.mounts)
    }

    // desc {{{3
    /// Print a description of the number and type of torpedoes.
    ///
    pub fn desc(&self) -> String {
        format!("{} - {:.1}\" / {:.0} mm, {:.2} ft / {:.2} m torpedo{}",
            self.num,
            self.diam.imp(),
            self.diam.metric(),
            self.len.imp(),
            self.len.metric(),
            match self.num { 1 => "", _ => "es", },
        )
    }

    // long_desc {{{3
    /// Print a full description of the torpedoes.
    ///
    pub fn long_desc(&self) -> Vec<String> {
        let mut d: Vec<String> = Vec::new();

        addto!(d, "{} {} {:.3} t total",
            self.desc(),
            match self.num {
                1 => "-".to_string(),
                _ => format!("- {:.3} t each,", self.wgt_weaps() / self.num as f64),
            },
            self.wgt_weaps()
        );
        addto!(d, "    {}",
            self.kind.desc(self.num, self.mounts)
        );

        d
    }
}

// TorpedoMountType {{{1
/// Type of torpedo mount.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum TorpedoMountType {
    #[default]
    FixedTubes,
    DeckSideTubes,
    CenterTubes,
    DeckReloads,
    BowTubes,
    SternTubes,
    BowAndSternTubes,
    SubmergedSideTubes,
    SubmergedReloads,
}

choice_enum!(TorpedoMountType {
    FixedTubes         => ("deck mounted carriage/fixed tube"),
    DeckSideTubes      => ("deck mounted side rotating tube"),
    CenterTubes        => ("deck mounted centre rotating tube"),
    DeckReloads        => ("deck mounted reload"),
    BowTubes           => ("submerged bow tube"),
    SternTubes         => ("submerged stern tube"),
    BowAndSternTubes   => ("submerged bow & stern tube"),
    SubmergedSideTubes => ("submerged side tube"),
    SubmergedReloads   => ("below water reload"),
});

impl TorpedoMountType { // {{{2
    // wgt_factor {{{3
    /// Multiplier used to determine weight of torpedo mounts.
    ///
    pub fn wgt_factor(&self) -> f64 {
        match self {
            Self::FixedTubes         => 0.25,
            Self::DeckSideTubes      => 1.0,
            Self::CenterTubes        => 1.0,
            Self::DeckReloads        => 0.25,
            Self::BowTubes           => 1.0,
            Self::SternTubes         => 1.0,
            Self::BowAndSternTubes   => 1.0,
            Self::SubmergedSideTubes => 1.0,
            Self::SubmergedReloads   => 0.25,
        }
    }

    // hull_space {{{3
    /// Hull space taken up by torpedo mounts.
    ///
    pub fn hull_space(&self, len: f64, diam: f64) -> f64 {
        match self {
            Self::FixedTubes |
            Self::DeckSideTubes |
            Self::CenterTubes |
            Self::DeckReloads => 0.0,

            Self::BowTubes |
            Self::SternTubes |
            Self::BowAndSternTubes |
            Self::SubmergedSideTubes => len * 2.5 * (diam * 2.75/12.0).powf(2.0),

            Self::SubmergedReloads   => len * 1.5 * (diam * 1.5/12.0).powf(2.0),
        }
    }

    // deck_space {{{3
    /// Deck space taken up by torpedo mounts.
    ///
    pub fn deck_space(&self, b: f64, num: u32, len: f64, diam: f64, mounts: u32) -> f64 {
        if mounts == 0 { return 0.0; }

        let num = num as f64;
        let mounts = mounts as f64;

        match self {
            Self::FixedTubes => len * diam / 12.0 * num,

            Self::DeckSideTubes => {
                f64::powf(
                    f64::sqrt(
                        f64::powf(len,2.0) + f64::powf(((num/mounts)*diam/12.0)+(num/mounts-1.0)*0.5,2.0)
                    )*0.5,2.0
                )*PI+(((num/mounts)*diam/12.0)+(num/mounts-1.0)*0.5)*0.5*len
            },

            Self::CenterTubes => {
                let x = f64::powf(len, 2.0);
                let y = f64::powf((num / mounts) * diam / 12.0 + (num / mounts-1.0) * 0.5, 2.0);

                f64::sqrt(x + y) * b * mounts
            }

            Self::DeckReloads => len * 1.5 * (diam + 6.0) / 12.0 * num,

            Self::BowTubes |
            Self::SternTubes |
            Self::BowAndSternTubes |
            Self::SubmergedSideTubes |
            Self::SubmergedReloads   => 0.0,
        }
    }

    // desc {{{3
    /// Description of torpedo mounts.
    ///
    pub fn desc(&self, tubes: u32, mounts: u32) -> String {
        let desc = match self {
            Self::FixedTubes         => "deck mounted carriage/fixed tube",
            Self::DeckSideTubes      => "deck mounted side rotating tube",
            Self::CenterTubes        => "deck mounted centre rotating tube",
            Self::DeckReloads        => "deck mounted reload",
            Self::BowTubes           => "submerged bow tube",
            Self::SternTubes         => "submerged stern tube",
            Self::BowAndSternTubes   => &format!("submerged bow {} stern tube", if tubes > 1 { "&" } else { "OR" }).to_owned(),
            Self::SubmergedSideTubes => "submerged side tube",
            Self::SubmergedReloads   => "below water reload",
        };

        let prefix = match self {
            Self::FixedTubes |
            Self::DeckSideTubes |
            Self::CenterTubes |
            Self::DeckReloads => {
                if tubes > 1 {
                    format!("In {} sets of ", mounts)
                } else {
                    "In a ".into()
                }
            }

            _ => "".into(),
        };

        prefix + desc + if tubes > 1 { "s" } else { "" }
    }
}

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;
    use crate::calc::UnitType;

    // Formula for Torpedoes::wgt_weaps().
    fn torp_weaps_wgt(diam: f64, len: f64, num: u32, year: u32) -> f64 {
        (PI * diam.powf(2.0) * len / ((f64::max(1907.0 - year as f64, 0.0) + 25.0) * 937.0)
            + (year as f64 - 1890.0) * 0.004)
            * num as f64
    }

    // Test wgt_factor {{{3
    macro_rules! test_wgt_factor {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, torp) = $value;

                    assert_eq!(expected, torp.wgt_factor());
                }
            )*
        }
    }

    test_wgt_factor! {
        // name:               (factor, torp)
        wgt_factor_fixed:      (0.25, TorpedoMountType::FixedTubes),
        wgt_factor_deck:       (1.0, TorpedoMountType::DeckSideTubes),
        wgt_factor_center:     (1.0, TorpedoMountType::CenterTubes),
        wgt_factor_reload:     (0.25, TorpedoMountType::DeckReloads),
        wgt_factor_bow:        (1.0, TorpedoMountType::BowTubes),
        wgt_factor_stern:      (1.0, TorpedoMountType::SternTubes),
        wgt_factor_bow_stern:  (1.0, TorpedoMountType::BowAndSternTubes),
        wgt_factor_sub_side:   (1.0, TorpedoMountType::SubmergedSideTubes),
        wgt_factor_sub_reload: (0.25, TorpedoMountType::SubmergedReloads),
    }

    // Test hull_space {{{3
    macro_rules! test_hull_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, torp) = $value;

                    let len = 18.0; let diam = 21.0;
                    assert_eq!(expected, to_place(torp.hull_space(len, diam), 2));
                }
            )*
        }
    }

    test_hull_space! {
        // name:               (factor, torp)
        hull_space_fixed:      (0.0, TorpedoMountType::FixedTubes),
        hull_space_deck:       (0.0, TorpedoMountType::DeckSideTubes),
        hull_space_center:     (0.0, TorpedoMountType::CenterTubes),
        hull_space_reload:     (0.0, TorpedoMountType::DeckReloads),
        hull_space_bow:        (1042.21, TorpedoMountType::BowTubes),
        hull_space_stern:      (1042.21, TorpedoMountType::SternTubes),
        hull_space_bow_stern:  (1042.21, TorpedoMountType::BowAndSternTubes),
        hull_space_sub_side:   (1042.21, TorpedoMountType::SubmergedSideTubes),
        hull_space_sub_reload: (186.05, TorpedoMountType::SubmergedReloads),
    }

    // Test deck_space {{{3
    macro_rules! test_deck_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, torp) = $value;

                    let len = 18.0; let diam = 21.0; let num = 2; let mounts = 2;
                    let b = 50.0;
                    assert_eq!(expected, to_place(torp.deck_space(b, num, len, diam, mounts), 2));
                }
            )*
        }
    }

    test_deck_space! {
        // name:               (factor, torp)
        deck_space_fixed:      (63.0, TorpedoMountType::FixedTubes),
        deck_space_deck:       (272.62, TorpedoMountType::DeckSideTubes),
        deck_space_center:     (1808.49, TorpedoMountType::CenterTubes),
        deck_space_reload:     (121.5, TorpedoMountType::DeckReloads),
        deck_space_bow:        (0.0, TorpedoMountType::BowTubes),
        deck_space_stern:      (0.0, TorpedoMountType::SternTubes),
        deck_space_bow_stern:  (0.0, TorpedoMountType::BowAndSternTubes),
        deck_space_sub_side:   (0.0, TorpedoMountType::SubmergedSideTubes),
        deck_space_sub_reload: (0.0, TorpedoMountType::SubmergedReloads),
    }

    // Test from/index round-trip {{{3
    #[test]
    fn from_matches_sship_codes() {
        assert_eq!(TorpedoMountType::from("0"), TorpedoMountType::FixedTubes);
        assert_eq!(TorpedoMountType::from("4"), TorpedoMountType::BowTubes);
        assert_eq!(TorpedoMountType::from("8"), TorpedoMountType::SubmergedReloads);
    }

    #[test]
    fn index_roundtrip() {
        for v in TorpedoMountType::ALL {
            assert_eq!(TorpedoMountType::from_index(v.index()), *v);
            assert_eq!(TorpedoMountType::from(v.index().to_string()), *v);
        }
    }

    #[test]
    fn from_unknown_falls_back_to_default() {
        assert_eq!(TorpedoMountType::from("99"), TorpedoMountType::default());
        assert_eq!(TorpedoMountType::from("abc"), TorpedoMountType::default());
        assert_eq!(TorpedoMountType::from(""), TorpedoMountType::default());
    }

    #[test]
    fn labels_match_dropdown_order() {
        let labels: Vec<&str> = TorpedoMountType::ALL.iter().map(|v| v.label()).collect();
        assert_eq!(
            labels,
            ["deck mounted carriage/fixed tube", "deck mounted side rotating tube",
             "deck mounted centre rotating tube", "deck mounted reload",
             "submerged bow tube", "submerged stern tube",
             "submerged bow & stern tube", "submerged side tube",
             "below water reload"]
        );
    }

    // Test torpedo_wgt_weaps {{{3
    macro_rules! test_torpedo_wgt_weaps {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, diam, len, num, year) = $value;

                    let mut torp = Torpedoes::default();
                    torp.kind = kind;
                    torp.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                    torp.len  = Measurement::new(len,  UnitType::LengthLong,  Units::Imperial);
                    torp.num = num; torp.year = year;

                    assert_eq!(to_place(expected, 3), to_place(torp.wgt_weaps(), 3));
                }
            )*
        }
    }
    test_torpedo_wgt_weaps! {
        // name:                       (wgt, kind, diam, len, num, year)
        wgt_weaps_torps_fixed_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::FixedTubes,         18.0, 21.0, 4, 1940),
        wgt_weaps_torps_deck_side_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::DeckSideTubes,      18.0, 21.0, 4, 1940),
        wgt_weaps_torps_center_tubes:        (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::CenterTubes,        18.0, 21.0, 4, 1940),
        wgt_weaps_torps_deck_reloads:        (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::DeckReloads,        18.0, 21.0, 4, 1940),
        wgt_weaps_torps_bow_tubes:           (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::BowTubes,           18.0, 21.0, 4, 1940),
        wgt_weaps_torps_stern_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::SternTubes,         18.0, 21.0, 4, 1940),
        wgt_weaps_torps_bow_and_stern_tubes: (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::BowAndSternTubes,   18.0, 21.0, 4, 1940),
        wgt_weaps_torps_submerged_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::SubmergedSideTubes, 18.0, 21.0, 4, 1940),
        wgt_weaps_torps_submerged_reloads:   (torp_weaps_wgt(18.0, 21.0, 4, 1940), TorpedoMountType::SubmergedReloads,   18.0, 21.0, 4, 1940),
    }

    // Test torpedo_wgt_mounts {{{3
    macro_rules! test_torpedo_wgt_mounts {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, diam, len, num, year) = $value;

                    let mut torp = Torpedoes::default();
                    torp.kind = kind;
                    torp.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                    torp.len  = Measurement::new(len,  UnitType::LengthLong,  Units::Imperial);
                    torp.num = num; torp.year = year;

                    assert_eq!(to_place(expected, 3), to_place(torp.wgt_mounts(), 3));
                }
            )*
        }
    }
    test_torpedo_wgt_mounts! {
        // name:                       (wgt, kind, diam, len, num, year)
        wgt_mounts_torps_fixed_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::FixedTubes.wgt_factor(),         TorpedoMountType::FixedTubes,         18.0, 21.0, 4, 1940),
        wgt_mounts_torps_deck_side_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::DeckSideTubes.wgt_factor(),       TorpedoMountType::DeckSideTubes,      18.0, 21.0, 4, 1940),
        wgt_mounts_torps_center_tubes:        (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::CenterTubes.wgt_factor(),         TorpedoMountType::CenterTubes,        18.0, 21.0, 4, 1940),
        wgt_mounts_torps_deck_reloads:        (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::DeckReloads.wgt_factor(),         TorpedoMountType::DeckReloads,        18.0, 21.0, 4, 1940),
        wgt_mounts_torps_bow_tubes:           (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::BowTubes.wgt_factor(),            TorpedoMountType::BowTubes,           18.0, 21.0, 4, 1940),
        wgt_mounts_torps_stern_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::SternTubes.wgt_factor(),          TorpedoMountType::SternTubes,         18.0, 21.0, 4, 1940),
        wgt_mounts_torps_bow_and_stern_tubes: (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::BowAndSternTubes.wgt_factor(),    TorpedoMountType::BowAndSternTubes,   18.0, 21.0, 4, 1940),
        wgt_mounts_torps_submerged_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::SubmergedSideTubes.wgt_factor(),  TorpedoMountType::SubmergedSideTubes, 18.0, 21.0, 4, 1940),
        wgt_mounts_torps_submerged_reloads:   (torp_weaps_wgt(18.0, 21.0, 4, 1940) * TorpedoMountType::SubmergedReloads.wgt_factor(),    TorpedoMountType::SubmergedReloads,   18.0, 21.0, 4, 1940),
    }

    // Test torpedo_wgt {{{3
    macro_rules! test_torpedo_wgt {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, diam, len, num, year) = $value;

                    let mut torp = Torpedoes::default();
                    torp.kind = kind;
                    torp.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                    torp.len  = Measurement::new(len,  UnitType::LengthLong,  Units::Imperial);
                    torp.num = num; torp.year = year;

                    assert_eq!(to_place(expected, 3), to_place(torp.wgt(), 3));
                }
            )*
        }
    }
    test_torpedo_wgt! {
        // name:                       (wgt, kind, diam, len, num, year)
        wgt_torps_fixed_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::FixedTubes.wgt_factor()),         TorpedoMountType::FixedTubes,         18.0, 21.0, 4, 1940),
        wgt_torps_deck_side_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::DeckSideTubes.wgt_factor()),       TorpedoMountType::DeckSideTubes,      18.0, 21.0, 4, 1940),
        wgt_torps_center_tubes:        (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::CenterTubes.wgt_factor()),         TorpedoMountType::CenterTubes,        18.0, 21.0, 4, 1940),
        wgt_torps_deck_reloads:        (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::DeckReloads.wgt_factor()),         TorpedoMountType::DeckReloads,        18.0, 21.0, 4, 1940),
        wgt_torps_bow_tubes:           (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::BowTubes.wgt_factor()),            TorpedoMountType::BowTubes,           18.0, 21.0, 4, 1940),
        wgt_torps_stern_tubes:         (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::SternTubes.wgt_factor()),          TorpedoMountType::SternTubes,         18.0, 21.0, 4, 1940),
        wgt_torps_bow_and_stern_tubes: (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::BowAndSternTubes.wgt_factor()),    TorpedoMountType::BowAndSternTubes,   18.0, 21.0, 4, 1940),
        wgt_torps_submerged_tubes:     (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::SubmergedSideTubes.wgt_factor()),  TorpedoMountType::SubmergedSideTubes, 18.0, 21.0, 4, 1940),
        wgt_torps_submerged_reloads:   (torp_weaps_wgt(18.0, 21.0, 4, 1940) * (1.0 + TorpedoMountType::SubmergedReloads.wgt_factor()),    TorpedoMountType::SubmergedReloads,   18.0, 21.0, 4, 1940),
    }

    // Test torpedo_hull_space {{{3
    macro_rules! test_torpedo_hull_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, diam, len, num) = $value;
                    let mut torp = Torpedoes::default();
                    torp.kind = kind;
                    torp.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                    torp.len  = Measurement::new(len,  UnitType::LengthLong,  Units::Imperial);
                    torp.num = num;

                    assert_eq!(expected, to_place(torp.hull_space(), 3));
                }
            )*
        }
    }
    test_torpedo_hull_space! {
        // name:                             (space, kind, diam, len, num)
        test_hull_space_fixed_tubes:         (0.0, TorpedoMountType::FixedTubes,         18.0, 21.0, 4),
        test_hull_space_deck_side_tubes:     (0.0, TorpedoMountType::DeckSideTubes,      18.0, 21.0, 4),
        test_hull_space_center_tubes:        (0.0, TorpedoMountType::CenterTubes,        18.0, 21.0, 4),
        test_hull_space_deck_reloads:        (0.0, TorpedoMountType::DeckReloads,        18.0, 21.0, 4),
        test_hull_space_bow_tubes:           (3573.281, TorpedoMountType::BowTubes,           18.0, 21.0, 4),
        test_hull_space_stern_tubes:         (3573.281, TorpedoMountType::SternTubes,         18.0, 21.0, 4),
        test_hull_space_bow_and_stern_tubes: (3573.281, TorpedoMountType::BowAndSternTubes,   18.0, 21.0, 4),
        test_hull_space_submerged_tubes:     (3573.281, TorpedoMountType::SubmergedSideTubes, 18.0, 21.0, 4),
        test_hull_space_submerged_reloads:   (637.875, TorpedoMountType::SubmergedReloads,   18.0, 21.0, 4),
    }

    // Test torpedo_deck_space {{{3
    macro_rules! test_torpedo_deck_space {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected,kind, diam, len, num, mounts) = $value;

                    let mut torp = Torpedoes::default();
                    torp.kind = kind;
                    torp.diam = Measurement::new(diam, UnitType::LengthSmall, Units::Imperial);
                    torp.len  = Measurement::new(len,  UnitType::LengthLong,  Units::Imperial);
                    torp.num = num; torp.mounts = mounts;

                    let b = 10.0;
                    assert_eq!(expected, to_place(torp.deck_space(b), 3));
                }
            )*
        }
    }
    test_torpedo_deck_space! {
        // name:                             (space, kind, diam, len, num, mounts)
        test_deck_space_fixed_tubes:         (126.0, TorpedoMountType::FixedTubes,         18.0, 21.0, 4, 2),
        test_deck_space_deck_side_tubes:     (392.732, TorpedoMountType::DeckSideTubes,      18.0, 21.0, 4, 2),
        test_deck_space_center_tubes:        (425.793, TorpedoMountType::CenterTubes,        18.0, 21.0, 4, 2),
        test_deck_space_deck_reloads:        (252.0, TorpedoMountType::DeckReloads,        18.0, 21.0, 4, 2),
        test_deck_space_bow_tubes:           (0.0, TorpedoMountType::BowTubes,           18.0, 21.0, 4, 2),
        test_deck_space_stern_tubes:         (0.0, TorpedoMountType::SternTubes,         18.0, 21.0, 4, 2),
        test_deck_space_bow_and_stern_tubes: (0.0, TorpedoMountType::BowAndSternTubes,   18.0, 21.0, 4, 2),
        test_deck_space_submerged_tubes:     (0.0, TorpedoMountType::SubmergedSideTubes, 18.0, 21.0, 4, 2),
        test_deck_space_submerged_reloads:   (0.0, TorpedoMountType::SubmergedReloads,   18.0, 21.0, 4, 2),
    }
}
