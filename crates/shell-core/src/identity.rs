use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityError {
    Empty,
    ContainsNul,
    TooLong,
    InvalidInteger,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "identity is empty",
            Self::ContainsNul => "identity contains NUL",
            Self::TooLong => "identity exceeds 512 bytes",
            Self::InvalidInteger => "identity integer is invalid",
        })
    }
}

impl std::error::Error for IdentityError {}

fn validate(value: &str) -> Result<(), IdentityError> {
    if value.is_empty() {
        Err(IdentityError::Empty)
    } else if value.contains('\0') {
        Err(IdentityError::ContainsNul)
    } else if value.len() > 512 {
        Err(IdentityError::TooLong)
    } else {
        Ok(())
    }
}

macro_rules! string_identity {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdentityError> {
                let value = value.into();
                validate(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }
    };
}

string_identity!(MonitorId);
string_identity!(ShellItemId);
string_identity!(WindowId);
string_identity!(ApplicationId);

macro_rules! integer_identity {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub $inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value
                    .parse::<$inner>()
                    .map(Self)
                    .map_err(|_| IdentityError::InvalidInteger)
            }
        }
    };
}

integer_identity!(RequestId, u64);
integer_identity!(Generation, u64);
integer_identity!(CorrelationId, u128);

impl Generation {
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_round_trip_and_sort_stably() {
        let mut monitors = [
            MonitorId::new("display-b").unwrap(),
            MonitorId::new("display-a").unwrap(),
        ];
        monitors.sort();
        assert_eq!(monitors[0].to_string(), "display-a");
        assert_eq!(monitors[0].to_string().parse(), Ok(monitors[0].clone()));
        assert_eq!("42".parse(), Ok(RequestId(42)));
        assert_eq!("43".parse(), Ok(Generation(43)));
        assert_eq!("44".parse(), Ok(CorrelationId(44)));
    }

    #[test]
    fn identities_reject_ambiguous_or_unbounded_values() {
        assert_eq!(MonitorId::new(""), Err(IdentityError::Empty));
        assert_eq!(ShellItemId::new("a\0b"), Err(IdentityError::ContainsNul));
        assert_eq!(WindowId::new("x".repeat(513)), Err(IdentityError::TooLong));
        assert_ne!(
            ApplicationId::new("a").unwrap(),
            ApplicationId::new("b").unwrap()
        );
    }
}
