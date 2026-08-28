//! Freeboard estimation for the "Flush deck" / "Mid break" buttons.
//!
//! These are stateless helpers ported from SpringSharp 3b3's
//! `btnFlushClick`/`btnBreakClick` (SpringSharp3b3.cs lines 1741 and 1770).

use crate::calc::hull::Hull;
use crate::calc::units::{Measurement, UnitType::LengthLong, Units};
use crate::calc::Freeboard;

// freeboard_est {{{1
/// Estimated freeboards for the "Flush deck" / "Mid break" buttons.
///
/// `lwl_ft` is the waterline length in feet, `adj` is the date adjustment
/// (`Ship::year_adj`), and `which` selects the preset (0 = flush deck, anything
/// else = mid break). The height fields of the returned [`Freeboard`] are in
/// imperial feet; the length fractions are left at their defaults.
///
pub fn freeboard_est(lwl_ft: f64, adj: f64, which: i32) -> Freeboard {
    let m = |v: f64| Measurement::new(v, LengthLong, Units::Imperial);
    let bow = (1.1 - (1.0 - adj) * 0.5) * lwl_ft.sqrt();

    let (fc_aft, fd_fwd, fd_aft, ad_fwd, ad_aft, qd_fwd, qd_aft) = match which {
        0 => {
            let other = 0.7 * lwl_ft.sqrt();
            let mid = (bow + other) / 2.0;
            (mid, mid, other, other, other, other, other)
        }
        _ => {
            let other = 0.9 * lwl_ft.sqrt();
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
/// Fill the freeboard height fields from the "Flush deck"/"Mid break" estimates.
///
/// Mirrors SpringSharp's `btnFlushClick`/`btnBreakClick`, writing the heights
/// back into the ship so the display, image and report stay in sync. The length
/// fractions are left untouched and carried through on the returned freeboard.
///
pub fn apply_freeboard_est(h: &mut Hull, est: &Freeboard) -> Freeboard {
    h.freeboard.fc_fwd = est.fc_fwd;
    h.freeboard.fc_aft = est.fc_aft;
    h.freeboard.fd_fwd = est.fd_fwd;
    h.freeboard.fd_aft = est.fd_aft;
    h.freeboard.ad_fwd = est.ad_fwd;
    h.freeboard.ad_aft = est.ad_aft;
    h.freeboard.qd_fwd = est.qd_fwd;
    h.freeboard.qd_aft = est.qd_aft;
    h.freeboard.clone()
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
        // LWL 400 ft, dateAdj 1.0 -> bow 22 ft, elsewhere 14 ft.
        let est = freeboard_est(400.0, 1.0, 0);
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
        let est = freeboard_est(400.0, 1.0, 1);
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
        let est = freeboard_est(400.0, 0.4, 0);
        assert!(close(est.fc_fwd.imp(), 16.0));
    }
}
