// choice_enum! {{{1
/// Generate boilerplate for enums representing a list of user choices, e.g., BowType and GunType.
///
/// Expands to:
///     ALL: every variant, in .sship index order
///     label(), index(), from_index(): presentation-layer helpers
///     fmt::Display: report prose
///     From<&str>, From<String>: parse a decimal index string
///
/// The variants must be listed in SpringSharp (.sship) index order, which is not always
/// declaration order. Each row carries the menu label followed by an optional report
/// prose string; when the prose is omitted, fmt::Display falls back to the label.
/// Parsing accepts any non-negative decimal integer; unknown or out-of-range values
/// yield the default variant.
///
#[macro_export]
macro_rules! choice_enum {
    ($name:ident { $( $variant:ident $( ( $init:expr ) )? => ( $label:expr $(, $display:expr)? ) ),+ $(,)? }) => {
        #[allow(dead_code)]
        impl $name {
            /// All variants of this enum, in declaration order.
            ///
            /// The position of a variant in this slice is its stable index,
            /// used by [`Self::index`] and [`Self::from_index`] to convert
            /// between a variant and a `usize` (e.g. for UI selection state
            /// or serialized indices).
            pub const ALL: &'static [$name] = &[ $( $name::$variant $( ( $init ) )? ),+ ];

            /// Returns the menu label for every variant, in [`Self::ALL`] order.
            ///
            /// Use this to populate UI elements (e.g. dropdowns) with the
            /// full set of choices for this enum.
            pub fn all_labels() -> Vec<&'static str> {
                Self::ALL.iter().map(|v| v.label()).collect()
            }

            /// Returns the short "menu" label for this variant.
            ///
            /// This is the text shown in choice lists (e.g. dropdown
            /// options); see [`Self::all_labels`] to get every variant's
            /// label at once. Use [`fmt::Display`] instead for the longer
            /// form shown in reports.
            pub fn label(&self) -> &'static str {
                match self {
                    $( $name::$variant { .. } => $label ),+
                }
            }

            /// Returns this variant's position in [`Self::ALL`].
            ///
            /// Returns `0` if the variant is somehow not found (should not
            /// happen in practice, since `ALL` is generated from the same
            /// variant list as `Self`).
            pub fn index(&self) -> usize {
                Self::ALL
                    .iter()
                    .position(|v| std::mem::discriminant(v) == std::mem::discriminant(self))
                    .unwrap_or(0)
            }

            /// Returns the variant at `index` in [`Self::ALL`], falling back
            /// to the default variant if `index` is out of range.
            ///
            /// Inverse of [`Self::index`].
            pub fn from_index(index: usize) -> Self {
                Self::ALL.get(index).cloned().unwrap_or_default()
            }
        }

        /// Convert from string index in SpringSharp files.
        impl From<&str> for $name {
            fn from(index: &str) -> Self {
                index.parse::<usize>()
                    .ok()
                    .and_then(|i| Self::ALL.get(i).cloned())
                    .unwrap_or_default()
            }
        }

        /// Convert from string index in SpringSharp files.
        ///
        impl From<String> for $name {
            fn from(index: String) -> Self {
                index.as_str().into()
            }
        }

        /// Convert to string used in reports.
        ///
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}",
                    match self {
                        $( $name::$variant { .. } =>
                            choice_enum!(@label_or_display $label $(, $display)?) ),+
                    })
            }
        }
    };

    (@label_or_display $label:expr) => { $label };
    (@label_or_display $label:expr, $display:expr) => { $display };
}

// labels_enum! {{{1
/// Generate boilerplate for enums whose variants each carry a list of menu labels,
/// e.g., UnitType, where a variant maps to its imperial and metric unit names.
///
/// Unlike choice_enum!, each row carries one or more labels (e.g. ("in", "mm")) and the
/// generated labels() and ALL() methods return the whole list for a variant. Expands to
/// presentation-layer helpers on the variants (via &self), with no type-level list:
///     labels(), ALL(): the menu labels for a variant
///     from_index(): a single label for a variant by index
///
/// Out-of-range indices fall back to the first label.
///
#[macro_export]
macro_rules! labels_enum {
    ($name:ident { $( $variant:ident => ( $( $label:expr ),+ ) ),+ $(,)? }) => {
        #[allow(dead_code)]
        impl $name {
            /// Returns the label options associated with this variant.
            ///
            /// Each variant defines its own fixed set of labels (e.g. unit
            /// abbreviations); this is the internal accessor used by
            /// [`Self::all_labels`] and [`Self::from_index`].
            fn labels(&self) -> &'static [&'static str] {
                match self {
                    $( $name::$variant => &[ $( $label ),+ ] ),+
                }
            }

            /// Returns all label options for this variant as an owned `Vec`.
            ///
            /// Use this to populate UI elements (e.g. dropdowns) with the
            /// choices available for the currently selected variant.
            pub fn all_labels(&self) -> Vec<&'static str> {
                self.labels().to_vec()
            }

            /// Returns the label at `index` for this variant, falling back
            /// to the first label if `index` is out of range.
            ///
            /// `index` typically corresponds to a UI selection (e.g. the
            /// index chosen in a dropdown populated by [`Self::all_labels`]).
            #[allow(clippy::wrong_self_convention)]
            pub fn from_index(&self, index: usize) -> &'static str {
                self.labels().get(index).copied().unwrap_or(self.labels()[0])
            }
        }
    };
}

// num! {{{1
/// Format a number with commas and the specified number of significant digits,
/// 0 by default. If all digits that would be displayed after the decimal are 0,
/// only an integer is returned.
///
// This is a macro instead of a function to avoid having to cast
// floats to ints or ints to floats
#[macro_export]
macro_rules! num {
    ($val:expr) => { num!($val, 0) };
    ($val:expr, $digits: expr) => {{
        let s = format_num::format_num!(&*format!(",.{}", $digits), $val);
        if let Some(dot) = s.find('.') {
            if s[dot + 1..].chars().all(|c| c == '0') {
                s[..dot].to_string()
            } else {
                s
            }
        } else {
            s
        }
    }};
}

// pct! {{{1
/// Treat a number as a percent and format a number with commas and the specified number of
/// significant digits, 0 by default. A trailing '%' is deliberately ommitted.
///
// This is a macro instead of a function to avoid having to cast
// floats to ints or ints to floats
#[macro_export]
macro_rules! pct {
    ($val:expr)                => { pct!($val, 0) };
    ($val:expr, $digits: expr) => { $crate::num!($val as f64 * 100.0, $digits)
    };
}

// addto! {{{1
/// Pass arguments to format!() and push to a Vec<String>.
///
#[macro_export]
macro_rules! addto {
    ($r:ident,$($tts:tt)*) => {
        $r.push(format!($($tts)*))
    };
    ($r:ident) => {
        $r.push("".to_string())
    };
}

// addif! {{{1
/// Return a formatted string if the condition is true.
/// Otherwise return an empty string.
///
#[macro_export]
macro_rules! addif {
    ($cond:expr, $($tts:tt)*) => {
        if $cond {
            format!($($tts)*)
        } else {
            "".into()
        }
    }
}
