use crate::engine::terrarin::block::{Block, BlockPos};
use crate::engine::terrarin::chunk::{Chunk, CHUNK_SIZE, CHUNK_SIZE_I, ChunkPos};
use crate::engine::terrarin::world::GameWorld;

pub trait ChunkGenerator {
    fn generate_chunk(&self, world: &GameWorld, pos: ChunkPos) -> Chunk;
}

pub struct FlatEarthGenerator {
    pub grass_level: i32,
    pub stone_level: i32,
}

impl ChunkGenerator for FlatEarthGenerator {
    fn generate_chunk(&self, _: &GameWorld, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::empty(pos);
        let chunk_block_y = pos.block_min().y;
        for y in 0..CHUNK_SIZE {
            let real_y = chunk_block_y + y as i32;
            let material = if real_y <= self.stone_level {
                Block { id: 2 }
            } else if real_y <= self.grass_level {
                Block { id: 1 }
            } else {
                Block { id: 0 }
            };
            if material.id != 0 {
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        chunk[x][y][z] = material
                    }
                }
            }
        }
        return chunk;
    }
}

