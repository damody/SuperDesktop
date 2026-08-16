use std::{error::Error, fmt};

pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_COLLECTION_ITEMS: usize = 1_024;
pub const MAX_ICON_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    Empty(&'static str),
    TextTooLong(&'static str),
    CollectionTooLarge(&'static str),
    OutOfRange(&'static str),
    InvalidValue(&'static str),
    FrameTooLarge { actual: usize, maximum: usize },
    UnsupportedProtocol { major: u16 },
    Expired,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(formatter, "{field} must not be empty"),
            Self::TextTooLong(field) => write!(formatter, "{field} exceeds the text limit"),
            Self::CollectionTooLarge(field) => write!(formatter, "{field} exceeds the item limit"),
            Self::OutOfRange(field) => write!(formatter, "{field} is out of range"),
            Self::InvalidValue(field) => write!(formatter, "{field} is invalid"),
            Self::FrameTooLarge { actual, maximum } => {
                write!(formatter, "frame is {actual} bytes; maximum is {maximum}")
            }
            Self::UnsupportedProtocol { major } => {
                write!(formatter, "protocol major {major} is unsupported")
            }
            Self::Expired => formatter.write_str("request deadline has expired"),
        }
    }
}

impl Error for ValidationError {}

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

pub fn validate_frame_size(bytes: &[u8]) -> Result<(), ValidationError> {
    if bytes.len() > MAX_FRAME_BYTES {
        Err(ValidationError::FrameTooLarge {
            actual: bytes.len(),
            maximum: MAX_FRAME_BYTES,
        })
    } else {
        Ok(())
    }
}
