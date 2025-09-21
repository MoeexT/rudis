use crate::{command::error::CommandError, protocol::Frame};

#[derive(Debug)]
pub struct Parser {
    cmd: String,
    parts: Vec<Frame>,
    cursor: usize,
}

impl Parser {
    pub fn new(frame: Frame) -> Result<Self, CommandError> {
        match frame {
            Frame::Array(Some(parts)) if !parts.is_empty() => {
                let cmd = match &parts[0] {
                    Frame::SimpleString(s) => s.clone(),
                    Frame::BulkString(Some(data)) => String::from_utf8(data.clone())
                        .map_err(|e| CommandError::FromUtf8Error(e))?,
                    _ => return Err(CommandError::WrongType),
                };

                Ok(Self {
                    cmd: cmd.to_lowercase(),
                    parts,
                    cursor: 0,
                })
            }
            Frame::Array(Some(_)) => Err(CommandError::InvalidCommandFormat(
                "Empty command array".to_string(),
            )),
            _ => Err(CommandError::WrongType),
        }
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn next_pair<T>(&mut self) -> Result<(String, T), CommandError>
    where
        T: TryFrom<Frame, Error = CommandError>,
    {
        let key: String = self.next()?;
        let value: T = self.next()?;
        Ok((key, value))
    }
    
    pub fn next<T>(&mut self) -> Result<T, CommandError>
    where
        T: TryFrom<Frame, Error = CommandError>,
    {
        if !self.has_next() {
            return Err(CommandError::InvalidArgumentNumber(
                self.cmd.clone(),
                self.cursor,
            ));
        }
        let part = self.parts[self.cursor].clone();
        self.cursor += 1;
        part.try_into()
    }

    pub fn has_next(&self) -> bool {
        self.cursor < self.parts.len()
    }
}

impl TryFrom<Frame> for String {
    type Error = CommandError;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        match value {
            Frame::SimpleString(s) => Ok(s),
            Frame::BulkString(Some(bytes)) => {
                String::from_utf8(bytes).map_err(|e| CommandError::FromUtf8Error(e))
            }
            _ => Err(CommandError::WrongType),
        }
    }
}

impl TryFrom<Frame> for i64 {
    type Error = CommandError;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        match value {
            Frame::Integer(num) => Ok(num),
            Frame::BulkString(Some(bytes)) => {
                let s = String::from_utf8(bytes).map_err(|e| CommandError::FromUtf8Error(e))?;
                s.parse().map_err(|e| CommandError::ParseIntError(e))
            }
            _ => Err(CommandError::WrongType),
        }
    }
}

impl TryFrom<Frame> for Vec<u8> {
    type Error = CommandError;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        match value {
            Frame::BulkString(Some(bytes)) => Ok(bytes),
            _ => Err(CommandError::WrongType),
        }
    }
}

impl TryFrom<Frame> for bool {
    type Error = CommandError;

    fn try_from(value: Frame) -> Result<Self, Self::Error> {
        match value {
            Frame::Integer(0) => Ok(false),
            Frame::Integer(1) => Ok(true),
            Frame::SimpleString(s) if s.to_uppercase() == "TRUE" => Ok(true),
            Frame::SimpleString(s) if s.to_uppercase() == "FALSE" => Ok(false),
            _ => Err(CommandError::WrongType),
        }
    }
}
