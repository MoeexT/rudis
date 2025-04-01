use anyhow::Result;
use bytes::BytesMut;

use crate::errors::RespError;


pub fn parse_resp(buf: &[u8]) -> Result<Vec<String>, RespError> {
    let message = String::from_utf8_lossy(&buf[..]);
    log::debug!("Parsing message: {:?}", message);
    let mut data = BytesMut::from(buf);
    if data.is_empty() {
        return Err(RespError::EmptyData);
    }

    let mut result = Vec::new();
    match data[0] {
        b'*' => {
            let _ = data.split_to(1);
            let len_end = data
                .iter()
                .position(|&b| b == b'\r')
                .ok_or(RespError::InvalidFormat)?;
            let len = std::str::from_utf8(&data[..len_end])?.parse::<usize>()?;
            let _ = data.split_to(len_end + 2);

            for _ in 0..len {
                if data[0] == b'$' {
                    let _ = data.split_to(1);
                    let str_len_end = data
                        .iter()
                        .position(|&b| b == b'\r')
                        .ok_or(RespError::InvalidFormat)?;
                    let str_len = std::str::from_utf8(&data[..str_len_end])?.parse::<usize>()?;
                    let _ = data.split_to(str_len_end + 2);
                    let str_data = String::from_utf8(data.split_to(str_len).to_vec())?;
                    let _ = data.split_to(2);
                    result.push(str_data);
                }
            }
        }
        _ => return Err(RespError::UnsupportedType),
    }
    Ok(result)
}
