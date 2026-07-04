use crate::compression::{self, CompressionType};

use super::region_impl::{Region, Result};

impl Region {
    /// Serialize region back to .mca bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let sector_size: u64 = 4096;
        let mut locations = [0u32; 1024];
        let mut timestamps = [0u32; 1024];
        let mut sector_data: Vec<Vec<u8>> = Vec::new();

        for i in 0..1024 {
            let local_x = (i % 32) as u8;
            let local_z = (i / 32) as u8;

            if let Some(entry) = self.chunks.get(&(local_x, local_z)) {
                timestamps[i] = entry.timestamp;

                let compressed = compression::compress(
                    entry.data.as_deref().unwrap_or_default(),
                    CompressionType::Zlib,
                )?;

                let total_len = 1 + compressed.len();
                let mut sector = Vec::with_capacity(4 + total_len);
                sector.extend(&(total_len as u32).to_be_bytes());
                sector.push(CompressionType::Zlib.id());
                sector.extend(&compressed);

                while sector.len() % sector_size as usize != 0 {
                    sector.push(0);
                }

                let offset = 2 + sector_data.len() as u32;
                locations[i] =
                    (offset << 8) | ((sector.len() / sector_size as usize) as u32 & 0xFF);
                sector_data.push(sector);
            }
        }

        let mut output =
            Vec::with_capacity(2 * 4096 + sector_data.iter().map(|s| s.len()).sum::<usize>());

        for loc in locations.iter() {
            output.extend(&loc.to_be_bytes());
        }
        for ts in timestamps.iter() {
            output.extend(&ts.to_be_bytes());
        }

        while output.len() < 2 * sector_size as usize {
            output.push(0);
        }

        for sector in &sector_data {
            output.extend(sector);
        }

        Ok(output)
    }
}
