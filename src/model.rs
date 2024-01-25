use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 6;
pub const FIELD_H: usize = 15;
pub const INVISIBLE_ROW_COUNT: usize = 3;
pub const CELL_SIZE: i32 = 40;
pub const COLOR_COUNT: i32 = 6;
pub const BLOCK_LEN: usize = 3; // 1ブロックのピース数
pub const ERASE_LEN: usize = 3; // この個数つながったら消す
pub const FALL_WAIT: i32 = 30;
pub const FLASHING_WAIT: i32 = 15;
pub const PIECE_FALL_WAIT: i32 = 4;
pub const EMPTY: i32 = 0;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
    Left,
    Right,
    Rotate,
    Down,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    Controllable, // 操作可能な状態
    Flashing,     // そろったピースが点滅している状態
    PieceFalling, // 足場がなくなったピースが落下している状態
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
    pub state: State,
    pub field: [[i32; FIELD_W]; FIELD_H],
    pub check_erase_result: [[bool; FIELD_W]; FIELD_H],
    pub current_x: usize,
    pub current_y: usize,
    pub current: [i32; BLOCK_LEN],
    pub next: [i32; BLOCK_LEN],
    pub erase: i32,
    pub max_erase: i32,
    pub combo: i32,
    pub total_erased: i32,
    pub max_combo: i32,
    pub fall_wait: i32,
    pub spawn_wait: i32,
    pub flashing_wait: i32,
    pub piece_fall_wait: i32,
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
            state: State::Controllable,
            field: [[EMPTY; FIELD_W]; FIELD_H],
            check_erase_result: [[false; FIELD_W]; FIELD_H],
            current: [1; BLOCK_LEN],
            current_x: 0,
            current_y: 0,
            next: [0; BLOCK_LEN],
            erase: 0,
            max_erase: 0,
            combo: -1,
            total_erased: 0,
            max_combo: 0,
            fall_wait: FALL_WAIT,
            spawn_wait: -1,
            flashing_wait: -1,
            piece_fall_wait: -1,
        };

        for i in 0..BLOCK_LEN {
            game.next[i] = game.rng.gen_range(1..=COLOR_COUNT)
        }
        game.spawn();

        game.current = [5, 5, 1];
        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 5, 0, 2],
            [0, 0, 0, 2, 0, 3],
            [0, 0, 0, 2, 0, 3],
            [0, 0, 1, 1, 0, 1],
            [0, 0, 4, 2, 6, 6],
            [0, 0, 4, 5, 6, 6],
            [0, 0, 3, 5, 1, 4],
        ];

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

        if self.state == State::Controllable {
            self.fall();

            match command {
                Command::Left => {
                    self.move_block(-1);
                }
                Command::Right => {
                    self.move_block(1);
                }
                Command::Down => {
                    self.fall_wait = 0;
                }
                Command::Rotate => {
                    self.rotate();
                }
                Command::None => {}
            }
        } else if self.state == State::Flashing {
            if self.flashing_wait > 0 {
                self.flashing_wait -= 1;
            }
            if self.flashing_wait == 0 {
                self.actually_erase();
                self.set_state(State::PieceFalling);
            }
        } else if self.state == State::PieceFalling {
            if self.piece_fall_wait > 0 {
                self.piece_fall_wait -= 1;
            }
            if self.piece_fall_wait == 0 {
                self.piece_fall();
                if self.check_erase() {
                    self.set_state(State::Flashing);
                } else {
                    self.set_state(State::Controllable);
                }
            }
        }
    }

    pub fn set_state(&mut self, new_state: State) {
        println!("state: {:?} -> {:?}", self.state, new_state);
        match new_state {
            State::Controllable => {
                assert!(self.state == State::Controllable || self.state == State::PieceFalling);
                self.erase = 0;
                self.combo = -1;
                self.spawn();
                if self.is_collide() {
                    self.is_over = true;
                    self.requested_sounds.push("crash.wav");
                }
            }
            State::Flashing => {
                assert!(self.state == State::Controllable || self.state == State::PieceFalling);

                self.combo += 1;
                self.flashing_wait = FLASHING_WAIT;
            }
            State::PieceFalling => {
                assert!(self.state == State::Flashing);
                self.piece_fall_wait = PIECE_FALL_WAIT;
            }
        }
        self.state = new_state;
    }

    pub fn piece_fall(&mut self) {
        for y in (0..FIELD_H).rev() {
            for x in 0..FIELD_W {
                if self.field[y][x] == EMPTY {
                    for y2 in (0..y).rev() {
                        if self.field[y2][x] != EMPTY {
                            self.field[y][x] = self.field[y2][x];
                            self.field[y2][x] = EMPTY;
                            break;
                        }
                    }
                }
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
        for i in (1..=(BLOCK_LEN - 1)).rev() {
            self.current[i] = self.current[i - 1];
        }
        self.current[0] = tmp;
    }

    pub fn fall(&mut self) {
        if self.fall_wait > 0 {
            self.fall_wait -= 1;
        }
        if self.fall_wait == 0 {
            self.current_y += 1;
            if self.is_collide() {
                self.current_y -= 1;
                self.settle();
                if self.check_erase() {
                    self.set_state(State::Flashing);
                } else {
                    self.set_state(State::Controllable);
                }
            }
            self.fall_wait = FALL_WAIT;
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

    pub fn check_erase(&mut self) -> bool {
        self.check_erase_result = Default::default();
        let mut exist = false;

        for y in 0..FIELD_H {
            for x in 0..FIELD_W {
                if self.field[y][x] != EMPTY {
                    let dirs: [(i32, i32); 4] = [(1, 0), (1, 1), (0, 1), (1, -1)];
                    for dir in dirs {
                        let mut is_same = true;
                        for i in 1..ERASE_LEN {
                            let x_ = x as i32 + dir.0 * i as i32;
                            let y_ = y as i32 + dir.1 * i as i32;
                            if !self.is_piece_exist(x_, y_)
                                || self.field[y_ as usize][x_ as usize] != self.field[y][x]
                            {
                                is_same = false;
                                break;
                            }
                        }
                        if is_same {
                            for i in 0..ERASE_LEN {
                                let x_ = x as i32 + dir.0 * i as i32;
                                let y_ = y as i32 + dir.1 * i as i32;
                                self.check_erase_result[y_ as usize][x_ as usize] = true;
                                exist = true;
                            }
                        }
                    }
                }
            }
        }
        return exist;
    }

    pub fn actually_erase(&mut self) {
        let mut erased_count: i32 = 0;
        for y in 0..FIELD_H {
            for x in 0..FIELD_W {
                if self.check_erase_result[y][x] {
                    self.field[y][x] = EMPTY;
                    erased_count += 1;
                }
            }
        }
        if erased_count > 0 {
            self.total_erased += erased_count;
            self.erase += erased_count;
            if self.max_erase < self.erase {
                self.max_erase = self.erase;
            }
            if self.max_combo < self.combo {
                self.max_combo = self.combo;
            }
            self.requested_sounds.push("erase.wav");
        }
    }

    pub fn is_piece_exist(&self, x: i32, y: i32) -> bool {
        0 <= x
            && x < FIELD_W as i32
            && 0 <= y
            && y < FIELD_H as i32
            && self.field[y as usize][x as usize] != EMPTY
    }

    pub fn spawn(&mut self) {
        println!("Spawn!");
        self.current = self.next;
        self.current_x = FIELD_W / 2;
        self.current_y = 0;
        for i in 0..BLOCK_LEN {
            self.next[i] = self.rng.gen_range(1..=COLOR_COUNT)
        }
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_check_erase1() {
        let mut game = Game::new();

        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 1, 1, 1, 1],
        ];
        game.check_erase();
        assert_eq!(
            game.check_erase_result,
            [
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false,  true,  true,  true,  true],
            ]
        );
    }
    
    #[test]
    fn test_check_erase2() {
        let mut game = Game::new();

        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
        ];
        game.check_erase();
        assert_eq!(
            game.check_erase_result,
            [
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false,  true],
                [false, false, false, false, false,  true],
                [false, false, false, false, false,  true],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
            ]
        );
    }
    
    #[test]
    fn test_check_erase3() {
        let mut game = Game::new();

        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 1, 0, 0],
            [0, 0, 0, 0, 1, 0],
            [0, 0, 0, 0, 0, 1],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
        ];
        game.check_erase();
        assert_eq!(
            game.check_erase_result,
            [
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false,  true, false, false],
                [false, false, false, false,  true, false],
                [false, false, false, false, false,  true],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
            ]
        );
    }

    #[test]
    fn test_check_erase4() {
        let mut game = Game::new();

        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 1, 0, 0, 0],
            [0, 0, 0, 1, 0, 0],
            [0, 0, 0, 0, 1, 0],
            [0, 0, 1, 1, 1, 1],
        ];
        game.check_erase();
        assert_eq!(
            game.check_erase_result,
            [
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false,  true, false, false, false],
                [false, false, false,  true, false, false],
                [false, false, false, false,  true, false],
                [false, false,  true,  true,  true,  true],
            ]
        );
    }

    #[test]
    fn test_check_erase5() {
        let mut game = Game::new();

        game.field = [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 1, 0],
            [0, 0, 0, 1, 0, 0],
            [0, 0, 1, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
        ];
        game.check_erase();
        assert_eq!(
            game.check_erase_result,
            [
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false,  true, false],
                [false, false, false,  true, false, false],
                [false, false,  true, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
                [false, false, false, false, false, false],
            ]
        );
    }
}
