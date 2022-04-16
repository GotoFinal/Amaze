mod engine;

use glam::{IVec2, Vec2};
use engine::gamesync::GameSync;
use engine::renderer::{Multisampling, Buffering, GraphicObject};
use engine::renderer::{Renderer, GraphicOptions, GraphicObjectDesc, Vertex, GraphicEngine};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::engine::renderer::primitives::generate_circle_vertices;

fn main() {
    // that was supposed to be code split, not move...

    let options = GraphicOptions {
        multisampling: Multisampling::Sample2,
        buffering: Buffering::Triple
    };

    let mut event_loop = EventLoop::new();
    let mut renderer: GraphicEngine = Renderer::init(options, &mut event_loop); // does that work? i think it will just copy the struct so i need to make it pointer of sort
    let mut object = renderer.create_graphic_object(GraphicObjectDesc {
        position: Vec2::new(0.2, -0.2),
        vertices: generate_circle_vertices(200, 0.05, Vec2::new(0.0, 0.0))
    });

    // renderer.render_loop_lazy_test(&mut event_loop);

    
    let frames_in_flight = renderer.buffer_size();
    // TODO: handle buffer size change?
    let mut sync: GameSync = GameSync::new(frames_in_flight);

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
                renderer.on_resize(new_size);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let translated = renderer.translate_position(Vec2::new(position.x as f32, position.y as f32));
                renderer.move_object(0, translated);
                // mouse_pos = position;
            }
            Event::MainEventsCleared => {}
            Event::RedrawEventsCleared => {
                renderer.validate();
                sync.get_current();
                renderer.render(&mut sync);
            }
            _ => (),
        }
    });
}
