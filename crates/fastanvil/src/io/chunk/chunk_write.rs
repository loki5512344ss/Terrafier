use std::collections::HashMap;

use super::chunk_impl::{Chunk, ChunkSection};

impl Chunk {
    /// Serialize this chunk back to an NBT Compound tag.
    pub fn to_nbt(&self) -> terrafier_nbt::Tag {
        let mut compound = self.raw.clone();

        compound.insert("xPos".into(), terrafier_nbt::Tag::Int(self.x));
        compound.insert("zPos".into(), terrafier_nbt::Tag::Int(self.z));
        compound.insert(
            "DataVersion".into(),
            terrafier_nbt::Tag::Int(self.data_version),
        );

        let sections_list: Vec<terrafier_nbt::Tag> =
            self.sections.iter().map(|s| s.to_nbt()).collect();
        compound.insert("sections".into(), terrafier_nbt::Tag::List(sections_list));

        if let Some(status) = &self.status {
            compound.insert("Status".into(), terrafier_nbt::Tag::String(status.clone()));
        }

        if !self.heightmaps.is_empty() {
            compound.insert(
                "Heightmaps".into(),
                terrafier_nbt::Tag::Compound(self.heightmaps.clone()),
            );
        }

        terrafier_nbt::Tag::Compound(compound)
    }
}

impl ChunkSection {
    /// Serialize section back to NBT Compound.
    pub fn to_nbt(&self) -> terrafier_nbt::Tag {
        let mut compound = HashMap::new();
        compound.insert("Y".into(), terrafier_nbt::Tag::Byte(self.section_y));

        let palette_list: Vec<terrafier_nbt::Tag> = self
            .palette
            .iter()
            .map(|p| terrafier_nbt::Tag::Compound(p.clone()))
            .collect();
        let mut block_states = HashMap::new();
        block_states.insert("palette".into(), terrafier_nbt::Tag::List(palette_list));
        if !self.block_data.is_empty() {
            block_states.insert(
                "data".into(),
                terrafier_nbt::Tag::LongArray(self.block_data.clone()),
            );
        }
        compound.insert(
            "block_states".into(),
            terrafier_nbt::Tag::Compound(block_states),
        );

        let biome_palette_list: Vec<terrafier_nbt::Tag> = self
            .biome_palette
            .iter()
            .map(|p| terrafier_nbt::Tag::Compound(p.clone()))
            .collect();
        let mut biomes = HashMap::new();
        biomes.insert(
            "palette".into(),
            terrafier_nbt::Tag::List(biome_palette_list),
        );
        if !self.biome_data.is_empty() {
            biomes.insert(
                "data".into(),
                terrafier_nbt::Tag::LongArray(self.biome_data.clone()),
            );
        }
        compound.insert("biomes".into(), terrafier_nbt::Tag::Compound(biomes));

        if let Some(bl) = &self.block_light {
            compound.insert(
                "BlockLight".into(),
                terrafier_nbt::Tag::ByteArray(bl.clone()),
            );
        }
        if let Some(sl) = &self.sky_light {
            compound.insert("SkyLight".into(), terrafier_nbt::Tag::ByteArray(sl.clone()));
        }

        terrafier_nbt::Tag::Compound(compound)
    }
}
