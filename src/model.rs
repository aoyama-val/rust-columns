use rand::prelude::*;
use std::time;

pub const FPS: i32 = 30;
pub const WORLD_W: f32 = 600.0;
pub const WORLD_H: f32 = 600.0;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
}

pub struct Game {
    pub rng: StdRng,
    pub is_over: bool,
    pub requested_sounds: Vec<&'static str>,
}

impl Game {
    pub fn new() -> Self {
        let now = time::SystemTime::now();
        let timestamp = now
            .duration_since(time::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_secs();
        let rng = StdRng::seed_from_u64(timestamp);
        println!("random seed = {}", timestamp);
        // let rng = StdRng::seed_from_u64(0);

        let game = Game {
            rng: rng,
            is_over: false,
            requested_sounds: Vec::new(),
        };

        game
    }

    pub fn update(&mut self, command: Command) {
        if self.is_over {
            return;
        }

        match command {
            Command::None => {}
        }
    }
}
