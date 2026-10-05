//! Newtypes for the protocol's numeric codes and bit fields. Both keep
//! every value, named or not, so a frame decodes and encodes back
//! unchanged even when it carries values from a later standard.

/// A code such as a message type or status: a newtype over the raw value
/// with a constant for each value the standard names.
macro_rules! code {
    (
        $(#[$meta:meta])*
        pub struct $name:ident($ty:ty) {
            $( $(#[$value_meta:meta])* const $value:ident = $raw:literal; )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $name(pub $ty);

        impl $name {
            $( $(#[$value_meta])* pub const $value: $name = $name($raw); )*

            /// The standard's name for the value, or `None` for a reserved
            /// one.
            pub const fn name(self) -> Option<&'static str> {
                match self.0 {
                    $( $raw => Some(stringify!($value)), )*
                    _ => None,
                }
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self.name() {
                    Some(name) => formatter.write_str(name),
                    None => write!(formatter, "{}({})", stringify!($name), self.0),
                }
            }
        }
    };
}

/// A bit field: a newtype over the raw value with a constant for each bit
/// the standard names, and set operations.
macro_rules! flags {
    (
        $(#[$meta:meta])*
        pub struct $name:ident($ty:ty) {
            $( $(#[$flag_meta:meta])* const $flag:ident = $raw:literal; )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub $ty);

        impl $name {
            $( $(#[$flag_meta])* pub const $flag: $name = $name($raw); )*

            const NAMED: &'static [(&'static str, $ty)] = &[ $( (stringify!($flag), $raw), )* ];

            pub const fn empty() -> Self {
                Self(0)
            }

            pub const fn bits(self) -> $ty {
                self.0
            }

            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }

            /// Every bit of `other` is set.
            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            pub const fn union(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }

            pub const fn difference(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }

            /// The standard's names of the bits set, in bit order; bits it
            /// does not name are left out.
            pub fn names(self) -> impl Iterator<Item = &'static str> {
                Self::NAMED
                    .iter()
                    .filter(move |(_, bit)| self.0 & bit != 0)
                    .map(|(name, _)| *name)
            }

            pub fn set(&mut self, other: Self, on: bool) {
                if on {
                    self.0 |= other.0;
                } else {
                    self.0 &= !other.0;
                }
            }
        }

        impl core::ops::BitOr for $name {
            type Output = Self;

            fn bitor(self, other: Self) -> Self {
                self.union(other)
            }
        }

        impl core::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0;
            }
        }

        impl core::ops::BitAnd for $name {
            type Output = Self;

            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }

        impl core::fmt::Debug for $name {
            /// The named bits, then any others in hex.
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(formatter, "{}(", stringify!($name))?;
                let mut rest = self.0;
                let mut first = true;
                for (name, bit) in Self::NAMED {
                    if self.0 & bit != 0 {
                        if !first {
                            formatter.write_str(" | ")?;
                        }
                        formatter.write_str(name)?;
                        rest &= !bit;
                        first = false;
                    }
                }
                if rest != 0 {
                    if !first {
                        formatter.write_str(" | ")?;
                    }
                    write!(formatter, "{:#x}", rest)?;
                }
                formatter.write_str(")")
            }
        }
    };
}

pub(crate) use {code, flags};
