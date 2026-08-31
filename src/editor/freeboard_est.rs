//! Freeboard estimation for the "Flush deck" / "Mid break" buttons.
//!
//! These are stateless helpers ported from SpringSharp 3b3's
//! `btnFlushClick`/`btnBreakClick` (SpringSharp3b3.cs lines 1741 and 1770).

use crate::calc::{
    Freeboard,
    Measurement,
    Ship,
    UnitType::LengthLong,
    Units,
};

// freeboard_est {{{1
/// Estimated freeboards for the "Flush deck" / "Mid break" buttons.
///
/// `which` selects the preset (0 = flush deck, anything else = mid break). The
/// height fields of the returned [`Freeboard`] are in imperial feet; the length
/// fractions are set to 0.
///
pub fn freeboard_est(lwl: Measurement, year: u32, which: i32) -> Freeboard {
    let m = |v: f64| Measurement::new(v, LengthLong, Units::Imperial);
    let lwl = lwl.imp();
    let bow = (1.1 - (1.0 - Ship::year_adj(year)) * 0.5) * lwl.sqrt();

    let (fc_aft, fd_fwd, fd_aft, ad_fwd, ad_aft, qd_fwd, qd_aft) = match which {
        0 => {
            let other = 0.7 * lwl.sqrt();
            let mid = (bow + other) / 2.0;
            (mid, mid, other, other, other, other, other)
        }
        _ => {
            let other = 0.9 * lwl.sqrt();
            let aft = other / 2.0;
            (other, other, other, aft, aft, aft, aft)
        }
    };

    Freeboard {
        fc_len: 0.0,
        fc_fwd: m(bow),
        fc_aft: m(fc_aft),
        fd_len: 0.0,
        fd_fwd: m(fd_fwd),
        fd_aft: m(fd_aft),
        ad_fwd: m(ad_fwd),
        ad_aft: m(ad_aft),
        qd_len: 0.0,
        qd_fwd: m(qd_fwd),
        qd_aft: m(qd_aft),
    }
}

// apply_freeboard_est {{{1
/// Build a [`Freeboard`] carrying the height estimates from "Flush deck"/"Mid
/// break".
///
/// Mirrors SpringSharp's `btnFlushClick`/`btnBreakClick`. This is a pure
/// constructor only; the caller is responsible for writing the returned
/// heights back into the ship so the display, image and report stay in sync.
///
pub fn apply_freeboard_est(est: &Freeboard) -> Freeboard {
    Freeboard {
        fc_fwd: est.fc_fwd,
        fc_aft: est.fc_aft,
        fd_fwd: est.fd_fwd,
        fd_aft: est.fd_aft,
        ad_fwd: est.ad_fwd,
        ad_aft: est.ad_aft,
        qd_fwd: est.qd_fwd,
        qd_aft: est.qd_aft,
        ..Freeboard::default()
    }
}

// Testing {{{1
#[cfg(test)]
mod tests {
    use super::*;

    // close {{{2
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    // Test flush deck estimate {{{2
    #[test]
    fn est_flush_deck() {
        // LWL 400 ft, year 1890 (dateAdj 1.0) -> bow 22 ft, elsewhere 14 ft.
        let lwl = Measurement::new(400.0, LengthLong, Units::Imperial);
        let est = freeboard_est(lwl, 1890, 0);
        assert!(close(est.fc_fwd.imp(), 22.0)); // fc_fwd
        assert!(close(est.fc_aft.imp(), 18.0)); // fc_aft
        assert!(close(est.fd_fwd.imp(), 18.0)); // fd_fwd
        assert!(close(est.fd_aft.imp(), 14.0)); // fd_aft
        assert!(close(est.qd_aft.imp(), 14.0)); // qd_aft
    }

    // Test mid break estimate {{{2
    #[test]
    fn est_mid_break() {
        // Other heights at 0.9 * sqrt(400) = 18 ft, aft of the break 9 ft.
        let lwl = Measurement::new(400.0, LengthLong, Units::Imperial);
        let est = freeboard_est(lwl, 1890, 1);
        assert!(close(est.fc_fwd.imp(), 22.0)); // fc_fwd
        assert!(close(est.fc_aft.imp(), 18.0)); // fc_aft
        assert!(close(est.fd_aft.imp(), 18.0)); // fd_aft
        assert!(close(est.ad_fwd.imp(), 9.0));  // ad_fwd
        assert!(close(est.qd_aft.imp(), 9.0));  // qd_aft
    }

    // Test estimate bow for an old ship {{{2
    #[test]
    fn est_old_ship_bow() {
        // year 1850 -> dateAdj 0.4 -> bow factor 0.8.
        let lwl = Measurement::new(400.0, LengthLong, Units::Imperial);
        let est = freeboard_est(lwl, 1850, 0);
        assert!(close(est.fc_fwd.imp(), 16.0));
    }
}
