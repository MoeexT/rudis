
pub const EMB_LEN: usize = 54;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EmbStr {
    len: u8,
    buf: [u8; EMB_LEN],
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Raw {
    buf: Vec<u8>,
}

/// Create a embed string from bytes
impl From<Vec<u8>> for EmbStr {
    fn from(value: Vec<u8>) -> Self {
        let len = value.len().min(EMB_LEN);
        let mut buf = [0u8; EMB_LEN];
        buf[0..len].copy_from_slice(&value[..len]);
        Self { len: len as u8, buf }
    }
}

/// Clone a embed string to bytes
impl Into<Vec<u8>> for EmbStr {
    fn into(self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.extend_from_slice(&self.buf[..self.len as usize]);
        vec
    }
}

/// Create a raw string from bytes
impl From<Vec<u8>> for Raw {
    fn from(buf: Vec<u8>) -> Self {
        return Self { buf };
    }
}

/// Clone a raw string to bytes
impl Into<Vec<u8>> for Raw {
    fn into(self) -> Vec<u8> {
        self.buf.clone()
    }
}

mod test {
    #[cfg(test)]
    use std::mem;

    #[cfg(test)]
    use crate::storage::object::encoding::sds::{EMB_LEN, EmbStr, Raw};

    #[test]
    fn ensure_static_size() {
        dbg!(mem::size_of::<EmbStr>());
        dbg!(mem::size_of::<Raw>());

        assert_eq!(mem::size_of::<EmbStr>(), 55);
        assert_eq!(mem::size_of::<Raw>(), 24);
    }

    #[test]
    fn test_vec_to_embstr_on_vec_len_less_than_buf() {
        let vec = vec![1u8; 30];
        let embstr: EmbStr = vec.into();

        let mut stub = [0; EMB_LEN];
        for i in 0..30 {
            stub[i] = 1;
        }
        assert_eq!(embstr.buf, stub);
        assert_eq!(embstr.len, 30);
    }
    
    #[test]
    fn test_vec_to_embstr_on_vec_len_equal_to_buf() {
        let vec:Vec<u8> = (0u8..EMB_LEN as u8).into_iter().collect();
        let embstr: EmbStr = vec.into();
        
        let mut stub = [0u8; EMB_LEN];
        for i  in 0..EMB_LEN {
            stub[i] = i as u8;
        }
        assert_eq!(embstr.buf, stub);
        assert_eq!(embstr.len, EMB_LEN as u8);
    }
    
    #[test]
    fn test_vec_to_embstr_on_vec_len_more_than_buf() {
        let vec = vec![1u8; 180];
        let embstr: EmbStr = vec.into();
        
        assert_eq!(embstr.buf, [1; EMB_LEN]);
        assert_eq!(embstr.len, EMB_LEN as u8);
    }
}
