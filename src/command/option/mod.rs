use crate::command::error::CommandError;

#[derive(PartialEq, Eq, Debug)]
pub(super) enum Expiration {
    EX(u64), // seconds
    PX(u64), // milliseconds
    EXAT(u64), // absolute timestamp in seconds
    PXAT(u64), // absolute timestamp in milliseconds
}

impl TryFrom<(String, String)> for Expiration {
    type Error = CommandError;

    fn try_from(pair: (String, String)) -> Result<Self, Self::Error> {
        let (k, v_str) = pair;
        let key = k.to_uppercase();
        let v = v_str.parse::<u64>().map_err(|e| CommandError::ParseIntError(e))?;
        match key.as_str() {
            "EX" => Ok(Expiration::EX(v)),
            "PX" => Ok(Expiration::PX(v)),
            "EXAT" => Ok(Expiration::EXAT(v)),
            "PXAT" => Ok(Expiration::PXAT(v)),
            _ => Err(CommandError::InvalidCommandFormat(format!("Unknown expiration type {}", k))),
        }
    }
}
