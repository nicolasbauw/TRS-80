use std::{error::Error, fmt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ConversionError {
    // String does not start with 0x
    InvalidAddressFormat,
    // Error while parsing int
    ParseIntError,
}

impl Error for ConversionError {}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConversionError : ")?;
        f.write_str(match self {
            ConversionError::InvalidAddressFormat => "String does not start with 0x",
            ConversionError::ParseIntError => "Error while parsing int",
        })
    }
}

impl From<std::num::ParseIntError> for ConversionError {
    fn from(_e: std::num::ParseIntError) -> ConversionError {
        ConversionError::ParseIntError
    }
}

pub trait HexStringToUnsigned {
    fn to_u16(&self) -> Result<u16, ConversionError>;
    fn to_u8(&self) -> Result<u8, ConversionError>;
}

impl HexStringToUnsigned for String {
    fn to_u16(&self) -> Result<u16, ConversionError> {
        match self.strip_prefix("0x") {
            Some(a) => {
                let r = u16::from_str_radix(a, 16)?;
                Ok(r)
            }
            None => Err(ConversionError::InvalidAddressFormat),
        }
    }
    fn to_u8(&self) -> Result<u8, ConversionError> {
        match self.strip_prefix("0x") {
            Some(a) => {
                let r = u8::from_str_radix(a, 16)?;
                Ok(r)
            }
            None => Err(ConversionError::InvalidAddressFormat),
        }
    }
}
