use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::ttf::Sdl2TtfContext;
use sdl2::video::{Window, WindowContext};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime};
mod model;
use crate::model::*;

pub const WINDOW_TITLE: &str = "rust-columns";
pub const SCREEN_WIDTH: i32 = FIELD_W as i32 * CELL_SIZE + INFO_WIDTH;
pub const SCREEN_HEIGHT: i32 = (FIELD_H - INVISIBLE_ROW_COUNT) as i32 * CELL_SIZE;
pub const INFO_WIDTH: i32 = 190;

struct Image<'a> {
    texture: Texture<'a>,
    #[allow(dead_code)]
    w: u32,
    h: u32,
}

impl<'a> Image<'a> {
    fn new(texture: Texture<'a>) -> Self {
        let q = texture.query();
        let image = Image {
            texture,
            w: q.width,
            h: q.height,
        };
        image
    }
}

struct Resources<'a> {
    images: HashMap<String, Image<'a>>,
    chunks: HashMap<String, sdl2::mixer::Chunk>,
    fonts: HashMap<String, sdl2::ttf::Font<'a, 'a>>,
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;

    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window(WINDOW_TITLE, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    sdl_context.mouse().show_cursor(false);

    init_mixer();

    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_blend_mode(BlendMode::Blend);

    let texture_creator = canvas.texture_creator();
    let mut resources = load_resources(&texture_creator, &mut canvas, &ttf_context);

    let mut event_pump = sdl_context.event_pump()?;

    let mut game = Game::new();

    println!("Keys:");
    println!("  Left    : Move left");
    println!("  Right   : Move right");
    println!("  Down    : Drop");
    println!("  Space   : Rotate");
    println!("  Enter   : Restart when gameover");

    'running: loop {
        let started = SystemTime::now();

        let mut command = Command::None;
        let mut is_keydown = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => {
                    is_keydown = true;
                    if code == Keycode::Escape {
                        break 'running;
                    }
                    match code {
                        Keycode::Return => {
                            if game.is_over {
                                game = Game::new();
                            }
                        }
                        Keycode::F1 => {
                            game.toggle_debug();
                            println!("{:?}", game);
                        }
                        Keycode::Left => command = Command::Left,
                        Keycode::Right => command = Command::Right,
                        Keycode::Down => command = Command::Down,
                        Keycode::Space => command = Command::Rotate,
                        _ => {}
                    };
                }
                _ => {}
            }
        }
        if !game.is_debug || is_keydown {
            game.update(command);
        }
        render(&mut canvas, &game, &mut resources)?;

        play_sounds(&mut game, &resources);

        let finished = SystemTime::now();
        let elapsed = finished.duration_since(started).unwrap();
        let frame_duration = Duration::new(0, 1_000_000_000u32 / model::FPS as u32);
        if elapsed < frame_duration {
            ::std::thread::sleep(frame_duration - elapsed)
        }
    }

    Ok(())
}

fn init_mixer() {
    let chunk_size = 1_024;
    mixer::open_audio(
        mixer::DEFAULT_FREQUENCY,
        mixer::DEFAULT_FORMAT,
        mixer::DEFAULT_CHANNELS,
        chunk_size,
    )
    .expect("cannot open audio");
    let _mixer_context = mixer::init(mixer::InitFlag::MP3).expect("cannot init mixer");
}

fn load_resources<'a>(
    texture_creator: &'a TextureCreator<WindowContext>,
    #[allow(unused_variables)] canvas: &mut Canvas<Window>,
    ttf_context: &'a Sdl2TtfContext,
) -> Resources<'a> {
    let mut resources = Resources {
        images: HashMap::new(),
        chunks: HashMap::new(),
        fonts: HashMap::new(),
    };

    let entries = fs::read_dir("resources/image").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".bmp") {
            let temp_surface = sdl2::surface::Surface::load_bmp(&path).unwrap();
            let texture = texture_creator
                .create_texture_from_surface(&temp_surface)
                .expect(&format!("cannot load image: {}", path_str));

            let basename = path.file_name().unwrap().to_str().unwrap();
            let image = Image::new(texture);
            resources.images.insert(basename.to_string(), image);
        }
    }

    let entries = fs::read_dir("./resources/sound").unwrap();
    for entry in entries {
        let path = entry.unwrap().path();
        let path_str = path.to_str().unwrap();
        if path_str.ends_with(".wav") {
            let chunk = mixer::Chunk::from_file(path_str)
                .expect(&format!("cannot load sound: {}", path_str));
            let basename = path.file_name().unwrap().to_str().unwrap();
            resources.chunks.insert(basename.to_string(), chunk);
        }
    }

    load_font(
        &mut resources,
        &ttf_context,
        "./resources/font/boxfont2.ttf",
        24,
        "boxfont",
    );

    resources
}

fn load_font<'a>(
    resources: &mut Resources<'a>,
    ttf_context: &'a Sdl2TtfContext,
    path_str: &str,
    point_size: u16,
    key: &str,
) {
    let font = ttf_context
        .load_font(path_str, point_size)
        .expect(&format!("cannot load font: {}", path_str));
    resources.fonts.insert(key.to_string(), font);
}

fn render(
    canvas: &mut Canvas<Window>,
    game: &Game,
    resources: &mut Resources,
) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(32, 32, 32));
    canvas.clear();

    let font = resources.fonts.get_mut("boxfont").unwrap();

    canvas.set_draw_color(Color::RGB(255, 128, 128));

    // render field
    for y in INVISIBLE_ROW_COUNT..FIELD_H {
        for x in 0..FIELD_W {
            if game.field[y][x] != EMPTY {
                let color;
                if game.state == State::Flashing && game.check_erase_result[y][x] {
                    if game.flashing_wait % 2 == 0 {
                        color = Color::RGB(255, 255, 255);
                    } else {
                        color = get_block_color(game.field[y][x]);
                    }
                } else {
                    color = get_block_color(game.field[y][x]);
                }
                let offset_y = if game.state == State::PieceFalling && game.piece_falling[y][x] {
                    game.piece_fall_offset
                } else {
                    0
                };
                canvas.set_draw_color(color);
                canvas.fill_rect(Rect::new(
                    (x as i32) * (CELL_SIZE as i32),
                    (y as i32 - INVISIBLE_ROW_COUNT as i32) * (CELL_SIZE as i32) + offset_y,
                    CELL_SIZE as u32,
                    CELL_SIZE as u32,
                ))?;
            }
        }
    }

    // render info
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.fill_rect(Rect::new(
        SCREEN_WIDTH - INFO_WIDTH,
        0,
        INFO_WIDTH as u32,
        SCREEN_HEIGHT as u32,
    ))?;

    // render current block
    if game.state == State::Controllable {
        for i in 0..BLOCK_LEN {
            let color = get_block_color(game.current[i]);
            canvas.set_draw_color(color);
            canvas.fill_rect(Rect::new(
                (game.current_x as i32) * (CELL_SIZE as i32),
                ((game.current_y + i) as i32 - INVISIBLE_ROW_COUNT as i32) * (CELL_SIZE as i32),
                CELL_SIZE as u32,
                CELL_SIZE as u32,
            ))?;
        }
    }

    // render next block
    for i in 0..BLOCK_LEN {
        let color = get_block_color(game.next_to_display[i]);
        canvas.set_draw_color(color);
        canvas.fill_rect(Rect::new(
            (FIELD_W as i32 + 1) * (CELL_SIZE as i32),
            (i as i32) * (CELL_SIZE as i32),
            CELL_SIZE as u32,
            CELL_SIZE as u32,
        ))?;
    }

    let font_color = Color::RGB(224, 224, 224);
    render_font(
        canvas,
        font,
        format!("JEWELS {:6}", game.total_erased).to_string(),
        SCREEN_WIDTH - INFO_WIDTH + 20,
        230,
        font_color,
        false,
    );
    render_font(
        canvas,
        font,
        format!("MAX ERASE {:3}", game.max_erased_at_one_time).to_string(),
        SCREEN_WIDTH - INFO_WIDTH + 20,
        270,
        font_color,
        false,
    );

    for i in 0..game.combo {
        render_font(
            canvas,
            font,
            format!("COMBO!").to_string(),
            (game.current_x + 1) as i32 * CELL_SIZE,
            (game.current_y as i32 - INVISIBLE_ROW_COUNT as i32 + 1 + i) as i32 * CELL_SIZE,
            get_block_color(1 + (game.frame + i) % COLOR_COUNT),
            false,
        );
    }

    if game.is_over {
        canvas.set_draw_color(Color::RGBA(255, 0, 0, 128));
        canvas.fill_rect(Rect::new(0, 0, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32))?;
    }

    canvas.present();

    Ok(())
}

fn render_font(
    canvas: &mut Canvas<Window>,
    font: &sdl2::ttf::Font,
    text: String,
    x: i32,
    y: i32,
    color: Color,
    center: bool,
) {
    let texture_creator = canvas.texture_creator();

    let surface = font.render(&text).blended(color).unwrap();
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .unwrap();
    let x: i32 = if center {
        x - texture.query().width as i32 / 2
    } else {
        x
    };
    canvas
        .copy(
            &texture,
            None,
            Rect::new(x, y, texture.query().width, texture.query().height),
        )
        .unwrap();
}

fn play_sounds(game: &mut Game, resources: &Resources) {
    for sound_key in &game.requested_sounds {
        let chunk = resources
            .chunks
            .get(&sound_key.to_string())
            .expect("cannot get sound");
        sdl2::mixer::Channel::all()
            .play(&chunk, 0)
            .expect("cannot play sound");
    }
    game.requested_sounds = Vec::new();
}

fn get_block_color(color_num: i32) -> Color {
    match color_num {
        1 => Color::RGB(255, 128, 128),
        2 => Color::RGB(128, 255, 128),
        3 => Color::RGB(128, 128, 255),
        4 => Color::RGB(255, 255, 128),
        5 => Color::RGB(128, 255, 255),
        6 => Color::RGB(255, 128, 255),
        _ => panic!(),
    }
}
