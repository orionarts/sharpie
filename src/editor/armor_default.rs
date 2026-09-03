//! Belt dimension estimation for the armor tab's "Default" button.
//!
//! These are stateless helpers ported from SpringSharp 3b3's
//! `beltButtonClick` (SpringSharp3b3.cs line 13164).

use crate::calc::Ship;

// DefaultBelts {{{1
/// Estimated belt dimensions (in imperial feet) for the armor "Default"
/// button.
///
pub struct DefaultBelts {
    /// Main belt length.
    pub main_len: f64,
    /// Main belt height.
    pub main_hgt: f64,
    /// End belt length.
    pub end_len: f64,
    /// End belt height.
    pub end_hgt: f64,
    /// Upper belt length.
    pub upper_len: f64,
    /// Upper belt height.
    pub upper_hgt: f64,
    /// Bulkhead length.
    pub bulkhead_len: f64,
    /// Bulkhead height.
    pub bulkhead_hgt: f64,
}

// default_belts {{{1
/// Estimated belt dimensions for the armor "Default" button.
///
/// Main belt runs the length between the forecastle and quarterdeck; the
/// heights derive from the beam, draft side and distributed freeboard.
///
pub fn default_belts(ship: &Ship) -> DefaultBelts {
    let lwl    = ship.hull.lwl().imp();
    let fc_len = ship.hull.freeboard.fc_len;
    let qd_len = ship.hull.freeboard.qd_len;
    let beam   = ship.hull.b.imp();
    let t      = ship.hull.t.imp();
    let cb     = ship.hull.cb();
    let dist   = ship.hull.freeboard.distributed();

    let t_side = ((1.006 - 0.0056 * cb.powf(-3.56)) * 2.0 - 1.0) * t;

    let main_len  = (1.0 - fc_len - qd_len) * lwl;
    let main_hgt  = (1.2 * beam.sqrt()).min(t_side + dist);
    let end_len   = lwl - main_len - 0.02;
    let upper_hgt = 8.0_f64.min(dist);
    let bulkhead_hgt = t_side;

    DefaultBelts {
        main_len,
        main_hgt,
        end_len,
        end_hgt:      main_hgt,
        upper_len:    main_len,
        upper_hgt,
        bulkhead_len: main_len,
        bulkhead_hgt,
    }
}

// Testing {{{1
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{
        Displacement,
        Length,
        Measurement,
        Ship,
        UnitType::LengthLong,
        Units,
    };

    fn ship() -> Ship {
        let mut s = Ship::default();
        s.hull.units = Units::Imperial;
        s.hull.disp = Displacement::Cb(0.6);
        s.hull.len = Length::Lwl(Measurement::new(400.0, LengthLong, Units::Imperial));
        s.hull.b = Measurement::new(40.0, LengthLong, Units::Imperial);
        s.hull.t = Measurement::new(25.0, LengthLong, Units::Imperial);
        s.hull.freeboard.fc_len = 0.2;
        s.hull.freeboard.fd_len = 0.3;
        s.hull.freeboard.qd_len = 0.3;
        s
    }

    #[test]
    fn default_belts_matches_reference() {
        let ship = ship();
        let d = default_belts(&ship);

        // Main belt length = (1 - 0.2 - 0.3) * 400 = 200
        assert!((d.main_len - 200.0).abs() < 1e-6);
        // Ends length = 400 - 200 - 0.02 = 199.98
        assert!((d.end_len - 199.98).abs() < 1e-6);
        assert!((d.upper_len - d.main_len).abs() < 1e-6);
        assert!((d.bulkhead_len - d.main_len).abs() < 1e-6);
        assert!((d.end_hgt - d.main_hgt).abs() < 1e-6);
    }
}
