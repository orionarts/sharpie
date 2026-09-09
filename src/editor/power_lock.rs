//! Stateless power-lock editing helpers.
//!
//! The power lock pins the installed horsepower ([`Engine::power_lock`]) so
//! the machinery weight and engine readouts stay constant while the hull is
//! worked. Recalc then solves the maximum speed (and range) at which the
//! current hull requires exactly the locked power (and bunkerage), mirroring
//! SpringSharp's "Lock Power" / "Recalc" buttons. These functions work only
//! on the domain [`Ship`]; UI formatting is the caller's (GUI layer)
//! responsibility.

use crate::calc::Ship;

// solve_vmax {{{1
/// Maximum speed whose required horsepower is `target_hp`.
///
/// [`Engine::hp`] is strictly increasing in speed (the v⁴ and v¹˒⁸³ terms
/// grow while the length factor shrinks), so bisection converges exactly,
/// unlike SpringSharp's diminishing fixed-step loop.
///
pub fn solve_vmax(ship: &Ship, target_hp: f64) -> f64 {
    let (d, lwl, leff, cs, ws) = ship_params(ship);

    if target_hp <= 0.0 { return 0.0; }
    if ship.engine.hp(MAX_SPEED, d, lwl, leff, cs, ws) <= target_hp { return MAX_SPEED; }

    let mut lo = 0.0;
    let mut hi = MAX_SPEED;

    for _ in 0..ITERATIONS {
        let mid = (lo + hi) / 2.0;
        if ship.engine.hp(mid, d, lwl, leff, cs, ws) < target_hp {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    (lo + hi) / 2.0
}

// solve_range {{{1
/// Range in nautical miles whose bunkerage weight is `target_bunker`.
///
/// Bunkerage is linear in range ([`Engine::bunker_for_range`]), so bisection
/// over the whole `u32` range converges quickly.
///
pub fn solve_range(ship: &Ship, target_bunker: f64) -> u32 {
    let (d, lwl, leff, cs, ws) = ship_params(ship);

    if target_bunker <= 0.0 { return 0; }
    if ship.engine.bunker_for_range(f64::from(u32::MAX), d, lwl, leff, cs, ws) <= target_bunker {
        return u32::MAX;
    }

    let mut lo = 0.0;
    let mut hi = f64::from(u32::MAX);

    for _ in 0..ITERATIONS {
        let mid = (lo + hi) / 2.0;
        if ship.engine.bunker_for_range(mid, d, lwl, leff, cs, ws) < target_bunker {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    ((lo + hi) / 2.0).round() as u32
}

const MAX_SPEED: f64 = 50.0;
const ITERATIONS: usize = 48;

// ship_params {{{1
/// The hull parameters that the engine power equations are evaluated with.
///
fn ship_params(ship: &Ship) -> (f64, f64, f64, f64, f64) {
    (
        ship.hull.d(),
        ship.hull.lwl().imp(),
        ship.hull.leff(),
        ship.hull.cs(),
        ship.hull.ws(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::calc::{
        BoilerType,
        Displacement,
        Length,
        Measurement,
        UnitType::LengthLong,
        Units,
    };

    fn ship() -> Ship {
        let mut s = Ship::default();
        s.hull.units = Units::Imperial;
        s.hull.disp = Displacement::Cb(0.6);
        s.hull.len = Length::Lwl(Measurement::new(400.0, LengthLong, Units::Imperial));
        s.hull.b = Measurement::new(40.0, LengthLong, Units::Imperial);
        s.hull.bb = Measurement::new(40.0, LengthLong, Units::Imperial);
        s.hull.t = Measurement::new(25.0, LengthLong, Units::Imperial);
        s.engine.year = 1900;
        s.engine.vmax = 20.0;
        s.engine.vcruise = 12.0;
        s.engine.range = 5000;
        s.engine.pct_coal = 0.0;
        s.engine.boiler.insert(BoilerType::Turbine);
        s
    }

    #[test]
    fn solve_vmax_recovers_target_power() {
        let ship = ship();
        let (d, lwl, leff, cs, ws) = ship_params(&ship);
        let target = ship.engine.hp(20.0, d, lwl, leff, cs, ws);
        assert!(target > 0.0);

        let v = solve_vmax(&ship, target);
        let got = ship.engine.hp(v, d, lwl, leff, cs, ws);
        assert!((got - target).abs() / target < 1e-6);
    }

    #[test]
    fn solve_vmax_clamps_at_max_speed() {
        let ship = ship();
        let (d, lwl, leff, cs, ws) = ship_params(&ship);
        let limit = ship.engine.hp(50.0, d, lwl, leff, cs, ws);
        assert_eq!(solve_vmax(&ship, limit * 1.5), MAX_SPEED);
    }

    #[test]
    fn solve_range_recovers_bunker_target() {
        let ship = ship();
        let (d, lwl, leff, cs, ws) = ship_params(&ship);
        let target = ship.engine.bunker_for_range(5000.0, d, lwl, leff, cs, ws);
        assert!(target > 0.0);

        let r = solve_range(&ship, target);
        let got = ship.engine.bunker_for_range(f64::from(r), d, lwl, leff, cs, ws);
        assert!((got - target).abs() / target < 1e-3);
        assert!((f64::from(r) - 5000.0).abs() < 50.0);
    }

    #[test]
    fn power_lock_overrides_hp_max() {
        let mut ship = ship();
        let (d, lwl, leff, cs, ws) = ship_params(&ship);
        let normal = ship.engine.hp_max(d, lwl, leff, cs, ws);

        ship.engine.power_lock = Some(43210.0);
        assert_eq!(ship.engine.hp_max(d, lwl, leff, cs, ws), 43210.0);

        ship.engine.power_lock = None;
        assert_eq!(ship.engine.hp_max(d, lwl, leff, cs, ws), normal);
    }
}