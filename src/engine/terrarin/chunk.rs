use std::cmp::min;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Deref, Index, IndexMut};
use std::process::Output;
use glam::{IVec2, IVec3, Vec2, Vec3};
use crate::engine::object::transform::Pos;
use crate::engine::terrarin::block::{AddXYZi32, Block, BlockPos};

pub const CHUNK_SIZE_EXP: u32 = 4;
pub const CHUNK_SIZE: usize = (2 as u32).pow(CHUNK_SIZE_EXP) as usize;
pub const CHUNK_SIZE_I: i32 = CHUNK_SIZE as i32;


pub struct Chunk {
    position: ChunkPos,
    blocks: [[[Block; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl Chunk {
    pub fn empty(pos: ChunkPos) -> Chunk {
        let blocks = [[[Block { id: 0 }; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
        return Chunk {
            position: pos,
            blocks,
        };
    }
    pub fn get_position(&self) -> ChunkPos {
        return self.position;
    }
}

pub struct ChunkIntoIterator {
    chunk: Chunk,
    min: BlockPos,
    x: i32,
    y: i32,
    z: i32,
}

impl IntoIterator for Chunk {
    type Item = (BlockPos, Block);
    type IntoIter = ChunkIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        return ChunkIntoIterator {
            min: self.position.block_min(),
            chunk: self,
            x: 0,
            y: 0,
            z: 0,
        };
    }
}

impl Iterator for ChunkIntoIterator {
    type Item = (BlockPos, Block);

    fn next(&mut self) -> Option<Self::Item> {
        if self.x == CHUNK_SIZE_I {
            return None;
        }
        let curr = self.chunk[self.x as usize][self.y as usize][self.z as usize];
        let pos = self.min.add(self.x, self.y, self.z);
        if self.z < CHUNK_SIZE_I {
            self.z += 1;
        } else {
            self.z = 0;
            if self.y < CHUNK_SIZE_I {
                self.y += 1;
            } else {
                self.y = 0;
                if self.x < CHUNK_SIZE_I {
                    self.x += 1;
                } else {
                    self.x = CHUNK_SIZE_I;
                    return None;
                }
            }
        }
        Some((pos, curr))
    }
}

impl Index<usize> for Chunk {
    type Output = [[Block; CHUNK_SIZE]; CHUNK_SIZE];
    fn index(&self, i: usize) -> &Self::Output {
        &self.blocks[i]
    }
}

impl IndexMut<usize> for Chunk {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.blocks[i]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ChunkPos(pub IVec3);

impl From<IVec3> for ChunkPos {
    fn from(value: IVec3) -> ChunkPos { ChunkPos::vec(value) }
}

impl Deref for ChunkPos {
    type Target = IVec3;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<BlockPos> for ChunkPos {
    fn from(value: BlockPos) -> ChunkPos { ChunkPos::from_block(value) }
}

impl From<Pos> for ChunkPos {
    fn from(value: Pos) -> ChunkPos { ChunkPos::from_world(value) }
}


impl ChunkPos {
    pub fn block_min(self) -> BlockPos { (self.0 * CHUNK_SIZE_I).into() }
    pub fn block_max(self) -> BlockPos { (self.block_min().0 + (CHUNK_SIZE_I - 1)).into() }
    pub fn world_min(self) -> Pos { (self.0 * CHUNK_SIZE_I).into() }
    pub fn world_max(self) -> Pos { (self.block_min().0 + (CHUNK_SIZE_I - 1)).into() }

    pub fn vec(pos: IVec3) -> ChunkPos { ChunkPos(pos) }
    pub fn new(x: i32, y: i32, z: i32) -> ChunkPos { ChunkPos::vec(IVec3::new(x, y, z)) }

    pub fn from_world(pos: Pos) -> ChunkPos {
        return Self::from_block(BlockPos::from_world(pos));
    }

    pub fn from_block(pos: BlockPos) -> ChunkPos { ChunkPos(pos.0 >> CHUNK_SIZE_EXP) }
}
