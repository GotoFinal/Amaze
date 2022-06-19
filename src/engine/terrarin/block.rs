use std::ops::{Add, Deref};
use glam::{IVec3, Vec3};
use num_traits::real::Real;
use crate::engine::object::transform::Pos;
use crate::engine::terrarin::chunk::{CHUNK_SIZE_EXP, CHUNK_SIZE_I, ChunkPos};

#[derive(Copy, Clone, PartialEq)]
pub struct Block {
    pub id: u16,
}

#[derive(Copy, Clone, PartialEq)]
pub struct BlockPos(pub IVec3);

impl From<IVec3> for BlockPos {
    fn from(value: IVec3) -> BlockPos { BlockPos::vec(value) }
}

impl From<ChunkPos> for BlockPos {
    fn from(value: ChunkPos) -> BlockPos { value.block_min() }
}

impl From<Pos> for BlockPos {
    fn from(value: Pos) -> BlockPos { value.block() }
}

impl Deref for BlockPos {
    type Target = IVec3;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl BlockPos {
    pub fn chunk(self) -> ChunkPos { ChunkPos::from_block(self) }
    pub fn pos(self) -> Pos { Pos::from_block(self) }
    pub fn pos_center(self) -> Pos { (Pos::from_block(self).0 + 0.5).into() }
    pub fn pos_center_ground(self) -> Pos { (Pos::from_block(self).add(Vec3::new(0.5, 0.0, 0.5))).into() }

    pub fn chunk_relative(self) -> BlockPos {
        let chunk_pos = self.chunk();
        let world_pos = (chunk_pos.0 * CHUNK_SIZE_I);
        return (self.0 - world_pos).into();
    }

    pub fn vec(pos: IVec3) -> BlockPos { BlockPos(pos) }
    pub fn new(x: i32, y: i32, z: i32) -> BlockPos { BlockPos::vec(IVec3::new(x, y, z)) }


    pub fn from_world(world_pos: Pos) -> BlockPos {
        return BlockPos(world_pos.floor().as_ivec3());
    }

    pub fn chunk_at(pos: i32) -> i32 { pos >> CHUNK_SIZE_EXP }

    pub fn chunk_relative_at(pos: i32) -> i32 {
        let world_pos = Self::chunk_at(pos) * CHUNK_SIZE_I;
        return pos - world_pos;
    }
}

// TODO: move this traits to other files so they willbe used to all other types
pub trait AddXYZi32 {
    fn add(self, x: i32, y: i32, z: i32) -> Self;
}

impl AddXYZi32 for BlockPos {
    fn add(self, x: i32, y: i32, z: i32) -> Self {
        return self.0.add(IVec3::new(x, y, z)).into();
    }
}

pub trait AddBlockPos {
    fn add(self, pos: BlockPos) -> Self;
}

impl AddBlockPos for BlockPos {
    fn add(self, pos: BlockPos) -> Self {
        return self.0.add(pos.0).into();
    }
}