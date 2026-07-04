use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::tile::Tile;

#[derive(Debug, Clone)]
pub struct Dimension {
    pub name: String,
    pub tiles: HashMap<(i32, i32), Tile>,
    pub min_height: i16,
    pub max_height: i16,
    pub seed: u64,
}

impl Serialize for Dimension {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let tiles: HashMap<String, &Tile> = self
            .tiles
            .iter()
            .map(|(&(tx, tz), tile)| (format!("{},{}", tx, tz), tile))
            .collect();
        #[derive(Serialize)]
        struct Dim<'a> {
            name: &'a str,
            tiles: HashMap<String, &'a Tile>,
            min_height: i16,
            max_height: i16,
            seed: u64,
        }
        Dim {
            name: &self.name,
            tiles,
            min_height: self.min_height,
            max_height: self.max_height,
            seed: self.seed,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Dimension {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Dim {
            name: String,
            tiles: HashMap<String, Tile>,
            min_height: i16,
            max_height: i16,
            seed: u64,
        }
        let dim = Dim::deserialize(d)?;
        let mut tiles = HashMap::with_capacity(dim.tiles.len());
        for (key, tile) in dim.tiles {
            if let Some((tx_s, tz_s)) = key.split_once(',')
                && let (Ok(tx), Ok(tz)) = (tx_s.parse::<i32>(), tz_s.parse::<i32>())
            {
                tiles.insert((tx, tz), tile);
            } else {
                log::warn!("Skipping dimension tile with malformed key: {}", key);
            }
        }
        Ok(Self {
            name: dim.name,
            tiles,
            min_height: dim.min_height,
            max_height: dim.max_height,
            seed: dim.seed,
        })
    }
}
