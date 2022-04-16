use std::cell::RefCell;
use legion::World;

trait Level {
    fn new() -> Self;
}

struct GameLevel {
    world: RefCell<World>
}

impl Level for GameLevel {
    fn new() -> Self {
        return GameLevel {
            // TODO: read about component groups
            world: RefCell::new(World::default())
        }
    }
}