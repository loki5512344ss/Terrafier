use std::io::Write;

use super::writer::{NbtWriter, Result, WriteError};
use crate::tag::Tag;

impl<W: Write> NbtWriter<W> {
    pub(crate) fn write_tag(&mut self, tag: &Tag, name: Option<&str>) -> Result<()> {
        self.write_u8(tag.id())?;
        if let Some(n) = name {
            self.write_string(n)?;
        }
        self.write_tag_payload(tag)?;
        Ok(())
    }

    pub(crate) fn write_tag_payload(&mut self, tag: &Tag) -> Result<()> {
        match tag {
            Tag::End => {}
            Tag::Byte(v) => self.write_u8(*v as u8)?,
            Tag::Short(v) => self.write_i16_be(*v)?,
            Tag::Int(v) => self.write_i32_be(*v)?,
            Tag::Long(v) => self.write_i64_be(*v)?,
            Tag::Float(v) => self.write_f32_be(*v)?,
            Tag::Double(v) => self.write_f64_be(*v)?,
            Tag::String(v) => self.write_string(v)?,
            Tag::List(items) => {
                if items.is_empty() {
                    self.write_u8(1)?;
                    self.write_i32_be(0)?;
                } else {
                    let elem_type = items[0].id();
                    self.write_u8(elem_type)?;
                    self.write_i32_be(items.len() as i32)?;
                    for item in items {
                        self.write_tag_payload(item)?;
                    }
                }
            }
            Tag::Compound(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for key in keys {
                    if let Some(val) = map.get(key) {
                        self.write_tag(val, Some(key))?;
                    }
                }
                self.write_u8(0)?;
            }
            Tag::ByteArray(v) => {
                self.write_i32_be(v.len() as i32)?;
                for b in v {
                    self.inner.write_all(&[*b as u8])?;
                }
            }
            Tag::IntArray(v) => {
                self.write_i32_be(v.len() as i32)?;
                for n in v {
                    self.write_i32_be(*n)?;
                }
            }
            Tag::LongArray(v) => {
                self.write_i32_be(v.len() as i32)?;
                for n in v {
                    self.write_i64_be(*n)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn write_tag_compound_root(&mut self, tag: &Tag) -> Result<()> {
        match tag {
            Tag::Compound(map) => {
                self.write_u8(9)?;
                self.write_string("")?;
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for key in keys {
                    if let Some(val) = map.get(key) {
                        self.write_tag(val, Some(key))?;
                    }
                }
                self.write_u8(0)?;
                Ok(())
            }
            _ => Err(WriteError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Root tag must be Compound",
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::writer::to_bytes;
    use super::super::writer::to_gzip_bytes;
    use crate::io::reader;
    use crate::tag::Tag;
    use std::collections::HashMap;

    #[test]
    fn test_roundtrip_byte() {
        let tag = Tag::Compound(HashMap::from([("val".into(), Tag::Byte(42))]));
        let bytes = to_bytes(&tag).unwrap();
        let parsed = reader::read_bytes(&bytes).unwrap();
        assert_eq!(tag, parsed);
    }

    #[test]
    fn test_roundtrip_nested() {
        let inner = Tag::Compound(HashMap::from([
            ("x".into(), Tag::Int(100)),
            ("y".into(), Tag::Int(200)),
        ]));
        let tag = Tag::Compound(HashMap::from([("pos".into(), inner)]));
        let bytes = to_bytes(&tag).unwrap();
        let parsed = reader::read_bytes(&bytes).unwrap();
        assert_eq!(tag, parsed);
    }

    #[test]
    fn test_roundtrip_all_types() {
        let mut map = HashMap::new();
        map.insert("byte".into(), Tag::Byte(1));
        map.insert("short".into(), Tag::Short(2));
        map.insert("int".into(), Tag::Int(3));
        map.insert("long".into(), Tag::Long(4));
        map.insert("float".into(), Tag::Float(5.0));
        map.insert("double".into(), Tag::Double(6.0));
        map.insert("string".into(), Tag::String("hello".into()));
        map.insert("bytearray".into(), Tag::ByteArray(vec![1, 2, 3]));
        map.insert("intarray".into(), Tag::IntArray(vec![4, 5, 6]));
        map.insert("longarray".into(), Tag::LongArray(vec![7, 8, 9]));
        let tag = Tag::Compound(map);
        let bytes = to_bytes(&tag).unwrap();
        let parsed = reader::read_bytes(&bytes).unwrap();
        assert_eq!(tag, parsed);
    }

    #[test]
    fn test_gzip_roundtrip() {
        let tag = Tag::Compound(HashMap::from([("val".into(), Tag::Int(12345))]));
        let gz = to_gzip_bytes(&tag).unwrap();
        let parsed = reader::read_gzip(&gz).unwrap();
        assert_eq!(tag, parsed);
    }
}
