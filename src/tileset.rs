use ggez::conf;
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics::{self, Color, Image, FillOptions};
use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::Source;
use glam::*;
use oorandom::Rand32;

use std::env;
use std::path::{self, Path};
use std::rc::Rc;

pub struct TileRule {
    empty: Box<[IVec2]>
}

pub struct TileSet {
    sprite_sheet: Image,
    count: usize,
    rules: Box<[TileRule]>
}

impl TileSet {
    pub fn new(ctx: &mut Context, sheet: impl AsRef<Path>, rules: Vec<Vec<IVec2>>) -> Self {
        let img = Image::new(ctx, sheet).unwrap();
        let count = (dbg!(img.width()) / dbg!(img.height())) as usize;
        assert_eq!(rules.len(), count);
        let rules = rules.into_iter().map(|coords| TileRule {
            empty: coords.into_boxed_slice() } ).collect::<Vec<_>>().into_boxed_slice();
        TileSet {
            sprite_sheet: img,
            count,
            rules,
        }
    }

    pub fn img(&self) -> graphics::Image {
        self.sprite_sheet.clone()
    }
}