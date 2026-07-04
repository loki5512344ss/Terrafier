use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockState {
    pub name: String,
    pub properties: HashMap<String, String>,
}

impl Hash for BlockState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        let mut keys: Vec<&String> = self.properties.keys().collect();
        keys.sort();
        for k in keys {
            k.hash(state);
            self.properties[k].hash(state);
        }
    }
}

impl BlockState {
    pub fn new(name: &str) -> Self {
        BlockState {
            name: name.to_string(),
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: &str, val: &str) -> Self {
        self.properties.insert(key.to_string(), val.to_string());
        self
    }
}

/// A palette mapping unique `BlockState`s to compact indices.
pub struct BlockPalette {
    pub entries: Vec<BlockState>,
    index_map: HashMap<BlockState, u32>,
}

impl Default for BlockPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockPalette {
    pub fn new() -> Self {
        BlockPalette {
            entries: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a block state, or return its existing index.
    pub fn add_or_get(&mut self, state: BlockState) -> u32 {
        if let Some(&idx) = self.index_map.get(&state) {
            return idx;
        }
        let idx = self.entries.len() as u32;
        self.index_map.insert(state.clone(), idx);
        self.entries.push(state);
        idx
    }

    pub fn get(&self, index: u32) -> Option<&BlockState> {
        self.entries.get(index as usize)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Minimum bits per entry needed for this palette.
    pub fn bits_per_entry(&self) -> u8 {
        crate::bits::BitArray::bits_needed(self.entries.len())
    }
}
