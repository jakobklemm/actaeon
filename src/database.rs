//! # Database
//!
//! Responsible for storing topics and possibly a copy of the routing
//! table on the local file system. All data will be stored in binary,
//! in order to minimize size and make interactions with Wire data as
//! easy as possible.

pub struct Database {
    pub path: String,
}

impl Database {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn convert(bytes: Vec<u8>) -> Vec<Vec<u8>> {
        if bytes.len() < 42 {
            return Vec::new();
        }
        let mut raw: Vec<Vec<u8>> = Vec::new();

        let mut i = 0;
        loop {
            if (i + 42) > bytes.len() {
                break;
            }

            let len = [bytes[i], bytes[i + 1]];
            let total = (len[0] as usize) * 255 + len[1] as usize;

            let subset = &bytes[i..(i + total)];

            raw.push(subset.to_vec());

            i += total;
        }

        return raw;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node::Address;
    use crate::topic::DataTopic;

    #[test]
    fn test_convert_one() {
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        let c = Database::convert(b);
        assert_eq!(c.first().unwrap().len(), 74);
    }

    #[test]
    fn test_convert_multi() {
        let mut data = Vec::new();
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        data.append(&mut b.to_vec());
        let addr = Address::generate("topic").unwrap();
        let mut t = DataTopic::new(addr);
        let n = Address::generate("node").unwrap();
        t.subscribe(n);
        let n = Address::generate("another").unwrap();
        t.subscribe(n);
        let b = t.as_bytes();
        data.append(&mut b.to_vec());

        let c = Database::convert(data);
        assert_eq!(c.first().unwrap().len(), 74);
        assert_eq!(c.last().unwrap().len(), 106);
        assert_eq!(c.len(), 2);
    }
}
