use glam::{Mat4, Quat, Vec3, Vec4};
use bevy_ecs::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct Transform {
    matrix: Mat4,
    position: Vec3,
    scale: Vec3,
    rotation: Quat
}

impl Transform {
    pub fn at(position: Vec3) -> Transform {
        return Self::new(position, Quat::IDENTITY, Vec3::ONE)
    }

    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, position);
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
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn translation(position: Vec3) -> Self {
        let mut ret = Self::identity();
        ret.set_position(position);
        ret
    }

    pub fn lerp(&self, other: &Transform, t: f32) -> Self {
        let position = self.position.lerp(other.position, t);
        let rotation = self.rotation.lerp(other.rotation, t);
        let scale = self.scale.lerp(other.scale, t);
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, position);
        Self {
            matrix,
            position,
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


    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, new_position: Vec3) {
        self.position = new_position;
        self.matrix = Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.position,
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
            self.position,
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
            self.position,
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
        self.position = (self.matrix * Vec4::W).truncate();

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