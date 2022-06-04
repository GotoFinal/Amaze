#![windows_subsystem = "windows"]

use std::cell::{Ref, RefCell, RefMut};
use std::f32::consts::PI;
use std::ops::Mul;
use std::sync::Arc;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;
use glam::{Quat, Vec2, Vec3};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit::dpi::PhysicalSize;

use engine::gamesync::GameSync;
use game_loop::helper::game_loop;

use crate::engine::input::{Input, InputSystem};
use crate::engine::object::gameobject::{Camera, Mesh, RenderId, Velocity};
use crate::engine::object::transform::Transform;
use crate::engine::renderer::graphic_object::GraphicObjectDesc;
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::renderer::{GraphicEngine, Renderer, Vertex};

mod engine;
mod game_loop;

fn update_camera(query: Query<(&Camera, &Transform)>, mut renderer: NonSendMut<GraphicEngine>) {
    for (camera, transform) in query.iter() {
        renderer.camera.camera = *camera;
        renderer.camera.transform = *transform;
    }
}

fn update_input(mut query: Query<(&mut Transform, &Camera)>, input_sys: NonSend<InputSystem>) {
    // TODO: doing this in system seems wrong
    let input = input_sys.get_move();
    let camera_rot = input_sys.get_mouse_move();
    let factor = ((PI * 2.0) / 360.0) * 0.05;
    let ang_x = camera_rot.x * factor;
    let ang_y = camera_rot.y * factor;
    for (mut transform, _) in query.iter_mut() {
        let mut transform: Mut<Transform> = transform;
        let current = transform.rotation();
        let horizontal = Quat::from_xyzw(0.0, ang_x.sin(), 0.0, ang_x.cos());
        let vertical = Quat::from_xyzw(ang_y.sin(), 0.0, 0.0, ang_y.cos());
        transform.set_rotation(horizontal * current * vertical);

        //  transform.forward(); TODO
        let position = transform.position();
        transform.set_position(position + (input / 100.0).extend(0.0));
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = Arc::new(WindowBuilder::new()
        .with_transparent(true)
        // .with_decorations(false)
        .with_resizable(true)
        .with_min_inner_size(PhysicalSize::new(400, 400))
        .with_title("Game or something idk yet")
        .build(&event_loop).unwrap());
    let options = GraphicOptions::default();
    let mut renderer: GraphicEngine = Renderer::init(options, window.clone());
    let mut world = World::default();
    let mut scheduler = Schedule::default();
    let mut input = InputSystem::create();
    // let graphics = RefCell::new(renderer);
    world.insert_non_send_resource(input);

    let scenes = easy_gltf::load("resources/cube.glb").unwrap();
    for scene in scenes {
        for model in scene.models {
            world.spawn()
                .insert(Transform::new(
                    Vec3::new(0.5, 0.0, 0.0),
                    Quat::IDENTITY,
                    Vec3::ONE,
                ))
                .insert(Mesh {
                    id: 0,
                    vertices: model.vertices().iter().map(|x| Vertex { position: x.position.into(), normal: x.normal.into() }).collect::<Vec<_>>(),
                    indices: model.indices().unwrap().iter().map(|x| *x as u16).collect::<Vec<_>>(),
                })
                .insert(RenderId { id: 0 });
        }
    }

    world.spawn()
        .insert(Transform::new(
            Vec3::new(0.0, 0.0, -1.0),
            Quat::IDENTITY,
            Vec3::ONE,
        ))
        .insert(Camera {
            far_clip_plane: 200.0,
            near_clip_plane: 0.1,
            field_of_view: 90.0,
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
