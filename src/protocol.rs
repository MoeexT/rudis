use std::fmt::Display;

use async_recursion::async_recursion;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

use crate::config::get_server_config;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Frame {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<Frame>>),
    Null,
    Boolean(bool),
    Exit,
}

#[derive(Error, Debug)]
pub enum FrameError {
    #[error("Invalid Frame format: {message}")]
    InvalidFormat {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Unsupported Frame type: {0}")]
    UnsupportedType(char),

    #[error("Incomplete data: expected {expected} bytes, got {actual} bytes")]
    IncompleteData { expected: usize, actual: usize },

    #[error("Unexpected Frame type: expected {expected}, got {actual}")]
    UnexpectedType {
        expected: String,
        actual: &'static str,
    },

    #[error("Null value not allowed")]
    NullValue,

    #[error("Empty Frame not allowed")]
    EmptyArray,

    #[error("Array length mismatch: expected {expected}, got {actual}")]
    ArrayLengthMismatch { expected: usize, actual: usize },

    #[error("UTF-8 conversion error")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("String conversion error")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error("Value is not an integer or out of range")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Value too long: {length} bytes (max: {max})")]
    ValueTooLong { length: usize, max: usize },

    #[error("Invalid boolean value: {0}")]
    InvalidBoolean(String),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("Command {0} execute error")]
    CommandError(String, #[source] crate::command::error::CommandError),
}

#[async_recursion]
pub async fn parse<R>(reader: &mut BufReader<R>) -> Result<Frame, FrameError>
where
    R: AsyncReadExt + Unpin + Send,
{
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.is_empty() {
        return Err(FrameError::InvalidFormat {
            message: "Empty input".to_string(),
            source: None,
        });
    }

    if line.len() > get_server_config().string_max_length as usize {
        return Err(FrameError::ValueTooLong {
            length: line.len(),
            max: get_server_config().string_max_length as usize,
        });
    }

    let prefix = line.chars().next().ok_or(FrameError::InvalidFormat {
            message: "Empty input".to_string(),
            source: None,
        })?;
    let content = &line[1..].trim_end();

    match prefix {
        '+' => Ok(Frame::SimpleString(content.to_string())),
        '-' => Ok(Frame::Error(content.to_string())),
        ':' => Ok(Frame::Integer(content.parse()?)),
        '$' => {
            // get string length
            let len: isize = content.parse()?;
            if len == -1 {
                return Ok(Frame::BulkString(None));
            }
            let mut buf = vec![0; len as usize];
            reader.read_exact(&mut buf).await?;
            // Read the trailing \r\n
            let mut crlf = [0; 2];
            reader.read_exact(&mut crlf).await?;
            Ok(Frame::BulkString(Some(buf)))
        }
        '*' => {
            let count: isize = content.parse()?;
            if count == -1 {
                return Ok(Frame::Array(None));
            }
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let val = parse(reader).await?;
                items.push(val);
            }
            Ok(Frame::Array(Some(items)))
        }
        _ => Err(FrameError::UnsupportedType(prefix)),
    }
}

impl<'a> Frame {
    /// Turns the `Frame` to bytes then write them to `BufWriter`,
    /// usually `BufWriter` comes from `TcpStream`
    ///
    /// ```
    ///
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the error that `writer.write_all` returns
    ///
    #[async_recursion]
    pub async fn write_to<W>(self, writer: &mut BufWriter<W>) -> Result<(), FrameError>
    where
        W: AsyncWriteExt + Unpin + Send,
    {
        match self {
            Frame::SimpleString(ss) => {
                writer.write_all(format!("+{}\r\n", ss).as_bytes()).await?
            }
            Frame::Error(ss) => writer.write_all(format!("-{}\r\n", ss).as_bytes()).await?,
            Frame::Integer(i) => writer.write_all(format!(":{}\r\n", i).as_bytes()).await?,
            Frame::BulkString(Some(data)) => {
                let len = format!("${}\r\n", data.len());
                writer.write_all(len.as_bytes()).await?;
                writer.write_all(&data).await?;
                writer.write_all("\r\n".as_bytes()).await?;
            }
            Frame::BulkString(None) => writer.write_all("$0\r\n\r\n".as_bytes()).await?,
            Frame::Array(Some(data)) => {
                for v in data.into_iter() {
                    v.write_to(writer).await?;
                }
            }
            Frame::Null => writer.write_all("_\r\n".as_bytes()).await?,
            Frame::Boolean(b) => {
                writer
                    .write_all(format!("#{}\r\n", if b { 't' } else { 'f' }).as_bytes())
                    .await?;
                log::trace!("write {}", b);
            }
            Frame::Exit => writer.write_all("bye\r\n".as_bytes()).await?,
            _ => writer.write_all("-err\r\n".as_bytes()).await?,
        }
        Ok(())
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Frame::SimpleString(_) => "SimpleString",
            Frame::Error(_) => "Error",
            Frame::Integer(_) => "Integer",
            Frame::BulkString(_) => "BulkString",
            Frame::Array(_) => "Array",
            Frame::Null => "Null",
            Frame::Boolean(_) => "Boolean",
            Frame::Exit => "Exit",
        }
    }
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::SimpleString(s) => write!(f, "{}", s),
            Frame::Error(e) => write!(f, "{}", e),
            Frame::Integer(i) => write!(f, "{}", i),
            Frame::BulkString(items) => {
                if let Some(items) = items {
                    write!(f, "\"{}\"", String::from_utf8_lossy(&items))
                } else {
                    write!(f, "\"\"")
                }
            }
            Frame::Array(vals) => {
                if let Some(vals) = vals {
                    write!(f, "[")?;
                    for val in vals.into_iter() {
                        val.fmt(f)?;
                    }
                    write!(f, "]")
                } else {
                    write!(f, "[]")
                }
            }
            Frame::Null => write!(f, "null"),
            Frame::Boolean(b) => write!(f, "{}", b),
            Frame::Exit => write!(f, "exit"),
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use crate::protocol::{Frame, parse};
    #[cfg(test)]
    use std::vec;
    #[cfg(test)]
    use tokio::io::{BufReader, BufWriter};

    #[test]
    fn test() {
        let s = String::from("12345\r\n");
        s.chars().next();
        dbg!("{}", &s[1..].trim_end());
    }

    #[tokio::test]
    async fn test_parse_array_ok() {
        let mut reader = BufReader::new("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n".as_bytes());
        assert_eq!(
            Frame::Array(Some(vec![
                Frame::BulkString(Some("hello".as_bytes().to_vec())),
                Frame::BulkString(Some("world".as_bytes().to_vec()))
            ])),
            parse(&mut reader).await.unwrap()
        );

        let mut reader =
            BufReader::new("*3\r\n$3\r\nset\r\n$3\r\nkey\r\n$5\r\nvalue\r\n".as_bytes());
        assert_eq!(
            Frame::Array(Some(vec![
                Frame::BulkString(Some("set".as_bytes().to_vec())),
                Frame::BulkString(Some("key".as_bytes().to_vec())),
                Frame::BulkString(Some("value".as_bytes().to_vec())),
            ])),
            parse(&mut reader).await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_write_simple_string_ok() {
        let buf = Vec::new();
        let mut writer = BufWriter::new(buf);
        let value = Frame::SimpleString("HelloWorld".to_string());
        let result = value.write_to(&mut writer).await;

        assert!(result.is_ok());
        assert_eq!(writer.buffer(), "+HelloWorld\r\n".as_bytes());
    }

    #[tokio::test]
    async fn test_write_array_ok() {
        let buf = Vec::new();
        let mut writer = BufWriter::new(buf);
        let value = Frame::Array(Some(vec![
            Frame::SimpleString("HelloWorld".to_string()),
            Frame::BulkString(Some(vec![1, 2, 3, 4, 5])),
            Frame::Integer(64),
        ]));
        let result = value.write_to(&mut writer).await;

        assert!(result.is_ok());
        assert_eq!(
            writer.buffer(),
            [
                "+HelloWorld".as_bytes(),
                "\r\n".as_bytes(),
                "$5\r\n".as_bytes(),
                &[1, 2, 3, 4, 5],
                "\r\n".as_bytes(),
                ":64\r\n".as_bytes(),
            ]
            .concat()
        );
    }
}
