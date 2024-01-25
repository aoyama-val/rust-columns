use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 6;
pub const FIELD_H: usize = 13;
pub const CELL_SIZE: i32 = 40;
pub const COLOR_COUNT: i32 = 6;
pub const BLOCK_LEN: usize = 3;
pub const FALL_WAIT: i32 = 30;
pub const SPAWN_WAIT: i32 = 30;
pub const EMPTY: i32 = 0;

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
    pub fall_frame: i32,
    pub spawn_wait: i32,
    pub controllable: bool,
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
            field: [[EMPTY; FIELD_W]; FIELD_H],
            current: [1; BLOCK_LEN],
            current_x: 0,
            current_y: 0,
            next: [0; BLOCK_LEN],
            erased_jewels: 0,
            max_combo: 0,
            fall_frame: FALL_WAIT,
            spawn_wait: -1,
            controllable: true,
        };

        for i in 0..BLOCK_LEN {
            game.next[i] = game.rng.gen_range(1..=COLOR_COUNT)
        }
        game.spawn();

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

        if self.controllable {
            match command {
                Command::Left => {
                    self.move_block(-1);
                }
                Command::Right => {
                    self.move_block(1);
                    if self.current_x + 1 < FIELD_W {
                        self.current_x += 1;
                        if self.is_collide() {
                            self.current_x -= 1;
                        }
                    }
                }
                Command::Down => {
                    self.fall_frame = self.frame;
                }
                Command::Rotate => {
                    self.rotate();
                }
                Command::None => {}
            }

            self.fall();
        } else {
            if self.spawn_wait > 0 {
                self.spawn_wait -= 1;
            }
            if self.spawn_wait == 0 {
                self.spawn();
                if self.is_collide() {
                    self.is_over = true;
                    self.requested_sounds.push("crash.wav");
                }
                self.spawn_wait = -1;
                // 足場がなくなったピースを落とす（アニメーション）
                // そろったピースを消す（アニメーション）
                self.controllable = true;
            }
        }
    }

    pub fn move_block(&mut self, dir: i32) {
        if dir == -1 && self.current_x == 0 {
            return;
        }
        if dir == 1 && self.current_x == FIELD_W - 1 {
            return;
        }
        self.current_x = (self.current_x as i32 + dir) as usize;
        if self.is_collide() {
            self.current_x = (self.current_x as i32 - dir) as usize;
        }
    }

    pub fn rotate(&mut self) {
        let tmp = self.current[BLOCK_LEN - 1];
        for i in 1..=(BLOCK_LEN - 1) {
            self.current[i] = self.current[i - 1];
        }
        self.current[0] = tmp;
    }

    pub fn fall(&mut self) {
        if self.frame == self.fall_frame {
            self.current_y += 1;
            if self.is_collide() {
                self.current_y -= 1;
                self.settle();
                self.spawn_wait = SPAWN_WAIT;
                self.controllable = false;
            }
            self.fall_frame = self.frame + FALL_WAIT;
        }
    }

    pub fn is_collide(&self) -> bool {
        let bottom_y = self.current_y + (BLOCK_LEN - 1);
        if bottom_y == FIELD_H {
            return true;
        }
        if self.field[bottom_y][self.current_x] != EMPTY {
            return true;
        }
        return false;
    }

    pub fn settle(&mut self) {
        for i in 0..BLOCK_LEN {
            self.field[self.current_y + i][self.current_x] = self.current[i];
        }
        self.requested_sounds.push("hit.wav");
    }

    pub fn spawn(&mut self) {
        self.current = self.next;
        self.current_x = FIELD_W / 2;
        self.current_y = 0;
        for i in 0..BLOCK_LEN {
            self.next[i] = self.rng.gen_range(1..=COLOR_COUNT)
        }
    }
}
