use std::collections::HashMap;
use terrafier_nbt::Tag;

use super::palette_impl::{BlockPalette, BlockState};

impl BlockState {
    /// Parse from `TAG_Compound` with `"Name"` and optional `"Properties"`.
    pub fn from_nbt(tag: &Tag) -> Option<Self> {
        let map = match tag {
            Tag::Compound(m) => m,
            _ => return None,
        };
        let name = match map.get("Name")? {
            Tag::String(s) => s.clone(),
            _ => return None,
        };
        let properties = match map.get("Properties") {
            Some(Tag::Compound(props)) => props
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        match v {
                            Tag::String(s) => s.clone(),
                            _ => String::new(),
                        },
                    )
                })
                .collect(),
            _ => HashMap::new(),
        };
        Some(BlockState { name, properties })
    }

    /// Serialize to `TAG_Compound`.
    pub fn to_nbt(&self) -> Tag {
        let mut map = HashMap::new();
        map.insert("Name".into(), Tag::String(self.name.clone()));
        if !self.properties.is_empty() {
            let props = self
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), Tag::String(v.clone())))
                .collect();
            map.insert("Properties".into(), Tag::Compound(props));
        }
        Tag::Compound(map)
    }
}

impl BlockPalette {
    /// Import from NBT palette list + block data `LongArray`.
    pub fn from_nbt(palette_list: &[Tag], data: &[i64]) -> (Self, crate::bits::BitArray) {
        let mut palette = BlockPalette::new();
        for tag in palette_list {
            if let Some(state) = BlockState::from_nbt(tag) {
                palette.add_or_get(state);
            }
        }
        let bpe = palette.bits_per_entry();
        (
            palette,
            crate::bits::BitArray::from_raw(data.to_vec(), bpe, 4096),
        )
    }

    /// Export to NBT palette list + block data `LongArray`.
    pub fn to_nbt(&self, bitarray: &crate::bits::BitArray) -> (Vec<Tag>, Vec<i64>) {
        let list: Vec<Tag> = self.entries.iter().map(|s| s.to_nbt()).collect();
        (list, bitarray.data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::super::palette_impl::{BlockPalette, BlockState};
    use terrafier_nbt::Tag;

    #[test]
    fn test_add_get() {
        let mut p = BlockPalette::new();
        let air = BlockState::new("minecraft:air");
        let stone = BlockState::new("minecraft:stone");
        assert_eq!(p.add_or_get(air.clone()), 0);
        assert_eq!(p.add_or_get(stone.clone()), 1);
        assert_eq!(p.add_or_get(air.clone()), 0);
        assert_eq!(p.get(0), Some(&air));
        assert_eq!(p.get(1), Some(&stone));
        assert!(p.get(99).is_none());
    }

    #[test]
    fn test_from_nbt() {
        let s = BlockState::new("minecraft:grass_block").with_property("snowy", "true");
        assert_eq!(BlockState::from_nbt(&s.to_nbt()).unwrap(), s);
    }

    #[test]
    fn test_to_nbt() {
        let nbt = BlockState::new("minecraft:stone").to_nbt();
        let map = match nbt {
            Tag::Compound(ref m) => m,
            _ => panic!("expected compound"),
        };
        let name = match map.get("Name").unwrap() {
            Tag::String(s) => s.as_str(),
            _ => panic!("expected string"),
        };
        assert_eq!(name, "minecraft:stone");
    }

    #[test]
    fn test_roundtrip() {
        let states = vec![
            BlockState::new("minecraft:air"),
            BlockState::new("minecraft:stone"),
            BlockState::new("minecraft:grass_block").with_property("snowy", "true"),
        ];
        let mut palette = BlockPalette::new();
        for s in &states {
            palette.add_or_get(s.clone());
        }
        let mut ba = crate::bits::BitArray::new(4096, palette.bits_per_entry());
        for i in 0..10 {
            ba.set(i, (i % 3) as i64);
        }
        let (list, data) = palette.to_nbt(&ba);
        let (palette2, ba2) = BlockPalette::from_nbt(&list, &data);
        assert_eq!(palette.len(), palette2.len());
        for i in 0..10 {
            assert_eq!(ba.get(i), ba2.get(i));
        }
    }
}
