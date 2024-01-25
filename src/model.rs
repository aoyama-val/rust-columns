use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 6;
pub const FIELD_H: usize = 13;
pub const CELL_SIZE: i32 = 40;
pub const COLOR_COUNT: i32 = 6;
pub const BLOCK_LEN: usize = 3;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
    Left,
    Right,
    Rotate,
    Down,
}

#[derive(Debug)]
pub struct Game {
    pub rng: StdRng,
    pub is_over: bool,
    pub is_debug: bool,
    pub frame: i32,
    pub requested_sounds: Vec<&'static str>,
    pub commands: Vec<Command>, // リプレイデータから読み込んだコマンド
    pub command_log: File,      // コマンドログ
    pub field: [[i32; FIELD_W]; FIELD_H],
    pub current_x: usize,
    pub current_y: usize,
    pub current: [i32; BLOCK_LEN],
    pub next: [i32; BLOCK_LEN],
    pub erased_jewels: i32,
    pub max_combo: i32,
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

        let mut game = Game {
            rng: rng,
            is_over: false,
            is_debug: false,
            frame: -1,
            requested_sounds: Vec::new(),
            commands: Vec::new(),
            command_log: File::create("command.log").unwrap(),
            field: [[0; FIELD_W]; FIELD_H],
            current: [0; BLOCK_LEN],
            current_x: 0,
            current_y: 0,
            next: [0; BLOCK_LEN],
            erased_jewels: 0,
            max_combo: 0,
        };

        for i in 0..BLOCK_LEN {
            game.current[i] = game.rng.gen_range(1..=COLOR_COUNT)
        }

        for i in 0..BLOCK_LEN {
            game.next[i] = game.rng.gen_range(1..=COLOR_COUNT)
        }

        // for y in 0..FIELD_H {
        //     for x in 0..FIELD_W {
        //         game.field[y][x] = game.rng.gen_range(1..=COLOR_COUNT);
        //     }
        // }

        game
    }

    pub fn toggle_debug(&mut self) {
        self.is_debug = !self.is_debug;
        println!("is_debug: {}", self.is_debug);
    }

    pub fn write_command_log(&mut self, command: Command) {
        self.command_log
            .write_all(format!("{:?}\n", command).as_bytes())
            .ok();
        self.command_log.flush().ok();
    }

    pub fn update(&mut self, mut command: Command) {
        self.frame += 1;

        if self.commands.len() > self.frame as usize {
            command = self.commands[self.frame as usize];
        }
        self.write_command_log(command);

        if self.is_over {
            return;
        }
    }
}
