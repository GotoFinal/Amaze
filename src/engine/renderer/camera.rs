use std::f32::consts::PI;
use glam::{Mat4, Quat, Vec3};
use crate::{Camera, Transform};

pub struct RendererCamera {
    pub transform: Transform,
    pub camera: Camera,
    old_camera: Camera,
    pub projection: Mat4,
    pub view: Mat4
}

impl RendererCamera {
    pub fn create(position: Vec3, camera: Camera) -> RendererCamera {
        return RendererCamera {
            transform: Transform::new(position, Quat::IDENTITY, Vec3::ONE),
            camera: camera,
            old_camera: camera,
            projection: Self::create_projection_matrix(camera.field_of_view, camera.aspect_ratio, camera.near_clip_plane, camera.far_clip_plane),
            view: Mat4::ZERO
        }
    }

    pub fn update(&mut self) {
        if self.camera != self.old_camera {
            self.old_camera = self.camera;
            self.projection = Self::create_projection_matrix(self.camera.field_of_view, self.camera.aspect_ratio, self.camera.near_clip_plane, self.camera.far_clip_plane)
        }

        // self.view = Mat4::look_at_rh(
        //     Vec3::new(0.0, 0.0, 0.0),
        //     self.transform.forward(),
        //     // camera.transform.up(),
        //     Vec3::new(0.0, 0.0, 1.0),
        // );
        // dbg!(self.transform.forward());
        self.view = self.transform.matrix().inverse();
    }

    fn create_projection_matrix(vertical_fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
        let mut perspective = Mat4::perspective_rh(vertical_fov.to_radians(), aspect_ratio, near, far);
        // perspective.y_axis.y *= -1.0;
        return perspective;

        let fov_rad = vertical_fov * 2.0 * PI / 360.0;
        let focal_length = 1.0 / (fov_rad / 2.0).tan();

        let x = focal_length / aspect_ratio;
        let y = -focal_length;
        let a = near / (far - near);
        let b = far * a;

        let mut projection = Mat4::ZERO;
        projection.row(0).x = x;
        projection.row(1).y = y;
        projection.row(2).z = a;
        projection.row(2).w = b;
        projection.row(3).y = -1.0; // or 1?
        // return Mat4::from_cols_array(
        //     &[
        //         x, 0.0, 0.0, 0.0,
        //         0.0, y, 0.0, 0.0,
        //         0.0, 0.0, a, b,
        //         0.0, 0.0, -1.0, 0.0
        //     ]
        // );
        projection
    }
}