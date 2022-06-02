#![windows_subsystem = "windows"]

use std::cell::{Ref, RefCell, RefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_ecs::world::World;

use glam::{Quat, Vec2, Vec3};
use winit::dpi::Position;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use engine::gamesync::GameSync;

use crate::engine::input::{Input, InputSystem};
use crate::engine::object::gameobject::{Camera, Mesh, RenderId, Transform, Velocity};
use crate::engine::renderer::graphic_object::GraphicObjectDesc;
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::primitives::generate_circle_mesh;
use crate::engine::renderer::renderer::{GraphicEngine, Renderer};

mod engine;

trait GameEngine {
    fn graphics(&self) -> Ref<GraphicEngine>;
    fn graphics_mut(&self) -> RefMut<GraphicEngine>;

    fn create() -> (GameData, EventLoop<()>);

    fn send_event<'a, T>(&mut self, winit_event: Event<'a, T>) -> Event<'a, T>;
}

struct GameData {
    graphics: RefCell<GraphicEngine>,
    input: InputSystem,
}

impl GameEngine for GameData {
    fn graphics(&self) -> Ref<GraphicEngine> {
        return self.graphics.borrow();
    }

    fn graphics_mut(&self) -> RefMut<GraphicEngine> {
        return self.graphics.borrow_mut();
    }

    fn create() -> (GameData, EventLoop<()>) {
        let event_loop = EventLoop::new();
        let options = GraphicOptions::default();
        let renderer: GraphicEngine = Renderer::init(options, &event_loop);
        return (GameData {
            graphics: RefCell::new(renderer),
            input: Input::create(),
        }, event_loop);
    }

    fn send_event<'a, T>(&mut self, winit_event: Event<'a, T>) -> Event<'a, T> {
        self.input.send_event(winit_event)
    }
}

fn update_camera(query: Query<(&Camera, &Transform)>, game_res: NonSendMut<GameData>) {
    for (camera, transform) in query.iter() {
        game_res.graphics.borrow_mut().camera.camera = *camera;
        game_res.graphics.borrow_mut().camera.transform = *transform;
    }
}

fn update_input_old(mut query: Query<(&mut Transform, &RenderId)>, game_res: NonSendMut<GameData>) {
    let input = game_res.input.get_move();
    if (input != Vec2::ZERO) {
        let mut renderer = game_res.graphics_mut();
        for (mut transform, id) in query.iter_mut() {
            transform.position += (input / 100.0).extend(0.0);
            renderer.move_object(id.id, transform.position);
        }
    }
}

fn update_input(mut query: Query<(&mut Transform, &Camera)>, game_res: NonSendMut<GameData>) {
    let input = game_res.input.get_move();
    if input != Vec2::ZERO {
        for (mut transform, _) in query.iter_mut() {
            transform.position += (input / 100.0).extend(0.0);
        }
    }
}

fn update_render(game_res: NonSendMut<GameData>) {
    let mut renderer = game_res.graphics_mut();
    renderer.validate();
    renderer.render();
}

fn main() {
    let (mut game, event_loop) = GameData::create();// how?

    let mut world = World::default();
    // Create a new Schedule, which defines an execution strategy for Systems
    let mut schedule = Schedule::default();
    let mesh = generate_circle_mesh(0, 200, 0.005);
    let mut x = -1.0;
    let mut y = -1.0;
    for i in 0..1000 {
        if (x < 1.0) {
            x += 0.01;
        } else {
            x = -1.0;
            y += 0.01;
        }
        world.spawn()
            .insert(Transform {
                position: Vec3::new(x, y, 0.0),
                scale: Vec3::ONE,
                rotation: Quat::IDENTITY,
            })
            .insert(mesh.clone())
            .insert(RenderId { id: 0 });
    }

    world.spawn()
        .insert(Transform {
            position: Vec3::new(0.0, 0.0, -1.0),
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY.inverse(),
        })
        .insert(Camera {
            far_clip_plane: 200.0,
            near_clip_plane: 0.1,
            field_of_view: 90.0,
        });

    schedule.add_stage("basic_stage", SystemStage::single_threaded()
        .with_system(update_camera)
        .with_system(update_render)
        .with_system(update_input),
    );


    let material = game.graphics.borrow().materials.borrow().get_default();

    let mut state: SystemState<Query<(&mut Transform, &Mesh, &mut RenderId)>> = SystemState::from_world(&mut world);
    let mut query = state.get_mut(&mut world);
    for (mut transform, mesh, mut id) in query.iter_mut() {
        // TODO: should just start a level that will handle creation of stuff
        id.id = game.graphics_mut().create_graphic_object(GraphicObjectDesc {
            transform: transform.clone(),
            mesh: mesh.clone(),
            material,
        });
    }

    // renderer.render_loop_lazy_test(&mut event_loop);

    world.insert_non_send_resource(game);
    // TODO: i guess i need another thread that will run this loop?
    event_loop.run(move |event, _, control_flow| {
        let mut game: Mut<GameData> = world.get_non_send_resource_mut().unwrap();
        let event = game.send_event(event);
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                game.graphics_mut().on_resize(new_size);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                // this needs to go
                // let mut renderer = game.graphics_mut();
                // let translated = renderer.translate_position(Vec2::new(position.x as f32, position.y as f32));
                // renderer.move_object(0, translated);
                // mouse_pos = position;
            }
            Event::RedrawEventsCleared => {
                schedule.run(&mut world);
            }
            _ => (),
        }
    });
}
