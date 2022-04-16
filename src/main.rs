use std::cell::{Ref, RefCell, RefMut};

use glam::Vec2;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use engine::gamesync::GameSync;

use crate::engine::object::gameobject::{Mesh, Transform};
use crate::engine::renderer::graphic_object::GraphicObjectDesc;
use crate::engine::renderer::options::GraphicOptions;
use crate::engine::renderer::primitives::generate_circle_vertices;
use crate::engine::renderer::renderer::{GraphicEngine, Renderer};

mod engine;

trait GameEngine {
    fn graphics(&self) -> Ref<GraphicEngine>;
    fn graphics_mut(&self) -> RefMut<GraphicEngine>;

    fn create() -> (GameData, EventLoop<()>);
}

struct GameData {
    graphics: RefCell<GraphicEngine>,
}

impl GameEngine for GameData {
    fn graphics(&self) -> Ref<GraphicEngine> {
        return self.graphics.borrow();
    }

    fn graphics_mut(&self) -> RefMut<GraphicEngine> {
        return self.graphics.borrow_mut();
    }

    fn create() -> (GameData, EventLoop<()>) {
        let mut event_loop = EventLoop::new();
        let options = GraphicOptions::default();
        let mut renderer: GraphicEngine = Renderer::init(options, &event_loop);
        return (GameData {
            graphics: RefCell::new(renderer)
        }, event_loop);
    }
}

fn main() {
    let (mut game, event_loop) = GameData::create();// how?

    // TODO: should just start a level that will handle creation of stuff
    game.graphics_mut().create_graphic_object(GraphicObjectDesc {
        transform: Transform::at(Vec2::new(0.2, -0.2)),
        mesh: Mesh {
            vertices: generate_circle_vertices(200, 0.05, Vec2::new(0.0, 0.0))
        },
    });

    // renderer.render_loop_lazy_test(&mut event_loop);

    event_loop.run(move |event, _, control_flow| {
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
                let mut renderer = game.graphics_mut();
                let translated = renderer.translate_position(Vec2::new(position.x as f32, position.y as f32));
                renderer.move_object(0, translated);
                // mouse_pos = position;
            }
            Event::MainEventsCleared => {}
            Event::RedrawEventsCleared => {
                let mut renderer = game.graphics_mut();
                renderer.validate();
                renderer.render();
            }
            _ => (),
        }
    });
}
