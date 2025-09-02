use std::fmt::Display;

use async_recursion::async_recursion;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RespValue {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RespValue>>),
    Null,
    Boolean(bool),
    Exit,
}

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

    #[error("Value is not an integer or out of range")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("I/O error")]
    Io(#[from] std::io::Error),
}

// 通用的 RespValue 转换 trait
pub trait FromRespValue: Sized {
    fn from_resp_value(value: RespValue, command_name: &str) -> Result<Self, RespError>;
}

// 为 String 实现转换
impl FromRespValue for String {
    fn from_resp_value(value: RespValue, command_name: &str) -> Result<Self, RespError> {
        match value {
            RespValue::SimpleString(s) => Ok(s),
            RespValue::BulkString(Some(bytes)) => String::from_utf8(bytes)
                .map_err(|e| RespError::FromUtf8Error(e)),
            _ => Err(RespError::InvalidFormat),
        }
    }
}

// 为 i64 实现转换
impl FromRespValue for i64 {
    fn from_resp_value(value: RespValue, command_name: &str) -> Result<Self, RespError> {
        match value {
            RespValue::Integer(num) => Ok(num),
            RespValue::BulkString(Some(bytes)) => {
                let s = String::from_utf8(bytes)
                    .map_err(|e| RespError::FromUtf8Error(e))?;
                s.parse().map_err(|_| RespError::InvalidFormat)
            }
            _ => Err(RespError::InvalidFormat),
        }
    }
}

// 为 bool 实现转换
impl FromRespValue for bool {
    fn from_resp_value(value: RespValue, command_name: &str) -> Result<Self, RespError> {
        match value {
            RespValue::Integer(0) => Ok(false),
            RespValue::Integer(1) => Ok(true),
            RespValue::SimpleString(s) if s.to_uppercase() == "TRUE" => Ok(true),
            RespValue::SimpleString(s) if s.to_uppercase() == "FALSE" => Ok(false),
            _ => Err(RespError::InvalidFormat),
        }
    }
}

impl FromRespValue for Vec<u8> {
    fn from_resp_value(value: RespValue, command_name: &str) -> Result<Self, RespError> {
        match value {
            RespValue::BulkString(Some(bytes)) => Ok(bytes),
            _ => Err(RespError::InvalidFormat),
        }
    }
}

#[async_recursion]
pub async fn parse_resp<R>(reader: &mut BufReader<R>) -> Result<RespValue, RespError>
where
    R: AsyncReadExt + Unpin + Send,
{
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.is_empty() {
        return Err(RespError::UnsupportedType);
    }

    let prefix = line.chars().next().ok_or(RespError::UnsupportedType)?;
    let content = &line[1..].trim_end();

    match prefix {
        '+' => Ok(RespValue::SimpleString(content.to_string())),
        '-' => Ok(RespValue::Error(content.to_string())),
        ':' => Ok(RespValue::Integer(content.parse()?)),
        '$' => {
            // get string length
            let len: isize = content.parse()?;
            if len == -1 {
                return Ok(RespValue::BulkString(None));
            }
            let mut buf = vec![0; len as usize];
            reader.read_exact(&mut buf).await?;
            // Read the trailing \r\n
            let mut crlf = [0; 2];
            reader.read_exact(&mut crlf).await?;
            Ok(RespValue::BulkString(Some(buf)))
        }
        '*' => {
            let count: isize = content.parse()?;
            if count == -1 {
                return Ok(RespValue::Array(None));
            }
            let mut items = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let val = parse_resp(reader).await?;
                items.push(val);
            }
            Ok(RespValue::Array(Some(items)))
        }
        _ => Err(RespError::UnsupportedType),
    }
}

impl<'a> RespValue {
    /// Turns the `RespValue` to bytes then write them to `BufWriter`,
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
    pub async fn write_to<W>(self, writer: &mut BufWriter<W>) -> Result<(), RespError>
    where
        W: AsyncWriteExt + Unpin + Send,
    {
        match self {
            RespValue::SimpleString(ss) => {
                writer.write_all(format!("+{}\r\n", ss).as_bytes()).await?
            }
            RespValue::Error(ss) => writer.write_all(format!("-{}\r\n", ss).as_bytes()).await?,
            RespValue::Integer(i) => writer.write_all(format!(":{}\r\n", i).as_bytes()).await?,
            RespValue::BulkString(Some(data)) => {
                let resp = format!("${}\r\n", data.len());
                writer.write_all(resp.as_bytes()).await?;
                writer.write_all(&data).await?;
                writer.write_all("\r\n".as_bytes()).await?;
            }
            RespValue::BulkString(None) => writer.write_all("$0\r\n\r\n".as_bytes()).await?,
            RespValue::Array(Some(data)) => {
                for v in data.into_iter() {
                    v.write_to(writer).await?;
                }
            }
            RespValue::Null => writer.write_all("_\r\n".as_bytes()).await?,
            RespValue::Boolean(b) => {
                writer
                    .write_all(format!("#{}\r\n", if b { 't' } else { 'f' }).as_bytes())
                    .await?;
                log::trace!("write {}", b);
            }
            RespValue::Exit => writer.write_all("bye\r\n".as_bytes()).await?,
            _ => writer.write_all("-err\r\n".as_bytes()).await?,
        }
        Ok(())
    }
}

impl Display for RespValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RespValue::SimpleString(s) => write!(f, "{}", s),
            RespValue::Error(e) => write!(f, "{}", e),
            RespValue::Integer(i) => write!(f, "{}", i),
            RespValue::BulkString(items) => {
                if let Some(items) = items {
                    write!(f, "\"{}\"", String::from_utf8_lossy(&items))
                } else {
                    write!(f, "\"\"")
                }
            }
            RespValue::Array(vals) => {
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
            RespValue::Null => write!(f, "null"),
            RespValue::Boolean(b) => write!(f, "{}", b),
            RespValue::Exit => write!(f, "exit"),
        }
    }
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use crate::resp::{RespValue, parse_resp};
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
            RespValue::Array(Some(vec![
                RespValue::BulkString(Some("hello".as_bytes().to_vec())),
                RespValue::BulkString(Some("world".as_bytes().to_vec()))
            ])),
            parse_resp(&mut reader).await.unwrap()
        );

        let mut reader =
            BufReader::new("*3\r\n$3\r\nset\r\n$3\r\nkey\r\n$5\r\nvalue\r\n".as_bytes());
        assert_eq!(
            RespValue::Array(Some(vec![
                RespValue::BulkString(Some("set".as_bytes().to_vec())),
                RespValue::BulkString(Some("key".as_bytes().to_vec())),
                RespValue::BulkString(Some("value".as_bytes().to_vec())),
            ])),
            parse_resp(&mut reader).await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_write_simple_string_ok() {
        let buf = Vec::new();
        let mut writer = BufWriter::new(buf);
        let value = RespValue::SimpleString("HelloWorld".to_string());
        let result = value.write_to(&mut writer).await;

        assert!(result.is_ok());
        assert_eq!(writer.buffer(), "+HelloWorld\r\n".as_bytes());
    }

    #[tokio::test]
    async fn test_write_array_ok() {
        let buf = Vec::new();
        let mut writer = BufWriter::new(buf);
        let value = RespValue::Array(Some(vec![
            RespValue::SimpleString("HelloWorld".to_string()),
            RespValue::BulkString(Some(vec![1, 2, 3, 4, 5])),
            RespValue::Integer(64),
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
