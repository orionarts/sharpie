use crate::calc::{Measurement, Units, UnitType::LengthLong};

use serde::{Deserialize, Serialize};

// Freeboard {{{1
/// Freeboard of the ship's deck, split into four sections (forecastle,
/// foredeck, afterdeck and quarterdeck). Each section is described by its
/// height forward and aft, and its length as a fraction of the total deck.
///
/// The afterdeck length is not stored directly; instead it is derived from
/// the forecastle, fore and quarter deck lengths (see [`Freeboard::ad_len`]).
///
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Freeboard {
    /// Forecastle length as a fraction of the total deck.
    pub fc_len: f64,
    /// Height of forecastle forward.
    pub fc_fwd: Measurement,
    /// Height of forecastle aft.
    pub fc_aft: Measurement,

    /// Foredeck length as a fraction of the total deck.
    pub fd_len: f64,
    /// Height of foredeck forward.
    pub fd_fwd: Measurement,
    /// Height of foredeck aft.
    pub fd_aft: Measurement,

    /// Height of aftdeck forward.
    pub ad_fwd: Measurement,
    /// Height of aftdeck aft.
    pub ad_aft: Measurement,

    /// Quarterdeck length as a fraction of the total deck.
    pub qd_len: f64,
    /// Height of quarterdeck forward.
    pub qd_fwd: Measurement,
    /// Height of quarterdeck aft.
    pub qd_aft: Measurement,
}

impl Default for Freeboard { // {{{2
    fn default() -> Self {
        Freeboard {
            fc_len: 0.2,
            fc_fwd: Measurement::new(0.0, LengthLong, Units::Imperial),
            fc_aft: Measurement::new(0.0, LengthLong, Units::Imperial),

            fd_len: 0.3,
            fd_fwd: Measurement::new(0.0, LengthLong, Units::Imperial),
            fd_aft: Measurement::new(0.0, LengthLong, Units::Imperial),

            ad_fwd: Measurement::new(0.0, LengthLong, Units::Imperial),
            ad_aft: Measurement::new(0.0, LengthLong, Units::Imperial),

            qd_len: 0.15,
            qd_fwd: Measurement::new(0.0, LengthLong, Units::Imperial),
            qd_aft: Measurement::new(0.0, LengthLong, Units::Imperial),
        }
    }
}

impl Freeboard { // {{{2
    // ad_len {{{3
    /// Length of the after deck as a fraction of the total
    /// deck based on forecastle, fore and aft decks.
    ///
    pub fn ad_len(&self) -> f64 {
        1.0 - self.fc_len - self.fd_len - self.qd_len
    }

    // fc {{{3
    /// Average forecastle height (weighted to slope up toward the bow).
    ///
    pub fn fc(&self) -> f64 {
        self.fc_aft.imp() + (self.fc_fwd.imp() - self.fc_aft.imp()) * 0.4
    }

    // fd {{{3
    /// Average foredeck height.
    ///
    pub fn fd(&self) -> f64 {
        self.fd_fwd.imp() + (self.fd_aft.imp() - self.fd_fwd.imp()) * 0.5
    }

    // ad {{{3
    /// Average afterdeck height.
    ///
    pub fn ad(&self) -> f64 {
        self.ad_fwd.imp() + (self.ad_aft.imp() - self.ad_fwd.imp()) * 0.5
    }

    // qd {{{3
    /// Average quarterdeck height.
    ///
    pub fn qd(&self) -> f64 {
        self.qd_fwd.imp() + (self.qd_aft.imp() - self.qd_fwd.imp()) * 0.5
    }

    // average {{{3
    /// Average freeboard.
    ///
    pub fn average(&self) -> Measurement {
        Measurement::new(
            self.fc() * self.fc_len +
            self.fd() * self.fd_len +
            self.ad() * self.ad_len() +
            self.qd() * self.qd_len,
            LengthLong, Units::Imperial
        )
    }

    // distributed {{{3
    /// Mean freeboard over the fore and aft decks ("distributed" freeboard)
    ///
    pub fn distributed(&self) -> f64 {
        if self.fd_len + self.ad_len() == 0.0 { return 0.0; }
        (self.fd() * self.fd_len + self.ad() * self.ad_len()) / (self.fd_len + self.ad_len())
    }
}

// Testing {{{1
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::test_support::*;

    // ad_len {{{3
    macro_rules! test_ad_len {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, fc_len) = $value;

                    let mut fb = Freeboard::default();
                    fb.fc_len = fc_len;
                    fb.fd_len = 0.25;
                    fb.qd_len = 0.25;

                    assert_eq!(expected, to_place(fb.ad_len(), 2));
                }
            )*
        }
    }
    test_ad_len! {
        // name: (ad_len, fc_len)
        ad_len_zero: (0.0, 0.5),
        ad_len_test: (0.25, 0.25),
    }

    // average {{{3
    macro_rules! test_average {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, fc_len) = $value;

                    let mut fb = Freeboard::default();

                    fb.fc_len = fc_len;
                    fb.fc_fwd = Measurement::new(10.0, LengthLong, Units::Imperial);
                    fb.fc_aft = Measurement::new(10.0, LengthLong, Units::Imperial);

                    fb.fd_len = (1.0 - fc_len) * 0.4;
                    fb.fd_fwd = Measurement::new(fb.fc_fwd.imp() + 5.0, LengthLong, Units::Imperial);
                    fb.fd_aft = fb.fc_fwd;

                    fb.ad_fwd = Measurement::new(fb.fc_fwd.imp() + 10.0, LengthLong, Units::Imperial);
                    fb.ad_aft = fb.fc_fwd;

                    fb.qd_len = (1.0 - fc_len) * 0.4;
                    fb.qd_fwd = Measurement::new(fb.fc_fwd.imp() - 5.0, LengthLong, Units::Imperial);
                    fb.qd_aft = fb.fc_fwd;

                    assert_eq!(expected, to_place(fb.average().imp(), 3));
                }
            )*
        }
    }
    test_average! {
        // name: (average, fc_len)
        average_test: (10.75, 0.25),
    }

    // distributed {{{3
    macro_rules! test_distributed {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, fc_len) = $value;

                    let mut fb = Freeboard::default();

                    fb.fc_len = fc_len;
                    fb.fc_fwd = Measurement::new(10.0, LengthLong, Units::Imperial);
                    fb.fc_aft = Measurement::new(10.0, LengthLong, Units::Imperial);

                    fb.fd_len = (1.0 - fc_len) * 0.4;
                    fb.fd_fwd = Measurement::new(fb.fc_fwd.imp() + 5.0, LengthLong, Units::Imperial);
                    fb.fd_aft = fb.fc_fwd;

                    fb.ad_fwd = Measurement::new(fb.fc_fwd.imp() + 10.0, LengthLong, Units::Imperial);
                    fb.ad_aft = fb.fc_fwd;

                    fb.qd_len = (1.0 - fc_len) * 0.4;
                    fb.qd_fwd = Measurement::new(fb.fc_fwd.imp() - 5.0, LengthLong, Units::Imperial);
                    fb.qd_aft = fb.fc_fwd;

                    assert_eq!(expected, to_place(fb.distributed(), 2));
                }
            )*
        }
    }
    test_distributed! {
        // name:         (dist, fc_len)
        distributed_test: (13.33, 10.0),
    }

    // fc {{{3
    macro_rules! test_fc {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, fc_fwd, fc_aft) = $value;

                    let mut fb = Freeboard::default();

                    fb.fc_fwd = Measurement::new(fc_fwd, LengthLong, Units::Imperial);
                    fb.fc_aft = Measurement::new(fc_aft, LengthLong, Units::Imperial);

                    assert_eq!(expected, fb.fc());
                }
            )*
        }
    }

    test_fc! {
        // name:         (fc, fc_fwd, fc_aft)
        fc_test_eq:        (10.0, 10.0, 10.0),
        fc_test_slope_fwd: (4.0, 10.0, 0.0),
        fc_test_slope_aft: (6.0, 0.0, 10.0),
    }

    // fd {{{3
    macro_rules! test_fd {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, fd_fwd, fd_aft) = $value;

                    let mut fb = Freeboard::default();

                    fb.fd_fwd = Measurement::new(fd_fwd, LengthLong, Units::Imperial);
                    fb.fd_aft = Measurement::new(fd_aft, LengthLong, Units::Imperial);

                    assert_eq!(expected, fb.fd());
                }
            )*
        }
    }

    test_fd! {
        // name:         (fd, fd_fwd, fd_aft)
        fd_test_eq:        (10.0, 10.0, 10.0),
        fd_test_slope_fwd: (5.0, 10.0, 0.0),
        fd_test_slope_aft: (5.0, 0.0, 10.0),
    }

    // ad {{{3
    macro_rules! test_ad {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, ad_fwd, ad_aft) = $value;

                    let mut fb = Freeboard::default();
                    fb.ad_fwd = Measurement::new(ad_fwd, LengthLong, Units::Imperial);
                    fb.ad_aft = Measurement::new(ad_aft, LengthLong, Units::Imperial);

                    assert_eq!(expected, fb.ad());
                }
            )*
        }
    }

    test_ad! {
        // name:         (ad, ad_fwd, ad_aft)
        ad_test_eq:        (10.0, 10.0, 10.0),
        ad_test_slope_fwd: (5.0, 10.0, 0.0),
        ad_test_slope_aft: (5.0, 0.0, 10.0),
    }

    // qd {{{3
    macro_rules! test_qd {
        ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (expected, qd_fwd, qd_aft) = $value;

                    let mut fb = Freeboard::default();

                    fb.qd_fwd = Measurement::new(qd_fwd, LengthLong, Units::Imperial);
                    fb.qd_aft = Measurement::new(qd_aft, LengthLong, Units::Imperial);

                    assert_eq!(expected, fb.qd());
                }
            )*
        }
    }

    test_qd! {
        // name:         (qd, qd_fwd, qd_aft)
        qd_test_eq:        (10.0, 10.0, 10.0),
        qd_test_slope_fwd: (5.0, 10.0, 0.0),
        qd_test_slope_aft: (5.0, 0.0, 10.0),
    }
}
