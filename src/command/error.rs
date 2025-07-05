use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Invalid command name: {0}")]
    InvalidCommand(String),

    #[error("Invalid arguments for command {0}: {1:?}")]
    InvalidArguments(String, Vec<String>),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}
