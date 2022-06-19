use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use crate::ChunkGenerator;
use crate::engine::terrarin::chunk::{Chunk, ChunkPos};

pub struct GameWorld {
    generator: Box<dyn ChunkGenerator>,
    chunks: HashMap<ChunkPos, RefCell<Chunk>>,
}

impl GameWorld {
    pub fn new(generator: Box<dyn ChunkGenerator>) -> GameWorld {
        return GameWorld {
            generator,
            chunks: HashMap::new(),
        };
    }

    pub fn chunk_at(&mut self, pos: ChunkPos) -> Ref<Chunk> {
        if !self.chunks.contains_key(&pos) {
            let generated = self.generator.generate_chunk(self, pos);
            self.chunks.insert(pos, RefCell::new(generated));
        }
        return self.chunks[&pos].borrow();
    }
}