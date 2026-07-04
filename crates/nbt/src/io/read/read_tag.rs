use std::collections::HashMap;
use std::io::Read;

use super::reader::{NbtReader, ReadError, Result};
use crate::tag::Tag;

impl<R: Read> NbtReader<R> {
    pub(crate) fn read_tag_payload(&mut self, tag_type: u8) -> Result<Tag> {
        match tag_type {
            0 => Ok(Tag::End),
            1 => Ok(Tag::Byte(self.read_u8()? as i8)),
            2 => Ok(Tag::Short(self.read_i16_be()?)),
            3 => Ok(Tag::Int(self.read_i32_be()?)),
            4 => Ok(Tag::Long(self.read_i64_be()?)),
            5 => Ok(Tag::Float(self.read_f32_be()?)),
            6 => Ok(Tag::Double(self.read_f64_be()?)),
            7 => Ok(Tag::String(self.read_string()?)),
            8 => {
                let elem_type = self.read_u8()?;
                let len = self.read_i32_be()? as usize;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    items.push(self.read_tag_payload(elem_type)?);
                }
                Ok(Tag::List(items))
            }
            9 => {
                let mut map = HashMap::new();
                loop {
                    let t = self.read_u8()?;
                    if t == 0 {
                        break;
                    }
                    let name = self.read_string()?;
                    let val = self.read_tag_payload(t)?;
                    map.insert(name, val);
                }
                Ok(Tag::Compound(map))
            }
            10 => {
                let len = self.read_i32_be()? as usize;
                let bytes = self.read_exact(len)?.to_vec();
                Ok(Tag::ByteArray(bytes.into_iter().map(|b| b as i8).collect()))
            }
            11 => {
                let len = self.read_i32_be()? as usize;
                let mut vals = Vec::with_capacity(len);
                for _ in 0..len {
                    vals.push(self.read_i32_be()?);
                }
                Ok(Tag::IntArray(vals))
            }
            12 => {
                let len = self.read_i32_be()? as usize;
                let mut vals = Vec::with_capacity(len);
                for _ in 0..len {
                    vals.push(self.read_i64_be()?);
                }
                Ok(Tag::LongArray(vals))
            }
            _ => Err(ReadError::UnknownTagType(tag_type)),
        }
    }

    pub(crate) fn read_tag_compound_root(&mut self) -> Result<Tag> {
        let t = self.read_u8()?;
        if t == 0 {
            return Ok(Tag::Compound(HashMap::new()));
        }
        if t != 9 {
            return Err(ReadError::UnknownTagType(t));
        }
        let _name = self.read_string()?;
        self.read_tag_payload(9)
    }
}

#[cfg(test)]
mod tests {
    use super::super::reader::read_bytes;
    use crate::tag::Tag;
    use std::collections::HashMap;

    fn root_envelope(data: &[u8]) -> Vec<u8> {
        let mut buf = vec![0x09, 0x00, 0x00];
        buf.extend(data);
        buf.push(0x00);
        buf
    }

    #[test]
    fn test_read_byte() {
        let payload = vec![0x01, 0x00, 0x04, b't', b'e', b's', b't', 0x2a, 0x00];
        let tag = read_bytes(&root_envelope(&payload)).unwrap();
        let expected = Tag::Compound(HashMap::from([("test".into(), Tag::Byte(42))]));
        assert_eq!(tag, expected);
    }

    #[test]
    fn test_read_string() {
        let payload = vec![
            0x07, 0x00, 0x04, b'n', b'a', b'm', b'e', 0x00, 0x05, b'H', b'e', b'l', b'l', b'o',
        ];
        let tag = read_bytes(&root_envelope(&payload)).unwrap();
        let expected = Tag::Compound(HashMap::from([(
            "name".into(),
            Tag::String("Hello".into()),
        )]));
        assert_eq!(tag, expected);
    }

    #[test]
    fn test_read_compound_nested() {
        let inner = vec![0x01, 0x00, 0x03, b'k', b'e', b'y', 0x07, 0x00];
        let mut payload = vec![0x09, 0x00, 0x05, b'c', b'h', b'i', b'l', b'd'];
        payload.extend(inner);
        payload.push(0x00);
        let tag = read_bytes(&root_envelope(&payload)).unwrap();
        let inner_map = HashMap::from([("key".into(), Tag::Byte(7))]);
        let expected = Tag::Compound(HashMap::from([("child".into(), Tag::Compound(inner_map))]));
        assert_eq!(tag, expected);
    }

    #[test]
    fn test_read_int_array() {
        let mut payload = vec![0x0b, 0x00, 0x03, b'a', b'r', b'r', 0x00, 0x00, 0x00, 0x02];
        payload.extend(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02]);
        payload.push(0x00);
        let tag = read_bytes(&root_envelope(&payload)).unwrap();
        let expected = Tag::Compound(HashMap::from([("arr".into(), Tag::IntArray(vec![1, 2]))]));
        assert_eq!(tag, expected);
    }

    #[test]
    fn test_read_long_array() {
        let mut payload = vec![0x0c, 0x00, 0x03, b'l', b'n', b'g', 0x00, 0x00, 0x00, 0x01];
        payload.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2a]);
        payload.push(0x00);
        let tag = read_bytes(&root_envelope(&payload)).unwrap();
        let expected = Tag::Compound(HashMap::from([("lng".into(), Tag::LongArray(vec![42]))]));
        assert_eq!(tag, expected);
    }
}
