use crate::labels_enum;
use serde::{Deserialize, Serialize};

// Units {{{1
#[derive(PartialEq, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum Units {
    #[default]
    Imperial,
    Metric,
}

impl From<String> for Units { // {{{2
    fn from(index: String) -> Self {
        index.as_str().into()
    }
}

impl From<&str> for Units {
    fn from(index: &str) -> Self {
        match index {
            "1" => Self::Metric,
            _   => Self::Imperial, // "0" and unknown strings default to imperial
        }
    }
}

// UnitType {{{1
#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub enum UnitType {
    #[default]
    LengthSmall,
    LengthLong,
    Area,
    Weight,
    Power,
    WeightPerArea,
}

labels_enum!(UnitType {
    LengthSmall   => ("in",        "mm"),
    LengthLong    => ("ft",        "m"),
    Area          => ("sq ft",     "sq m"),
    Weight        => ("lbs",       "kg"),
    Power         => ("hp",        "kW"),
    WeightPerArea => ("lbs/sq ft", "kg/sq m"),
});

impl UnitType { // {{{2
    const INCH2MM: f64         = 25.4; // exact
    const FEET2METERS: f64     = 0.3048; // exact
    const SQFEET2SQMETERS: f64 = 0.09290304;
    const POUND2KG: f64        = 0.45359237; // exact
    const HP2KW: f64           = 0.74569987;

    const fn imperial_to_metric(&self) -> f64 {
        match self {
            Self::LengthSmall   => Self::INCH2MM,
            Self::LengthLong    => Self::FEET2METERS,
            Self::Area          => Self::SQFEET2SQMETERS,
            Self::Weight        => Self::POUND2KG,
            Self::Power         => Self::HP2KW,
            Self::WeightPerArea => Self::POUND2KG / Self::SQFEET2SQMETERS,
        }
    }
}

// Measurement {{{1
//
#[derive(PartialEq, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Measurement {
    pub imp_v: f64,
    pub metric_v: f64,
    units: Units,
    factor: f64,
}

impl Measurement { // {{{2
    pub const fn new(v: f64, unit_type: UnitType, units: Units) -> Self {
        let factor = unit_type.imperial_to_metric();
        let (imp_v, metric_v) = match units {
            Units::Imperial => (v, v * factor),
            Units::Metric   => (v / factor, v)
        };
        Self {imp_v, metric_v, units, factor}
    }

    pub fn metric(&self) -> f64 {
        self.metric_v
    }

    pub fn imp(&self) -> f64 {
        self.imp_v
    }

    pub fn set_units(&mut self, u: Units) {
        self.units = u;
    }
}

// Testing {{{1
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;

    const INCH2MM: f64         = 25.4;
    const FEET2METERS: f64     = 0.3048;
    const SQFEET2SQMETERS: f64 = 0.09290304;
    const POUND2KG: f64        = 0.45359237;
    const HP2KW: f64           = 0.74569987;

    // Test from {{{2
    macro_rules! test_from {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, index) = $value;

                    assert_eq!(expected, Units::from(index));
                }
            )*
        }
    }

    test_from! {
        // name:       (units, index)
        from_0:        (Units::Imperial, "0"),
        from_1:        (Units::Metric, "1"),
        from_invalid:  (Units::Imperial, "2"),
        from_garbage:  (Units::Imperial, "garbage"),
        from_empty:    (Units::Imperial, ""),
    }

    // Test default {{{2
    #[test]
    fn default_imperial() {
        assert_eq!(Units::Imperial, Units::default());
    }

    // Test measurement metric {{{2
    macro_rules! test_measurement_metric {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, value, unit_type, units) = $value;
                    let m = Measurement::new(value, unit_type, units);

                    assert_eq!(to_place(expected, 6), to_place(m.metric(), 6));
                }
            )*
        }
    }

    test_measurement_metric! {
        mm_len_small:   (1.0 * INCH2MM,                    1.0, UnitType::LengthSmall,   Units::Imperial),
        mm_len_long:    (1.0 * FEET2METERS,                1.0, UnitType::LengthLong,    Units::Imperial),
        mm_area:        (1.0 * SQFEET2SQMETERS,            1.0, UnitType::Area,          Units::Imperial),
        mm_weight:      (1.0 * POUND2KG,                   1.0, UnitType::Weight,        Units::Imperial),
        mm_power:       (1.0 * HP2KW,                      1.0, UnitType::Power,         Units::Imperial),
        mm_wgt_area:    (1.0 / SQFEET2SQMETERS * POUND2KG, 1.0, UnitType::WeightPerArea, Units::Imperial),
        mm_len_small_m: (1.0,                              1.0, UnitType::LengthSmall,   Units::Metric),
        mm_len_long_m:  (1.0,                              1.0, UnitType::LengthLong,    Units::Metric),
        mm_area_m:      (1.0,                              1.0, UnitType::Area,          Units::Metric),
        mm_weight_m:    (1.0,                              1.0, UnitType::Weight,        Units::Metric),
        mm_power_m:     (1.0,                              1.0, UnitType::Power,         Units::Metric),
        mm_wgt_area_m:  (1.0,                              1.0, UnitType::WeightPerArea, Units::Metric),
    }

    // Test measurement imp {{{2
    macro_rules! test_measurement_imp {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, value, unit_type, units) = $value;
                    let m = Measurement::new(value, unit_type, units);

                    assert_eq!(to_place(expected, 6), to_place(m.imp(), 6));
                }
            )*
        }
    }

    test_measurement_imp! {
        imp_len_small:  (1.0 / INCH2MM, 1.0, UnitType::LengthSmall, Units::Metric),
        imp_len_long:   (1.0 / FEET2METERS, 1.0, UnitType::LengthLong, Units::Metric),
        imp_area:       (1.0 / SQFEET2SQMETERS, 1.0, UnitType::Area, Units::Metric),
        imp_weight:     (1.0 / POUND2KG, 1.0, UnitType::Weight, Units::Metric),
        imp_power:      (1.0 / HP2KW, 1.0, UnitType::Power, Units::Metric),
        imp_wgt_area:   (1.0 / POUND2KG * SQFEET2SQMETERS, 1.0, UnitType::WeightPerArea, Units::Metric),
        imp_len_small_i: (1.0, 1.0, UnitType::LengthSmall, Units::Imperial),
        imp_len_long_i:  (1.0, 1.0, UnitType::LengthLong, Units::Imperial),
        imp_area_i:      (1.0, 1.0, UnitType::Area, Units::Imperial),
        imp_weight_i:    (1.0, 1.0, UnitType::Weight, Units::Imperial),
        imp_power_i:     (1.0, 1.0, UnitType::Power, Units::Imperial),
        imp_wgt_area_i:  (1.0, 1.0, UnitType::WeightPerArea, Units::Imperial),
    }

    // Test labels and ALL {{{2
    #[test]
    fn labels_match_expected() {
        assert_eq!(UnitType::LengthSmall.labels(),   &["in", "mm"]);
        assert_eq!(UnitType::LengthLong.labels(),    &["ft", "m"]);
        assert_eq!(UnitType::Area.labels(),          &["sq ft", "sq m"]);
        assert_eq!(UnitType::Weight.labels(),        &["lbs", "kg"]);
        assert_eq!(UnitType::Power.labels(),         &["hp", "kW"]);
        assert_eq!(UnitType::WeightPerArea.labels(), &["lbs/sq ft", "kg/sq m"]);

        assert_eq!(UnitType::LengthSmall.ALL(),   &["in", "mm"]);
        assert_eq!(UnitType::LengthLong.ALL(),    &["ft", "m"]);
        assert_eq!(UnitType::Area.ALL(),          &["sq ft", "sq m"]);
        assert_eq!(UnitType::Weight.ALL(),        &["lbs", "kg"]);
        assert_eq!(UnitType::Power.ALL(),         &["hp", "kW"]);
        assert_eq!(UnitType::WeightPerArea.ALL(), &["lbs/sq ft", "kg/sq m"]);
    }

    // Test from_index {{{2
    #[test]
    fn from_index_label() {
        assert_eq!(UnitType::LengthSmall.from_index(0),   "in");
        assert_eq!(UnitType::LengthSmall.from_index(1),   "mm");
        assert_eq!(UnitType::LengthLong.from_index(0),    "ft");
        assert_eq!(UnitType::Area.from_index(1),          "sq m");
        assert_eq!(UnitType::Weight.from_index(0),        "lbs");
        assert_eq!(UnitType::Power.from_index(1),         "kW");
        assert_eq!(UnitType::WeightPerArea.from_index(0), "lbs/sq ft");
    }

    // Test from_index out of range falls back to first label {{{2
    #[test]
    fn from_index_out_of_range_falls_back_to_first() {
        assert_eq!(UnitType::LengthSmall.from_index(5), "in");
        assert_eq!(UnitType::Weight.from_index(2),      "lbs");
    }
}
