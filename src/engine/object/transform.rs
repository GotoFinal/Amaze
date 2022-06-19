use std::fmt::{Display, Formatter};
use std::ops::Deref;
use glam::{IVec3, Mat4, Quat, Vec3, Vec4};
use bevy_ecs::prelude::*;
use num_traits::real::Real;
use crate::engine::terrarin::block::BlockPos;
use crate::engine::terrarin::chunk::{Chunk, CHUNK_SIZE_I, ChunkPos};

#[derive(Component, Clone, Copy, Debug)]
pub struct Pos(pub Vec3);

impl Display for Pos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
    // fn fmt(&self, f: &mut Formatter<'_>) -> Result<T, E> {
    //     write!(f, "({}, {})", self.x, self.y)
    // }
}

impl From<Vec3> for Pos {
    fn from(value: Vec3) -> Pos { Pos::vec(value) }
}

impl From<IVec3> for Pos {
    fn from(value: IVec3) -> Pos { Pos::vec(value.as_vec3()) }
}

impl From<ChunkPos> for Pos {
    fn from(value: ChunkPos) -> Pos { value.world_min() }
}

impl From<BlockPos> for Pos {
    fn from(value: BlockPos) -> Pos { value.pos() }
}

impl Deref for Pos {
    type Target = Vec3;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl Pos {
    pub fn chunk(self) -> ChunkPos { ChunkPos::from_world(self) }
    pub fn block(self) -> BlockPos { BlockPos::from_world(self) }

    pub fn chunk_relative(self) -> Pos {
        let chunk_pos = self.chunk();
        let world_pos = (chunk_pos.0 * CHUNK_SIZE_I).as_vec3();
        return (self.0 - world_pos).into();
    }

    pub fn vec(pos: Vec3) -> Pos { Pos(pos) }
    pub fn new(x: f32, y: f32, z: f32) -> Pos { Pos::vec(Vec3::new(x, y, z)) }

    pub fn from_block(pos: BlockPos) -> Pos { Pos(pos.as_vec3()) }
    pub fn from_chunk(pos: ChunkPos) -> Pos { Pos((pos.0 * CHUNK_SIZE_I).as_vec3()) }

    pub fn block_at(pos: f32) -> i32 { pos.floor() as i32 }
    pub fn chunk_at(pos: f32) -> i32 { BlockPos::chunk_at(Self::block_at(pos)) }
    pub fn chunk_relative_at(pos: f32) -> f32 {
        let world_pos = (Self::chunk_at(pos) * CHUNK_SIZE_I) as f32;
        return pos - world_pos
    }
}

#[derive(Component, Clone, Copy)]
pub struct Transform {
    matrix: Mat4,
    position: Pos,
    scale: Vec3,
    rotation: Quat
}

impl Transform {
    pub fn at(position: Pos) -> Transform {
        return Self::new(position, Quat::IDENTITY, Vec3::ONE)
    }

    pub fn new(position: Pos, rotation: Quat, scale: Vec3) -> Self {
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, position.0);
        Self {
            matrix,
            position,
            rotation,
            scale,
        }
    }

    pub fn identity() -> Self {
        Self {
            matrix: Mat4::IDENTITY,
            position: Vec3::ZERO.into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn translation(position: Pos) -> Self {
        let mut ret = Self::identity();
        ret.set_position(position.0);
        ret
    }

    pub fn lerp(&self, other: &Transform, t: f32) -> Self {
        let position = self.position.lerp(other.position.0, t);
        let rotation = self.rotation.lerp(other.rotation, t);
        let scale = self.scale.lerp(other.scale, t);
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self {
            matrix,
            position: position.into(),
            rotation,
            scale,
        }
    }

    pub fn matrix(&self) -> Mat4 {
        self.matrix
    }

    pub fn set_matrix(&mut self, new_matrix: Mat4) {
        self.matrix = new_matrix;
    }


    pub fn position(&self) -> Pos {
        self.position
    }

    pub fn set_position(&mut self, new_position: Vec3) {
        self.position = new_position.into();
        self.matrix = Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            new_position,
        );
    }

    pub fn scale(&self) -> Vec3 {
        self.scale
    }

    pub fn set_scale(&mut self, new_scale: Vec3) {
        self.scale = new_scale;
        self.matrix = Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.position.0,
        );
    }

    pub fn rotation(&self) -> Quat {
        self.rotation
    }

    pub fn set_rotation(&mut self, new_rotation: Quat) {
        self.rotation = new_rotation;
        self.matrix = Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.position.0,
        );
    }

    pub fn right(&self) -> Vec3 {
        self.matrix.x_axis.truncate()
    }

    pub fn up(&self) -> Vec3 {
        self.matrix.y_axis.truncate()
    }

    pub fn forward(&self) -> Vec3 {
        self.matrix.z_axis.truncate()
    }

    pub fn set_basis(&mut self, right: &Vec3, up: &Vec3, forward: &Vec3) {
        self.matrix.x_axis = right.extend(0.0);
        self.matrix.y_axis = up.extend(0.0);
        self.matrix.z_axis = forward.extend(0.0);
        self.update_pos_scale_rotation_from_matrix();
    }

    fn update_pos_scale_rotation_from_matrix(&mut self) {
        // Extranct the position from the matrix
        self.position = (self.matrix * Vec4::W).truncate().into();

        // Extract the scale and rotation from the matrix
        let scale = Vec3::new(
            self.matrix.x_axis.length(),
            self.matrix.y_axis.length(),
            self.matrix.z_axis.length(),
        );
        self.scale = scale;
        let rotation_matrix = Mat4::from_cols(
            Vec4::new(
                self.matrix.x_axis.x / scale.x,
                self.matrix.x_axis.y / scale.y,
                self.matrix.x_axis.z / scale.z,
                0.0,
            ),
            Vec4::new(
                self.matrix.y_axis.x / scale.x,
                self.matrix.y_axis.y / scale.y,
                self.matrix.y_axis.z / scale.z,
                0.0,
            ),
            Vec4::new(
                self.matrix.z_axis.x / scale.x,
                self.matrix.z_axis.y / scale.y,
                self.matrix.z_axis.z / scale.z,
                0.0,
            ),
            Vec4::W,
        );
        self.rotation = Quat::from_mat4(&rotation_matrix);
    }
}