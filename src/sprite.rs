use std::rc::Rc;

use ggez::{graphics::{Image, Rect, draw, DrawParam}, Context, GameError};
use glam::*;

#[derive(Clone)]
pub struct SpriteSheet {
    image: Image,
    frames: Rc<[Rect]>
}

pub struct Sprite {
    sheet: SpriteSheet,
    pub pos: Vec2,
    pub scale: Vec2
}

impl SpriteSheet {
    pub fn new(sheet: Image, count_x: usize, count_y: usize) -> Self {
        let mut frames = Vec::with_capacity(count_x*count_y);
        let (sz_x, sz_y) = (1.0 / count_x as f32, 1.0 / count_y as f32);
        for x in 0..count_x {
            for y in 0..count_y {
                let (x, y) = (x as f32, y as f32);
                frames.push(
                    Rect::new(x*sz_x, y*sz_y, sz_x, sz_y)
                );
            }
        }
        SpriteSheet {
            image: sheet,
            frames: frames.into()
        }
    }

    pub fn sprite(&self, pos: Vec2) -> Sprite {
        Sprite {
            sheet: self.clone(),
            pos,
            scale: vec2(1.0, 1.0)
        }
    }
}

impl Sprite {
    pub fn draw_frame(&self, ctx: &mut Context, frame: usize) -> Result<(), GameError> {
        let src = self.sheet.frames[frame];
        draw(ctx, &self.sheet.image, DrawParam::new().dest(self.pos).scale(self.scale).offset(vec2(0.5, 0.5)).src(src))
    }
}
