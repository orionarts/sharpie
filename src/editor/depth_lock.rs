//! Stateless depth-lock editing helpers.
//!
//! The depth lock pins the keel-to-deck height for each of the eight deck
//! corners so that a draft change is compensated by moving the freeboards in
//! the opposite direction. These functions work only on the domain [`Hull`];
//! UI formatting is the caller's (GUI layer) responsibility.

use crate::calc::hull::Hull;
use crate::calc::units::{Measurement, UnitType::LengthLong, Units};
use crate::calc::Freeboard;

// stash_depths {{{1
/// Capture the depth stash (freeboard + draft) for each of the eight deck
/// corners, as a [`Freeboard`] whose height fields hold the totals in imperial
/// feet (order fc_fwd, fc_aft, fd_fwd, fd_aft, ad_fwd, ad_aft, qd_fwd, qd_aft);
/// the length fractions are left at their defaults.
///
pub fn stash_depths(h: &Hull) -> Freeboard {
    let t = h.t.imp();
    let m = |v: f64| Measurement::new(v, LengthLong, Units::Imperial);

    Freeboard {
        fc_fwd: m(h.freeboard.fc_fwd.imp() + t),
        fc_aft: m(h.freeboard.fc_aft.imp() + t),
        fd_fwd: m(h.freeboard.fd_fwd.imp() + t),
        fd_aft: m(h.freeboard.fd_aft.imp() + t),
        ad_fwd: m(h.freeboard.ad_fwd.imp() + t),
        ad_aft: m(h.freeboard.ad_aft.imp() + t),
        qd_fwd: m(h.freeboard.qd_fwd.imp() + t),
        qd_aft: m(h.freeboard.qd_aft.imp() + t),
        ..Freeboard::default()
    }
}

// derive_freeboards {{{1
/// Re-derive the freeboard boxes from the depth stashes after a draft change.
///
/// Each freeboard becomes (stash - draft), so the keel-to-deck height holds
/// steady. The hull freeboards are updated in place, and the resulting
/// freeboard (length fractions preserved) is returned.
///
pub fn derive_freeboards(h: &mut Hull, depths: &Freeboard) -> Freeboard {
    let t = h.t.imp();
    let m = |v: f64| Measurement::new(v, LengthLong, Units::Imperial);

    h.freeboard.fc_fwd = m(depths.fc_fwd.imp() - t);
    h.freeboard.fc_aft = m(depths.fc_aft.imp() - t);
    h.freeboard.fd_fwd = m(depths.fd_fwd.imp() - t);
    h.freeboard.fd_aft = m(depths.fd_aft.imp() - t);
    h.freeboard.ad_fwd = m(depths.ad_fwd.imp() - t);
    h.freeboard.ad_aft = m(depths.ad_aft.imp() - t);
    h.freeboard.qd_fwd = m(depths.qd_fwd.imp() - t);
    h.freeboard.qd_aft = m(depths.qd_aft.imp() - t);
    h.freeboard.clone()
}
