use sdl2::{rect::{Rect, Point}, pixels::Color};
use vecm::vec::{Vec2i, Vec2u};

use crate::{board::{Board, ColorTheme}, renderer::Renderer, pieces::{Side, Piece}};




pub struct BoardRenderer {
    board_ground: Vec<(Rect, Color)>,
    hovering: Option<Vec2i>,
    field_size: u32,
    color_theme: ColorTheme,
    pub selected: Option<Vec2i>,
    valid_mvs_tick: f32,
    last_move_tick: f32,
}


impl BoardRenderer {
    pub fn new(field_size: u32, color_theme: ColorTheme, size: Vec2u) -> Self {
        let mut board_ground: Vec<(Rect, Color)> = Vec::new();
        let mut color = Color::WHITE;
        for x in 0..(size.x as i32) {
            for y in 0..(size.y as i32) {
                let rect = Rect::new(field_size as i32 * x, field_size as i32 * y, field_size, field_size);
                if (x % 2 == 1 && y % 2 == 0) || (x % 2 == 0 && y % 2 == 1) {
                    //color = black
                    color = color_theme.board_secondary;
                } else {
                    color = color_theme.board_primary;
                }
                board_ground.push((rect, color));
            }
        }
        Self {board_ground, hovering: None, selected: None, valid_mvs_tick: 0.0, last_move_tick: 0.0 , field_size, color_theme}
    }

    pub fn hover(&mut self, pos: Vec2i) {
        self.hovering = Some(pos);
    }


    pub fn render(&mut self, turn: &Side, board: &Board, renderer: &mut Renderer) {
        for rect in &self.board_ground {
            renderer.draw_rect(rect.0, rect.1, 0);
        }

        for (x, y_row) in  board.board.iter().enumerate() {
            for (y, optional_piece) in y_row.iter().enumerate() {
                //dont draw selection
                let field_pos = Vec2i::new(x as i32,y as i32);
                if let Some(selected) = self.selected && selected == field_pos {
                    
                    //if something selected then draw valid moves
                    let r_size = (Vec2u::fill(self.field_size) * 3) / 4;
                    if let Some(valid_moves) = board.valid_moves.get(&selected) {
                        for mv in valid_moves {
                            let r_center = *mv * self.field_size as i32 + Vec2i::fill(self.field_size as i32 / 2);
                            let rect = Rect::from_center(Point::new(r_center.x, r_center.y), r_size.x, r_size.y);
                            let color = self.color_theme.valid_moves;
                            renderer.draw_rect(rect, color, 0);
                        }
                    }
                    let r_center = field_pos * self.field_size as i32 + Vec2i::fill(self.field_size as i32 / 2);
                    let color = self.color_theme.selection;
                    let rect = Rect::from_center(Point::new(r_center.x, r_center.y), r_size.x, r_size.y);
                    renderer.draw_rect(rect, color, 0);

                    continue;
                }

                //possible moves: depth = 1

                if let Some(piece) = optional_piece {
                    let mut window_pos = field_pos * self.field_size as i32;
                    let mut size = self.field_size;
                    //hovering expands piece
                    if let Some(hover_pos) = self.hovering{
                        if &piece.side == turn && hover_pos.x == x as i32 && hover_pos.y == y as i32{
                            window_pos -= 5;
                            size += 10;
                        }
                    }

                    renderer.draw_image(
                        piece.ty,
                        piece.side,
                        Rect::new(window_pos.x,window_pos.y, size, size),
                        2
                    )           
                }
            }
        }
        self.hovering = None
    }

    pub fn unselect(&mut self) {
        self.selected = None
    }

    pub fn get_selected_piece(&self, board: &Board) -> Option<Piece> {
        if let Some(selected) = self.selected {
            return board.get_piece_at_pos(selected)
        }
        None
    }

    pub fn select(&mut self, cursor_field: Vec2i, turn: Side, board: &Board) -> Option<Piece> {
        //previous selection
        if let Some(selection) = board.get_piece_at_pos(cursor_field) {
            if selection.side == turn {
                self.selected = Some(cursor_field);
                return Some(selection)
            }
        }
        None
    }

}