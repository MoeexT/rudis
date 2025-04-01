use thiserror::Error;

#[derive(Error, Debug)]
pub enum RespError {
    #[error("Empty data")]
    EmptyData,

    #[error("Invalid RESP format")]
    InvalidFormat,

    #[error("Unsupported RESP type")]
    UnsupportedType,

    #[error("UTF-8 conversion error")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("String conversion error")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("Integer parsing error")]
    ParseIntError(#[from] std::num::ParseIntError),
}
