use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    /// Abstract and universal error
    #[error("Invalid command: `{0}`")]
    InvalidCommand(String),

    #[error("Invalid format for `{0}` command")]
    InvalidCommandFormat(String),

    #[error("Wrong number of arguments for `{0}` command, expect {1} value(s)")]
    InvalidArgumentNumber(String, usize),
    
    #[error("Wrong format for `{0}` argument")]
    InvalidArgumentFormat(String),
    
    #[error("Operation against a key holding the wrong kind of value")]
    WrongType,

    #[error("Super huge value(length: {0}) for `{1}` command")]
    SuperHugeString(usize, String),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("From UTF-8 error: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("Value is not an integer or out of range")]
    ParseIntError(#[from] core::num::ParseIntError),
}
