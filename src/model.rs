use rand::prelude::*;
use std::{fs::File, io::Write, time};

pub const FPS: i32 = 30;
pub const FIELD_W: usize = 6;
pub const FIELD_H: usize = 16;
pub const INVISIBLE_ROW_COUNT: usize = 3;
pub const CELL_SIZE: i32 = 40;
pub const COLOR_COUNT: i32 = 6;
pub const BLOCK_LEN: usize = 3; // 1ブロックのピース数
pub const ERASE_LEN: usize = 3; // この個数つながったら消す
pub const FALL_WAIT: i32 = 30;
pub const FLASHING_WAIT: i32 = 15;
pub const PIECE_FALL_SPEED: i32 = 15;
pub const EMPTY: i32 = 0;

// $varの値が
//   > 0 : ウェイト中
//  == 0 : ブロック実行
//   < 0 : ブロック実行せず、ウェイトも減らさない
macro_rules! wait {
    ($var:expr, $block:block) => {
        if $var > 0 {
            $var -= 1;
        }
        if $var == 0 {
            $block
        }
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Command {
    None,
    Left,
    Right,
    Rotate,
    Down,
}

impl Command {
    pub fn from_str(str: &str) -> Command {
        match str {
            "None" => Command::None,
            "Left" => Command::Left,
            "Right" => Command::Right,
            "Rotate" => Command::Rotate,
            "Down" => Command::Down,
            _ => Command::None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum State {
    #[default]
    Controllable, // 操作可能な状態
    Flashing,     // そろったピースが点滅している状態
    PieceFalling, // 足場がなくなったピースが落下している状態
}

#[derive(Debug, Default)]
pub struct Game {
    pub rng: Option<StdRng>,
    pub is_over: bool,
    pub is_debug: bool,
    pub frame: i32,
    pub requested_sounds: Vec<&'static str>,
    pub commands: Vec<Command>,    // リプレイデータから読み込んだコマンド
    pub command_log: Option<File>, // コマンドログ
    pub replay_loaded: bool,
    pub state: State,
    pub field: [[i32; FIELD_W]; FIELD_H],
    pub check_erase_result: [[bool; FIELD_W]; FIELD_H],
    pub piece_falling: [[bool; FIELD_W]; FIELD_H],
    pub current_x: usize,
    pub current_y: usize, // 3個つらなっている一番上のピースの座標
    pub current: [i32; BLOCK_LEN],
    pub next: [i32; BLOCK_LEN],
    pub next_to_display: [i32; BLOCK_LEN],
    pub erased_one_time: i32, // 連鎖も含めていっぺんに消した個数
    pub max_erased_at_one_time: i32,
    pub combo: i32, // 現在進行中のコンボ数
    pub total_erased: i32,
    pub max_combo: i32,
    pub fall_wait: i32,
    pub spawn_wait: i32,
    pub flashing_wait: i32,
    pub piece_fall_wait: i32,
    pub piece_fall_offset: i32,
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
        //let rng = StdRng::seed_from_u64(1706226338);

        let mut game = Game {
            rng: Some(rng),
            command_log: Some(File::create("command.log").unwrap()),
            frame: -1,
            ..Default::default()
        };

        game.set_state(State::Controllable);
        game.spawn();
        game.spawn();
        game.next_to_display = game.next;

        //game.current = [5, 5, 4];
        //game.next = [3, 1, 4];
        //game.field = [
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 0, 0, 0],
        //    [0, 0, 0, 6, 0, 0],
        //    [0, 0, 0, 5, 0, 2],
        //    [0, 0, 0, 2, 0, 3],
        //    [0, 0, 0, 2, 0, 3],
        //    [0, 0, 1, 1, 0, 1],
        //    [0, 0, 4, 2, 6, 6],
        //    [0, 0, 4, 5, 3, 1],
        //    [4, 0, 6, 5, 6, 4],
        //    [4, 5, 1, 3, 2, 2],
        //];

        game.load_replay("replay.dat");

        game
    }

    pub fn toggle_debug(&mut self) {
        self.is_debug = !self.is_debug;
        println!("is_debug: {}", self.is_debug);
    }

    pub fn load_replay(&mut self, filename: &str) {
        if let Some(content) = std::fs::read_to_string(filename).ok() {
            let mut commands = Vec::new();
            for (_, line) in content.lines().enumerate() {
                let command = Command::from_str(line);
                commands.push(command);
            }
            self.replay_loaded = true;
            self.commands = commands;
        }
    }

    pub fn write_command_log(&mut self, command: Command) {
        self.command_log
            .as_ref()
            .unwrap()
            .write_all(format!("{:?}\n", command).as_bytes())
            .ok();
        self.command_log.as_ref().unwrap().flush().ok();
    }

    pub fn update(&mut self, mut command: Command) {
        self.frame += 1;

        if self.replay_loaded {
            if self.commands.len() > self.frame as usize {
                command = self.commands[self.frame as usize];
            }
        } else {
            self.write_command_log(command);
        }

        if self.is_over {
            return;
        }

        match self.state {
            State::Controllable => {
                wait!(self.fall_wait, {
                    self.fall();
                });

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
            }
            State::Flashing => {
                wait!(self.flashing_wait, {
                    self.actually_erase();
                    self.set_state(State::PieceFalling);
                });
            }
            State::PieceFalling => {
                if self.piece_fall() {
                } else {
                    if self.check_erase() {
                        self.set_state(State::Flashing);
                    } else {
                        self.set_state(State::Controllable);
                    }
                }
            }
        }
    }

    pub fn set_state(&mut self, new_state: State) {
        match new_state {
            State::Controllable => {
                assert!(self.state == State::Controllable || self.state == State::PieceFalling);
                self.erased_one_time = 0;
                self.combo = -1;
                self.spawn();
                self.check_gameover();
            }
            State::Flashing => {
                assert!(self.state == State::Controllable || self.state == State::PieceFalling);

                self.combo += 1;
                self.flashing_wait = FLASHING_WAIT;
            }
            State::PieceFalling => {
                assert!(self.state == State::Flashing);
                self.piece_fall_offset = 0;
            }
        }
        self.state = new_state;
    }

    pub fn check_piece_fall(&mut self) -> bool {
        let mut checked = false;
        for y in (0..=FIELD_H - 2).rev() {
            for x in 0..FIELD_W {
                // 1マス下が空、または落下中ならそのマスも落下中とする
                if self.field[y][x] != EMPTY
                    && (self.field[y + 1][x] == EMPTY || self.piece_falling[y + 1][x])
                {
                    self.piece_falling[y][x] = true;
                    checked = true;
                } else {
                    self.piece_falling[y][x] = false;
                }
            }
        }
        return checked;
    }

    pub fn piece_fall(&mut self) -> bool {
        let mut should_continue = true;
        self.piece_fall_offset += PIECE_FALL_SPEED;
        if self.piece_fall_offset >= CELL_SIZE {
            self.piece_fall_offset = 0;
            self.actually_piece_fall();
            if !self.check_piece_fall() {
                should_continue = false;
            }
        }
        return should_continue;
    }

    pub fn actually_piece_fall(&mut self) {
        for y in (0..=FIELD_H - 2).rev() {
            for x in 0..FIELD_W {
                if self.piece_falling[y][x] {
                    self.field[y + 1][x] = self.field[y][x];
                    self.field[y][x] = EMPTY;
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
        if self.is_intersect() {
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
        self.current_y += 1;
        if self.is_intersect() {
            self.current_y -= 1;
            self.settle();
            if self.check_erase() {
                self.set_state(State::Flashing);
            } else {
                self.set_state(State::Controllable);
            }
        }
        if self.current_y == INVISIBLE_ROW_COUNT {
            self.next_to_display = self.next;
        }
        self.fall_wait = FALL_WAIT;
    }

    pub fn is_intersect(&self) -> bool {
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
        let mut checked = false;

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
                                checked = true;
                            }
                        }
                    }
                }
            }
        }
        return checked;
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
            self.erased_one_time += erased_count;
            if self.max_erased_at_one_time < self.erased_one_time {
                self.max_erased_at_one_time = self.erased_one_time;
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
        self.current = self.next;
        self.current_x = FIELD_W / 2;
        self.current_y = 0;
        for i in 0..BLOCK_LEN {
            self.next[i] = self.rng.as_mut().unwrap().gen_range(1..=COLOR_COUNT)
        }
    }

    pub fn check_gameover(&mut self) {
        // 最上部の上（フィールドからはみ出た場所）に1個でも宝石が積みあがるか、右から3列目のみはみ出していなくても空いているマスが無くなるとゲームオーバー
        let mut is_over = false;
        for x in 0..FIELD_W {
            if self.field[INVISIBLE_ROW_COUNT - 1][x] != EMPTY {
                is_over = true;
                break;
            }
        }
        if self.field[INVISIBLE_ROW_COUNT][FIELD_W - 3] != EMPTY {
            is_over = true;
        }
        if is_over {
            self.is_over = true;
            self.requested_sounds.push("crash.wav");
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
