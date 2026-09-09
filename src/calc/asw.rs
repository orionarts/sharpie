use crate::calc::{Measurement, Ship, Units};
use crate::{addif, addto, choice_enum};

use serde::{Deserialize, Serialize};

use std::fmt;

// ASW {{{1
/// ASW weapons and deployment gear.
///
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ASW {
    /// Units.
    pub units: Units,

    /// Year ASW system was designed.
    pub year: u32,

    /// Number of weapons.
    pub num: u32,
    /// Number of reloads.
    pub reload: u32,

    /// Weight of a single weapon.
    pub wgt: Measurement,

    /// Type of weapon.
    pub kind: ASWType,
}

impl ASW { // {{{2
    // wgt {{{3
    /// Weight of weapons, reloads and mounts.
    ///
    pub fn wgt(&self) -> f64 {
        self.wgt_weaps() + self.wgt_mounts()
    }

    // wgt_weaps {{{3
    /// Weight of weapons and reloads.
    ///
    pub fn wgt_weaps(&self) -> f64 {
        (self.num + self.reload) as f64 * self.wgt.imp() / Ship::POUND2TON
    }

    // wgt_mounts {{{3
    /// Weight of mounts.
    ///
    pub fn wgt_mounts(&self) -> f64 {
        self.wgt_weaps() * self.kind.mount_wgt_factor()
    }

    // desc {{{3
    /// Print a description of the number and type of ASW weapons.
    ///
    pub fn desc(&self) -> String {
        format!("{} - {:.2} lbs / {:.2} kg {}{}",
            self.num,
            self.wgt.imp(),
            self.wgt.metric(),
            self.kind.desc(),
            addif!(self.reload > 0, " + {} reloads", self.reload),
        )
    }

    // long_desc {{{3
    /// Print a full description of the ASW weapons.
    ///
    pub fn long_desc(&self) -> Vec<String> {
        let mut d: Vec<String> = Vec::new();

        addto!(d, "{} - {:.3} t total",
            self.desc(),
            self.wgt_weaps()
        );
        if self.kind.dc_desc() != "" {
            addto!(d, "    {}", self.kind.dc_desc());
        }

        d
    }
}

// ASWType {{{1
/// Type of ASW deployment gear.
///
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug, Default)]
pub enum ASWType {
    #[default]
    SternRacks,
    Throwers,
    Hedgehogs,
    SquidMortars,
}

choice_enum!(ASWType {
    SternRacks   => ("Stern depth charge racks"),
    Throwers     => ("Depth charge throwers"),
    Hedgehogs    => ("Hedgehog style A/S mortars"),
    SquidMortars => ("Squid style A/S mortars"),
});

impl ASWType { // {{{2
    // mount_wgt_factor {{{3
    /// Multiplier used to calculate total mount weight.
    ///
    pub fn mount_wgt_factor(&self) -> f64 {
        match self {
            Self::SternRacks   => 0.25,
            Self::Throwers     => 0.5,
            Self::Hedgehogs    => 0.5,
            Self::SquidMortars => 10.0,
        }
    }

    // desc {{{3
    /// Description of deployment gear.
    ///
    pub fn desc(&self) -> String {
        match self {
            Self::SternRacks   => "Depth Charges",
            Self::Throwers     => "Depth Charges",
            Self::Hedgehogs    => "ahead throwing AS Mortars",
            Self::SquidMortars => "trainable AS Mortars",
        }.into()
    }

    // dc_desc {{{3
    /// Description used to differentiate DC types.
    ///
    pub fn dc_desc(&self) -> String {
        match self {
            Self::SternRacks   => "in Stern depth charge racks",
            Self::Throwers     => "in Depth depth throwers",
            Self::Hedgehogs    => "",
            Self::SquidMortars => "",
        }.into()
    }
}

// Tests {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;
    use crate::calc::UnitType;

    // Test mount_wgt_factor {{{3
    macro_rules! test_mount_wgt_factor {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, asw) = $value;

                    assert_eq!(expected, asw.mount_wgt_factor());
                }
            )*
        }
    }

    test_mount_wgt_factor! {
        // name: (factor, asw)
        racks:   (0.25, ASWType::SternRacks),
        throw:   (0.5, ASWType::Throwers),
        hedge:   (0.5, ASWType::Hedgehogs),
        squid:   (10.0, ASWType::SquidMortars),
    }

    // Test from/index round-trip {{{3
    #[test]
    fn from_matches_sship_codes() {
        assert_eq!(ASWType::from("0"), ASWType::SternRacks);
        assert_eq!(ASWType::from("1"), ASWType::Throwers);
        assert_eq!(ASWType::from("2"), ASWType::Hedgehogs);
        assert_eq!(ASWType::from("3"), ASWType::SquidMortars);
    }

    #[test]
    fn index_roundtrip() {
        for v in ASWType::ALL {
            assert_eq!(ASWType::from_index(v.index()), *v);
            assert_eq!(ASWType::from(v.index().to_string()), *v);
        }
    }

    #[test]
    fn from_unknown_falls_back_to_default() {
        assert_eq!(ASWType::from("99"), ASWType::default());
        assert_eq!(ASWType::from("abc"), ASWType::default());
        assert_eq!(ASWType::from(""), ASWType::default());
    }

    #[test]
    fn labels_match_dropdown_order() {
        let labels: Vec<&str> = ASWType::ALL.iter().map(|v| v.label()).collect();
        assert_eq!(
            labels,
            ["Stern depth charge racks", "Depth charge throwers",
             "Hedgehog style A/S mortars", "Squid style A/S mortars"]
        );
    }

    // Test asw_wgt_weaps {{{3
    macro_rules! test_asw_wgt_weaps {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut asw = ASW::default();
                    asw.kind = kind; asw.num = num; asw.reload = reload;
                    asw.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(expected, to_place(asw.wgt_weaps(), 3));
                }
            )*
        }
    }
    test_asw_wgt_weaps! {
        // name:                     (wgt, kind, num, reload, wgt)
        wgt_weaps_asw_stern_racks:   (0.893, ASWType::SternRacks, 100, 100, 10.0),
        wgt_weaps_asw_throwers:      (0.893, ASWType::Throwers, 100, 100, 10.0),
        wgt_weaps_asw_hedgehogs:     (0.893, ASWType::Hedgehogs, 100, 100, 10.0),
        wgt_weaps_asw_squid_mortars: (0.893, ASWType::SquidMortars, 100, 100, 10.0),
    }

    // Test asw_wgt_mounts {{{3
    macro_rules! test_asw_wgt_mounts {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut asw = ASW::default();
                    asw.kind = kind; asw.num = num; asw.reload = reload;
                    asw.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(expected, to_place(asw.wgt_mounts(), 3));
                }
            )*
        }
    }
    test_asw_wgt_mounts! {
        // name:                      (expected, kind, num, reload, wgt)
        wgt_mounts_asw_stern_racks:   (0.223, ASWType::SternRacks, 100, 100, 10.0),
        wgt_mounts_asw_throwers:      (0.446, ASWType::Throwers, 100, 100, 10.0),
        wgt_mounts_asw_hedgehogs:     (0.446, ASWType::Hedgehogs, 100, 100, 10.0),
        wgt_mounts_asw_squid_mortars: (8.929, ASWType::SquidMortars, 100, 100, 10.0),
    }

    // Test asw_wgt {{{3
    macro_rules! test_asw_wgt {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, kind, num, reload, wgt) = $value;

                    let mut asw = ASW::default();
                    asw.kind = kind; asw.num = num; asw.reload = reload;
                    asw.wgt = Measurement::new(wgt, UnitType::Weight, Units::Imperial);

                    assert_eq!(to_place(expected, 3), to_place(asw.wgt(), 3));
                }
            )*
        }
    }
    test_asw_wgt! {
        // name:                      (expected, kind, num, reload, wgt)
        wgt_asw_stern_racks:   (200.0 * 10.0 / Ship::POUND2TON * (1.0 + ASWType::SternRacks.mount_wgt_factor()), ASWType::SternRacks, 100, 100, 10.0),
        wgt_asw_throwers:      (200.0 * 10.0 / Ship::POUND2TON * (1.0 + ASWType::Throwers.mount_wgt_factor()), ASWType::Throwers, 100, 100, 10.0),
        wgt_asw_hedgehogs:     (200.0 * 10.0 / Ship::POUND2TON * (1.0 + ASWType::Hedgehogs.mount_wgt_factor()), ASWType::Hedgehogs, 100, 100, 10.0),
        wgt_asw_squid_mortars: (200.0 * 10.0 / Ship::POUND2TON * (1.0 + ASWType::SquidMortars.mount_wgt_factor()), ASWType::SquidMortars, 100, 100, 10.0),
    }
}
