#![feature(let_chains)]
#![feature(hash_drain_filter)]
pub mod pieces;
pub mod board;
pub mod macros;
pub mod atlas;
pub mod renderer;
pub mod board_renderer;
pub mod dtos;
pub mod game;
pub mod color_themes;
pub mod boardc;
pub mod gamec;
pub mod game_renderer;
pub mod boardb;
pub mod gameb;
pub mod castle;

use atlas::TextureAtlas;
use binverse::error::BinverseError;
use board::{Board};
use board_renderer::BoardRenderer;
use game_renderer::GameRenderer;
use dtos::{PlayerInfo, Move, GameInfo};
use game::PlayerType;
use pieces::{Piece, Side};
use input::InputHandler;
use renderer::Renderer;
use sdl2::image::{LoadTexture, InitFlag};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::{rect::{Rect, Point}, pixels::Color, render::{Canvas, Texture}, video::Window, sys::PropModePrepend};
use vecm::vec::{Vec2i, Vec2u, Vec2, VecInto};

use std::collections::{HashMap, HashSet};
use std::env::Args;
use std::net::TcpStream;
use std::ops::Add;
use std::path::Path;
use std::sync::mpsc::{self, TryRecvError, Receiver};
use std::thread::JoinHandle;
//use world::celo::Celo;
use std::time::{Duration, Instant};
use rand::Rng;

mod input; 


use crate::boardc::BoardC;
use crate::color_themes::ColorTheme;
use crate::game::{Game, Remote, GameState};
use crate::gameb::GameB;
use crate::gamec::GameC;
use crate::input::Control;

fn receive_mvs(mut tcp_stream: TcpStream, moves: mpsc::Sender<Move>) -> Result<(), BinverseError> {
    loop {
        moves.send(dtos::recv(&mut tcp_stream)?).unwrap();
    }
}

struct MultiplayerUtils {
    tcp_stream: TcpStream,
    moves_rx: mpsc::Receiver<Move>,
    my_side: Side,
}

fn parse_args(args: &mut Args) -> (bool, bool, Option<usize>, Option<usize>, Option<String>, Option<String>){
    args.skip(1);
    let mut versus = true;
    let mut server = false;
    let mut ai = None;
    let mut ip = None;
    let mut fen = None;
    let mut vai = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--server" => panic!("Not available at the moment"),//server = true,
            "-a" | "--ai" => ai = Some(
                args.next()
                    .expect("give ai depth as argument")
                    .parse::<usize>()
                    .expect("depth has to be a positive integer")
                ),
            "-v" | "--vai" => vai = Some(
                args.next()
                    .expect("give ai depth as argument")
                    .parse::<usize>()
                    .expect("depth has to be a positive integer")
                ),
            "-f" | "--fen" => fen = Some(args.next().expect("fen expected after -f/--fen")),
            "-c" | "--c" => ip = Some(args.next().expect("connect requires ip")), 
            _ => eprintln!("unrecognized arg {arg}"),
        }
    }
    (versus, server, ai, vai, ip, fen)
} 

#[derive(Debug)]
enum ConnectionError {
    Playername,
    IPParse,
    Send,
    Receive
}

fn connect(ip: String) -> Result<MultiplayerUtils, ConnectionError> {
    println!("Type in your name: ");
    let mut player_name = String::new();
    std::io::stdin().read_line(&mut player_name).or(Err(ConnectionError::Playername))?;
    player_name = player_name.trim().to_owned();
    println!("Waiting for opponent");
    let mut tcp_stream = TcpStream::connect(ip).or(Err(ConnectionError::IPParse))?;
    dtos::send(&mut tcp_stream, PlayerInfo { name: player_name }).or(Err(ConnectionError::Send))?;
    let game_info: GameInfo = dtos::recv(&mut tcp_stream).or(Err(ConnectionError::Receive))?;
    let my_side = if game_info.is_black { Side::Black } else { Side::White };
    println!("Your Enemy has connected: {}", game_info.other_player);
    println!("Your are: {}", my_side);

    
    let tcp_stream_clone = tcp_stream.try_clone().unwrap();
    let (sender, rx) = mpsc::channel();

    std::thread::spawn(|| receive_mvs(tcp_stream_clone, sender));
    Ok(MultiplayerUtils { tcp_stream, moves_rx: rx, my_side})
}

fn try_apply_remote_move(game: &mut Game) {
    if let PlayerType::Remote(remote) = &game.turn() {
        match remote.rx.try_recv() {
            Ok(new_move) => {
                println!("Receiving move {:?} for {:?}", new_move, game.turn);
                if !game.make_move(Vec2i::new(new_move.x1 as i32, 7 - new_move.y1 as i32), Vec2i::new(new_move.x2 as i32, 7 - new_move.y2 as i32)) {
                    panic!("Opponent move not accepted");
                }
                game.change_turn();
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => panic!("Disconnected"),
        }
    }
}


fn main() -> Result<(), String> {
    let mut args = std::env::args();
    let (versus, server, ai, vai, ip, fen) = parse_args(&mut args);
    let mut mp = false;

    let mut gameb = GameB::versus();
    let mut game = Game::versus();


    if let Some(ip)  = ip {
        let mp_utils = match connect(ip) {
            Ok(utils) => {mp = true; utils},
            Err(err) => panic!("Error connecting: {:?}", err)
        };
        game = Game::remote(
            Remote::new(mp_utils.tcp_stream, mp_utils.moves_rx), 
            match mp_utils.my_side {
                Side::Black => true,
                Side::White => false,
            }
        );
    }

    if let Some(depth) = ai {
        let mut rng = rand::thread_rng();
        let is_white: bool = rng.gen();
        game = Game::cpu(depth, is_white)
    }

    if let Some(depth) = vai {
        game = Game::vcpu(depth)
    }

    if let Some(fen) = fen {
       match Board::from_fen(&fen) {
        Ok((b,t)) => {game.board = b; game.turn = t},
        Err(err) => println!("Fen error: {:?}", err),
    }
    }



    let font_path = &Path::new("../../res/IBMPlexSerif-Medium.ttf");
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let mut font = ttf_context.load_font(font_path, 128)?;
    font.set_style(sdl2::ttf::FontStyle::BOLD);


    let window = video_subsystem.window("Chess", 400, 400)
        //.resizable()
        .position_centered()
        .build()
        .expect("could not initialize video subsystem");

    let mut canvas: Canvas<sdl2::video::Window> = window.into_canvas().build()
        .expect("could not make a canvas");
    
    let mut screen_size = Vec2u::new(400, 400);
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    let mut event_pump = sdl_context.event_pump()?;
    let mut inputs = InputHandler::new();

    let chess_pieces = &Path::new("../../res/chess_pieces.png");
    let texture_creator = canvas.texture_creator();
    let pieces_texture = texture_creator.load_texture(chess_pieces)?;
    let mut tex_atlas = TextureAtlas::new(&pieces_texture, 90);

    let field_size = 50;
    let board_size = Vec2u::fill(8);

    let mut color_lifted = true;
    let mut pieces_lifted = true;

    let mut renderer = Renderer::new(&tex_atlas, 200.0, &mut canvas);
    game.board.calculate_valid_moves(game.turn);
    let mut board_renderer = BoardRenderer::new(field_size, board_size, 2.0);
    let mut game_renderer = GameRenderer::new(field_size, board_size, 100.0);



    let mut last_frame_time = Instant::now();



    fn spawn_move_computer(board: Board, depth: usize, turn: Side) -> JoinHandle<(Vec2i, Vec2i)> { 
        std::thread::spawn(move || {
            compute_best_move(&board, depth, turn, true).0
        })
    }

    /*fn minimax(board: &Board, depth: usize, maximizing_side: Side) -> i32 {
        if depth == 0 { //or game is over
            return board.evaluate(maximizing_side) // maximzing side right?
        }

    }*/



    fn compute_best_move(board: &Board, depth: usize, turn: Side, is_top: bool) -> ((Vec2i, Vec2i), i32) { 
        let next_moves_by_piece = board.valid_moves.clone();
        let mut best_move = ((Vec2i::zero(), Vec2i::zero()), std::i32::MIN);
        let mut progress = 0;
        let total: i32 = next_moves_by_piece.iter().map(|(_,v)| v.len() as i32).sum();
        for (piece_pos, mvs) in next_moves_by_piece {
            for dst in mvs {
                if is_top {
                    progress +=1;
                    println!("{} / {}", progress, total)
                }
                let mut board = board.clone();
                let eval = match board.make_move(&piece_pos, &dst, turn) {
                    Ok(game_state) => {
                        match game_state {
                            GameState::Running => {if depth == 0 {
                                board.evaluate(turn)
                            } else {
                                -compute_best_move(&board, depth - 1, !turn, false).1
                            }},
                            GameState::Winner(side) => {if turn == side {i32::MAX} else {i32::MIN}}
                            GameState::Draw => i32::MIN,
                        }
                    },
                    Err(_) => {0},
                };
                if eval > best_move.1 {
                    best_move = ((piece_pos, dst), eval);
                }
            }
        }
        best_move
    }

    let mut next_move_option: Option<JoinHandle<(Vec2i, Vec2i)>> = None;


    'running: loop {
        let current_frame_time = Instant::now();
        let dt = (current_frame_time - last_frame_time).as_secs_f32();
        last_frame_time = current_frame_time;

        inputs.handle_events(&mut event_pump);
        if inputs.quit {
            break 'running;
        }

        
        let cursor_field_xy = inputs.mouse_pos / field_size;
        let cursor_field = pos!(cursor_field_xy.x,cursor_field_xy.y) as u8;

        //colortheme
        if inputs.pressed(Control::Color) && color_lifted {
            game_renderer.next_theme();
        }
        if inputs.pressed(Control::Pieces) && pieces_lifted {
            tex_atlas.next_theme();
        }
        if inputs.pressed(Control::Escape) {
            game_renderer.unselect();
        }
        color_lifted = !inputs.pressed(Control::Color);
        pieces_lifted = !inputs.pressed(Control::Pieces);

        if inputs.left_click {
            if let Some(selected) = game_renderer.selected && gameb.turn().is_me() {
                gameb.make_move(selected, cursor_field);
                game_renderer.unselect();

            } else {
                game_renderer.select(cursor_field, gameb.turn, &gameb.board);
            }
        }

        game_renderer.update_mouse_pos(inputs.mouse_pos);
        game_renderer.render(&gameb, &mut renderer, dt);
        renderer.render();



        use sdl2::mouse::MouseButton::*;
        inputs.mouse_up(Left);
        inputs.mouse_up(Right);

    }

    Ok(())
}
