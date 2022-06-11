#![windows_subsystem = "windows"]

use std::cell::{Ref, RefCell, RefMut};
use std::f32::consts::PI;
use std::ops::Mul;
use std::sync::Arc;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
use glam::{EulerRot, Quat, Vec2, Vec3};
use vulkano::pipeline::graphics::viewport::Viewport;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};

use engine::gamesync::GameSync;
use game_loop::helper::game_loop;
use crate::engine::input;

use crate::engine::input::{Input, InputSystem};
use crate::engine::object::gameobject::{Camera, Mesh, RenderId, Velocity};
use crate::engine::object::transform::Transform;
use crate::engine::renderer::graphic_object::GraphicObjectDesc;
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::renderer::{GraphicEngine, Renderer, Vertex};
use crate::input::{ASCEND, ROTATE};

mod engine;
mod game_loop;

fn update_camera(mut query: Query<(&mut Camera, &Transform)>, mut renderer: NonSendMut<GraphicEngine>) {
    for (mut camera, transform) in query.iter_mut() {
        let size = renderer.surface.window().inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;
        camera.aspect_ratio = aspect_ratio;

        let position: Vec3 = transform.position();
        let mut rotation: Vec3 = transform.rotation().to_euler(EulerRot::YXZ).into();
        rotation = rotation * (180.0 / PI);
        let str = format!("{}, {}", position, rotation);
        renderer.surface.window().set_title(str.as_str());
        renderer.camera.camera = *camera;
        renderer.camera.transform = *transform;
        renderer.camera.update();
    }
}

fn update_input(mut query: Query<(&mut Transform, &Camera)>, input_sys: NonSend<InputSystem>) {
    // TODO: doing this in system seems wrong
    let ascend = input_sys.get_axis(ASCEND);
    let rotate = input_sys.get_axis3d(ROTATE) * 0.005;
    let input = input_sys.get_move();
    let camera_rot = input_sys.get_mouse_move();
    let factor = ((PI * 2.0) / 360.0) * 0.02;
    let ang_x = camera_rot.x * factor;
    let ang_y = camera_rot.y * factor;
    for (mut transform, _) in query.iter_mut() {
        let mut transform: Mut<Transform> = transform;
        let current = transform.rotation();

        // does not work when rolled by 180*
        let horizontal = Quat::from_xyzw(0.0, ang_x.sin(), 0.0, ang_x.cos());
        let vertical = Quat::from_xyzw(ang_y.sin(), 0.0, 0.0, ang_y.cos());
        let quat = current * horizontal  * vertical;

        let mut position = transform.position();
        let forward = transform.forward() * input.y;
        position = (position + forward * 0.05);
        let right = transform.right() * input.x;
        position = (position + right * 0.05);
        let ascend = transform.up() * ascend;
        position = (position + ascend * 0.05);
        transform.set_position(position);

        let rot = Quat::from_euler(EulerRot::YXZ, rotate.x, rotate.y, rotate.z);
        transform.set_rotation(quat * rot)
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = Arc::new(WindowBuilder::new()
        .with_transparent(true)
        // .with_decorations(false)
        .with_resizable(true)
        .with_min_inner_size(PhysicalSize::new(1600, 1200))
        .with_title("Game or something idk yet")
        .build(&event_loop).unwrap());
    window.set_cursor_grab(true);
    window.set_cursor_visible(false);
    let options = GraphicOptions::default();
    let mut renderer: GraphicEngine = Renderer::init(options, window.clone());
    let mut world = World::default();
    let mut scheduler = Schedule::default();
    let mut input = InputSystem::create();
    // let graphics = RefCell::new(renderer);
    world.insert_non_send_resource(input);

    let scenes = easy_gltf::load("resources/torus.glb").unwrap();
    for scene in scenes {
        for model in scene.models {
            let mesh = Mesh {
                id: 0,
                vertices: model.vertices().iter().map(|x| Vertex { position: x.position.into(), normal: x.normal.into() }).collect::<Vec<_>>(),
                indices: model.indices().unwrap().iter().map(|x| *x as u16).collect::<Vec<_>>(),
            };
            world.spawn()
                .insert(Transform::new(
                    Vec3::new(3.0, 0.0, 10.0),
                    Quat::IDENTITY,
                    Vec3::ONE,
                ))
                .insert(mesh.clone())
                .insert(RenderId { id: 0 });
        }
    }

    let size = window.inner_size();
    let aspect_ratio = size.width as f32 / size.height as f32;
    world.spawn()
        .insert(Transform::new(
            Vec3::new(0.0, 0.0, 0.0),
            Quat::IDENTITY,
            Vec3::ONE,
        ))
        .insert(Camera {
            aspect_ratio: aspect_ratio,
            far_clip_plane: 200.0,
            near_clip_plane: 0.05,
            field_of_view: 70.0,
        });

    scheduler.add_stage("basic_stage", SystemStage::single_threaded()
        .with_system(update_camera)
        .with_system(update_input),
    );


    let material = renderer.materials.borrow().get_default();

    let mut state: SystemState<Query<(&mut Transform, &Mesh, &mut RenderId)>> = SystemState::from_world(&mut world);
    let mut query = state.get_mut(&mut world);
    for (mut transform, mesh, mut id) in query.iter_mut() {
        // TODO: should just start a level that will handle creation of stuff
        id.id = renderer.create_graphic_object(GraphicObjectDesc {
            transform: transform.clone(),
            mesh: mesh.clone(),
            material,
        });
    }

    world.insert_non_send_resource(renderer);


    game_loop(event_loop, window, world, 144, 0.5, move |g| {
        scheduler.run(&mut g.game);
        let mut input: Mut<InputSystem> = g.game.get_non_send_resource_mut().unwrap();
        input.send_end_frame_event()
    }, |g| {
        let mut renderer: Mut<GraphicEngine> = g.game.get_non_send_resource_mut().unwrap();
        renderer.validate();
        renderer.render();
    }, |g, event| {
        let mut input: Mut<InputSystem> = g.game.get_non_send_resource_mut().unwrap();
        input.send_event(event);
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                g.exit_next_iteration = true;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                let mut renderer: Mut<GraphicEngine> = g.game.get_non_send_resource_mut().unwrap();
                renderer.on_resize(new_size.clone());
            }
            _ => (),
        }
    });
}
