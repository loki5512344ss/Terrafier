//! Chunk data structures for Minecraft Anvil format.
use std::collections::HashMap;

/// A parsed chunk from an Anvil (.mca) region file.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub data_version: i32,
    pub sections: Vec<ChunkSection>,
    pub block_entities: HashMap<String, HashMap<String, terrafier_nbt::Tag>>,
    pub heightmaps: HashMap<String, terrafier_nbt::Tag>,
    pub status: Option<String>,
    pub biomes: Vec<i32>,
    pub raw: HashMap<String, terrafier_nbt::Tag>,
}

/// A single vertical section (16x16x16 blocks) within a chunk.
#[derive(Debug, Clone)]
pub struct ChunkSection {
    pub section_y: i8,
    pub palette: Vec<HashMap<String, terrafier_nbt::Tag>>,
    pub block_data: Vec<i64>,
    pub biome_palette: Vec<HashMap<String, terrafier_nbt::Tag>>,
    pub biome_data: Vec<i64>,
    pub block_light: Option<Vec<i8>>,
    pub sky_light: Option<Vec<i8>>,
}
impl Chunk {
    /// Parse a chunk from an NBT Compound tag.
    pub fn from_nbt(tag: &terrafier_nbt::Tag) -> Option<Self> {
        let compound = match tag {
            terrafier_nbt::Tag::Compound(m) => m,
            _ => return None,
        };

        let x = get_int(compound, "xPos")?;
        let z = get_int(compound, "zPos")?;
        let data_version = get_int(compound, "DataVersion").unwrap_or(0);

        let mut sections = Vec::new();
        if let Some(terrafier_nbt::Tag::List(section_list)) = compound.get("sections") {
            for section_tag in section_list {
                if let Some(section) = ChunkSection::from_nbt(section_tag) {
                    sections.push(section);
                }
            }
        }

        let mut block_entities = HashMap::new();
        if let Some(terrafier_nbt::Tag::List(entity_list)) = compound.get("block_entities") {
            for entity in entity_list {
                if let terrafier_nbt::Tag::Compound(m) = entity {
                    let key = format!("{:?}", m.get("id"));
                    block_entities.insert(key, m.clone());
                }
            }
        }

        let mut heightmaps = HashMap::new();
        if let Some(terrafier_nbt::Tag::Compound(hm)) = compound.get("Heightmaps") {
            for (k, v) in hm {
                heightmaps.insert(k.clone(), v.clone());
            }
        }

        let status = compound.get("Status").and_then(|t| match t {
            terrafier_nbt::Tag::String(s) => Some(s.clone()),
            _ => None,
        });

        let biomes = Vec::new();
        let raw = compound.clone();

        Some(Self {
            x,
            z,
            data_version,
            sections,
            block_entities,
            heightmaps,
            status,
            biomes,
            raw,
        })
    }
}
impl ChunkSection {
    /// Parse a section from an NBT Compound tag.
    pub fn from_nbt(tag: &terrafier_nbt::Tag) -> Option<Self> {
        let compound = match tag {
            terrafier_nbt::Tag::Compound(m) => m,
            _ => return None,
        };

        let section_y = get_byte(compound, "Y")?;

        let palette = if let Some(terrafier_nbt::Tag::List(list)) =
            compound.get("block_states").and_then(|t| match t {
                terrafier_nbt::Tag::Compound(m) => m.get("palette"),
                _ => None,
            }) {
            list.iter()
                .filter_map(|t| match t {
                    terrafier_nbt::Tag::Compound(m) => Some(m.clone()),
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };

        let block_data =
            if let Some(terrafier_nbt::Tag::Compound(bs)) = compound.get("block_states") {
                if let Some(terrafier_nbt::Tag::LongArray(data)) = bs.get("data") {
                    data.clone()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

        let biome_palette = compound
            .get("biomes")
            .and_then(|t| match t {
                terrafier_nbt::Tag::Compound(m) => m.get("palette"),
                _ => None,
            })
            .and_then(|t| {
                if let terrafier_nbt::Tag::List(list) = t {
                    Some(
                        list.iter()
                            .filter_map(|t| match t {
                                terrafier_nbt::Tag::Compound(m) => Some(m.clone()),
                                _ => None,
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let biome_data = compound
            .get("biomes")
            .and_then(|t| match t {
                terrafier_nbt::Tag::Compound(m) => m.get("data"),
                _ => None,
            })
            .and_then(|t| {
                if let terrafier_nbt::Tag::LongArray(data) = t {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let block_light = compound.get("BlockLight").and_then(|t| {
            if let terrafier_nbt::Tag::ByteArray(data) = t {
                Some(data.clone())
            } else {
                None
            }
        });

        let sky_light = compound.get("SkyLight").and_then(|t| {
            if let terrafier_nbt::Tag::ByteArray(data) = t {
                Some(data.clone())
            } else {
                None
            }
        });

        Some(Self {
            section_y,
            palette,
            block_data,
            biome_palette,
            biome_data,
            block_light,
            sky_light,
        })
    }
}
fn get_int(map: &HashMap<String, terrafier_nbt::Tag>, key: &str) -> Option<i32> {
    map.get(key).and_then(|t| match t {
        terrafier_nbt::Tag::Int(v) => Some(*v),
        _ => None,
    })
}

fn get_byte(map: &HashMap<String, terrafier_nbt::Tag>, key: &str) -> Option<i8> {
    map.get(key).and_then(|t| match t {
        terrafier_nbt::Tag::Byte(v) => Some(*v),
        _ => None,
    })
}
