use ggez::conf;
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics::{self, Color};
use ggez::{Context, ContextBuilder, GameResult};
use ggez::audio::Source;
use glam::*;
use oorandom::Rand32;

use std::env;
use std::path;


#[derive(Debug, Clone, Copy)]
enum LR {
    Left,
    Right
}

impl LR {
    fn to_f32(self) -> f32 {
        match self {
            LR::Left => -1.0,
            LR::Right => 1.0
        }
    }
}

trait Draw {
    fn draw(&self, assets: &mut Assets, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult;
}

struct Player {
    pos: Vec2,
    velocity: Vec2,
    last_velocity: Vec2,
    facing: LR,
    animation_frame: f32,
    bbox_size: f32,
    life: i32,
}

impl Draw for Player {
    fn draw(&self, assets: &mut Assets, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult {
        let (screen_w, screen_h) = world_coords;
        let pos = world_to_screen_coords(screen_w, screen_h, self.pos);
        let frame = if self.velocity.x == 0.0 { 0 } else { self.animation_frame.floor() as usize };
        let image = &assets.player.frames[frame];
        let drawparams = graphics::DrawParam::new()
            .dest(pos)
            .offset(Vec2::new(0.5, 0.5))
            .scale([-self.facing.to_f32()*1.0, 1.0]);
        graphics::draw(ctx, image, drawparams)
    }
}

#[derive(Debug, Clone, Copy)]
struct Candy {
    pos: Vec2,
    velocity: Vec2,
    animation_frame: f32,
    bbox_size: f32,
    is_collected: bool,
}

impl Draw for Candy {
    fn draw(&self, assets: &mut Assets, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult {
        let (screen_w, screen_h) = world_coords;
        let pos = world_to_screen_coords(screen_w, screen_h, self.pos);
        let frame = self.animation_frame.floor() as usize;
        let image = &assets.candy;//.frames[frame];
        let scale = if self.velocity.y > 0.0 {
            1.0 - self.velocity.y.max(0.0).min(80.0) / 80.0
        } else {
            1.0
        };
        let drawparams = graphics::DrawParam::new()
            .dest(pos)
            .offset(Vec2::new(0.5, 0.5))
            .scale([scale, scale])
            ;
        graphics::draw(ctx, image, drawparams)
    }
}

type SpriteFrame = graphics::Image;

struct Stage<const W: usize, const H: usize> {
    block_palette: Arc<Vec<SpriteFrame>>,
    blocks: [[Option<usize>; H]; W],
    background: SpriteFrame,
}

fn get_rank(score: u32, health: i32) -> std::string::String {
    (if score == 0 && health == PLAYER_LIFE {
        "-"
    }
    else {
        let ratio = health as f32 / PLAYER_LIFE as f32;
        let bonus = score as f32  * ratio / 10.0;
        let rankscore = ratio*0.5 + bonus*0.5;
        if rankscore < 0.5 {
            "F"
        } else if rankscore < 0.75 {
            "D"
        } else if rankscore < 0.9 {
            "C"
        } else if rankscore < 1.0 {
            "B"
        } else if rankscore < 1.2 {
            "A"
        } else if rankscore < 1.4 {
            "S"
        } else if rankscore < 1.5 {
            "SS"
        } else {
            "SS+"
        }
    }).to_string()
    
}

impl<const W: usize, const H: usize> Draw for Stage<W, H> {
    fn draw(&self, _assets: &mut Assets, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult {
        let (screen_w, screen_h) = world_coords;
        // let pos = world_to_screen_coords(screen_w, screen_h, self.pos);

        let image = &self.background;
        let drawparams = graphics::DrawParam::new()
            .dest(Vec2::new(0.0, 0.0));
        graphics::draw(ctx, image, drawparams)?;

        for x in 0..W {
            for y in 0..H {
                if let Some(block_id) = self.blocks[x][y] {
                    let pos = world_to_screen_coords(screen_w, screen_h, Vec2::new(
                        (x as f32 - W as f32 / 2.0) * 32.0, 
                        (y as f32- H as f32 / 2.0) * -32.0));

                    let image = &self.block_palette[block_id];
                    let drawparams = graphics::DrawParam::new()
                        .dest(pos);
                    graphics::draw(ctx, image, drawparams)?;
                }
            }
        }
        Ok(())
    }
}

use std::sync::Arc;

#[derive(Clone)]
struct Sprite {
    frames: Arc<Vec<SpriteFrame>>
}

#[derive(Clone)]
struct Particle {
    sprite: Sprite,
    pos: Vec2,
    animation_frame: f32,
}

impl Particle {
    fn update(&mut self, dt: f32) -> bool {
        self.animation_frame += dt * 30.0;
        // keep if:
        (self.animation_frame.floor() as usize) < self.sprite.frames.len()
    }
}

impl Draw for Particle {
    fn draw(&self, _assets: &mut Assets, ctx: &mut Context, world_coords: (f32, f32)) -> GameResult {
        let (screen_w, screen_h) = world_coords;
        let pos = world_to_screen_coords(screen_w, screen_h, self.pos);
        let frame = self.animation_frame.floor() as usize;
        let image = &self.sprite.frames[frame];//.frames[frame];
        let drawparams = graphics::DrawParam::new()
            .dest(pos)
            .offset(Vec2::new(0.5, 0.5))
            ;
        graphics::draw(ctx, image, drawparams)
    }
}


const PLAYER_LIFE: i32 = 12;


/// Acceleration in pixels per second.
const PLAYER_THRUST: f32 = 300.0;
const PLAYER_BREAK_THRUST: f32 = PLAYER_THRUST * 3.0;
const GRAVITY: f32 = 200.0;

const PLAYER_VEL: f32 = 400.0;

fn player_handle_input(actor: &mut Player, input: &ControllerState, dt: f32) {
    let (facing, target_vel) = if input.left { 
        (LR::Left, -1.0 )
    } else if input.right { (LR::Right, 1.0) } else { (actor.facing, 0.0) };

    if actor.velocity.x != 0.0 {
        if actor.last_velocity.x == 0.0 {
            actor.animation_frame += 0.5;
        }
        actor.animation_frame = (actor.animation_frame + 0.2 * actor.velocity.x.abs() / PLAYER_VEL) % 2.0;
    }
    else {
        actor.animation_frame = 0.0;
    }

    actor.facing = facing;
    let target_vel = target_vel * PLAYER_VEL;
    let thrust_sign =
        if actor.velocity.x < target_vel {
            1.0
        } else if actor.velocity.x > target_vel {
            -1.0 
        } else {
            0.0
        };
    
    let thrust =
        if target_vel == 0.0 || actor.velocity.x.signum() != target_vel.signum(){
            PLAYER_BREAK_THRUST * dt * thrust_sign
        } else{
            PLAYER_THRUST * dt * thrust_sign
        };

    
    actor.last_velocity = actor.velocity;
    actor.velocity.x = if (actor.velocity.x - target_vel).abs() <= thrust {
        target_vel
    } else {
        actor.velocity.x + thrust
    };
}

fn update_player_position(actor: &mut Player, dt: f32) {
    let dv = actor.velocity * dt;
    actor.pos += dv;
}

fn world_to_screen_coords(screen_width: f32, screen_height: f32, point: Vec2) -> Vec2 {
    let x = point.x + screen_width / 2.0;
    let y = screen_height - (point.y + screen_height / 2.0);
    Vec2::new(x, y)
}



struct Assets {
    player: Sprite,
    bg: SpriteFrame,
    candy: SpriteFrame,
    font: graphics::Font,
    collect_animation: Sprite,
    bgm: Source,
    bg_palette: Arc<Vec<SpriteFrame>>,
    lifebar: SpriteFrame,
    lifebar_bg: SpriteFrame,
}

#[derive(Debug, Default)]
struct ControllerState {
    left: bool,
    right: bool
}

struct MainState {
    player: Player,
    candies: Vec<Candy>,
    stage: Stage<20, 15>,
    score: u32,
    assets: Assets,
    screen_width: f32,
    screen_height: f32,
    input: ControllerState,
    rng: Rand32,
    particles: Vec<Particle>,
    difficulty: f32,
    is_first_frame: bool,
}

impl MainState {
    fn new(ctx: &mut Context, assets: Assets) -> GameResult<MainState> {
        println!("Game resource path: {:?}", ctx.filesystem);

        print_instructions();

        let (width, height) = graphics::drawable_size(ctx);

        // Seed our RNG
        let seed = 0;
        let rng = Rand32::new(seed);

        // let assets = Assets::new(ctx)?;
        let player = Player {
            pos: Vec2::new(0.0, -height/ 2.0 + 32.0 + 16.0),
            velocity: Vec2::new(0.0, 0.0),
            last_velocity: Vec2::new(0.0, 0.0),
            facing: LR::Right,
            animation_frame: 0.0,
            bbox_size: 10.0,
            life: PLAYER_LIFE,
        };
        let candies = Vec::new();

        let mut stage = Stage { blocks: [[None; 15]; 20], block_palette: assets.bg_palette.clone(), background: assets.bg.clone()};

        for x in 0..20 {
            stage.blocks[x][14] = Some(0);
        }

        
        let s = MainState {
            player,
            candies,
            stage,
            score: 0,
            assets,
            screen_width: width,
            screen_height: height,
            input: ControllerState::default(),
            rng,
            particles: Vec::new(),
            difficulty: 0.0,
            is_first_frame: true
        };

        Ok(s)
    }


   
}

/// **********************************************************************
/// A couple of utility functions.
/// **********************************************************************

fn print_instructions() {
    println!();
    println!("Welcome to Pogin!");
    println!();
    println!("How to play:");
    println!("L/R arrow keys to move");
    println!("Catch candy to appease the Pogin.");
    println!();
}

const DIFFICULTY_RATE: f32 = 0.15;

impl EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        const DESIRED_FPS: u32 = 120;

        while ggez::timer::check_update_time(ctx, DESIRED_FPS) {
            let seconds = 1.0 / (DESIRED_FPS as f32);

            self.difficulty += seconds * DIFFICULTY_RATE;
            let rate = (seconds + 1.0) * 0.001;
            if self.rng.rand_float() < rate || self.is_first_frame {
                self.is_first_frame = false;

                let pos = Vec2::new( ((self.rng.rand_float() - 0.5)*2.0) * self.screen_width * 0.45, 150.0 );
                let mut velx = ((self.rng.rand_float() - 0.5)*2.0)*40.0;
                let vely = self.rng.rand_float()*60.0+20.0;

                if self.screen_width / 2.0 < (pos.x + velx * 2.0).abs() {
                    velx *= -1.0;
                }

                // let time_until_ground = 

                self.candies.push(Candy {
                    pos,
                    velocity: Vec2::new(velx, vely ),
                    animation_frame: 0.0,
                    bbox_size: 10.0,
                    is_collected: false,
                });
            }

            
            player_handle_input(&mut self.player, &self.input, seconds);


            update_player_position(&mut self.player, seconds);
            if self.player.pos.x.abs() > self.screen_width / 2.0 {
                self.player.pos.x = self.screen_width / 2.0 * self.player.pos.x.signum();
                self.player.velocity.x = 0.0;
            }


            for candy in &mut self.candies {
                candy.velocity.y -= GRAVITY * seconds;
                candy.pos += candy.velocity * seconds;
            }

            for candy in &mut self.candies {
                let pdistance = candy.pos - self.player.pos;
                if pdistance.length() < (self.player.bbox_size + candy.bbox_size) {
                    self.score += 1;
                    candy.is_collected = true;
                    self.particles.push(Particle {
                        pos: Vec2::new(self.player.pos.x, self.player.pos.y+16.0),
                        // velocity: Vec2::new(0.0, 0.0),
                        animation_frame: 0.0,
                        sprite: self.assets.collect_animation.clone(),
                    });
                }
                else if candy.pos.y < self.screen_height * -0.5 {
                    self.player.life -= 1;
                    candy.is_collected = true;
                    
                }
            }

            self.particles = self.particles.iter_mut().filter_map(
                |p| {
                    if p.update(seconds) {
                        Some(p.clone())
                    }
                    else {
                        None
                    }
                }).collect();

            self.candies.retain(|candy| !candy.is_collected);

            if self.player.life <= 0 {
                println!("Game over!");
                event::quit(ctx);
            }
        }

        

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {

        // graphics::clear(ctx, Color::from_rgb(180, 100, 200));

        {
            
            let assets = &mut self.assets;
            let coords = (self.screen_width, self.screen_height);

            self.stage.draw(assets, ctx, coords)?;

            for candy in self.candies.iter() {
                candy.draw(assets, ctx, coords)?;
            }

            for particle in self.particles.iter() {
                particle.draw(assets, ctx, coords)?;
            }

            //let p = &self.player as &dyn Draw;

            self.player.draw(assets, ctx, coords)?;

        }

        let level_str = format!("Rank: {}", get_rank(self.score, self.player.life));
        let score_str = format!("Score: {}", self.score);
        let life_str = format!("{}", self.player.life);
        let level_display = graphics::Text::new((level_str, self.assets.font, 32.0));
        let score_display = graphics::Text::new((score_str, self.assets.font, 32.0));
        let life_display = graphics::Text::new((life_str, self.assets.font, 32.0));
        graphics::draw(ctx, &level_display, (Vec2::new(10.0, 10.0), 0.0, Color::WHITE))?;
        graphics::draw(ctx, &score_display, (Vec2::new(200.0, 10.0), 0.0, Color::WHITE))?;
        graphics::draw(ctx, &life_display, (Vec2::new(10.0, 40.0), 0.0, Color::WHITE))?;

        for i in 0..PLAYER_LIFE {
            let x = (i as f32) * (5.0) + 44.0;

            let drawparams = graphics::DrawParam::new()
                .dest(Vec2::new(x, 42.0));
            graphics::draw(ctx, &self.assets.lifebar_bg, drawparams)?;
            if i < self.player.life {
                graphics::draw(ctx, &self.assets.lifebar, drawparams)?;
            }
        }
            
        

        graphics::present(ctx)?;

        Ok(())
    }


    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::Left => {
                self.input.left = true;
            }
            KeyCode::Right => {
                self.input.right = true;
            }
            KeyCode::P => {
                let img = graphics::screenshot(ctx).expect("Could not take screenshot");
                img.encode(ctx, graphics::ImageFormat::Png, "/screenshot.png")
                    .expect("Could not save screenshot");
            }
            KeyCode::Escape => event::quit(ctx),
            _ => (), 
        }
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymod: KeyMods) {
        match keycode {
            KeyCode::Left => {
                self.input.left = false;
            }
            KeyCode::Right => {
                self.input.right = false;
            }
            _ => (), 
        }
    }
}


pub fn main() -> GameResult {

    // ggez will look in ./resources
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ContextBuilder::new("pogin", "dunkyl")
        .window_setup(conf::WindowSetup::default().title("Pogin!"))
        .window_mode(conf::WindowMode::default().dimensions(640.0, 480.0))
        .add_resource_path(resource_dir);

    let (mut ctx, events_loop) = cb.build()?;



    let player = Sprite { 
        frames: Arc::new(vec![
            graphics::Image::new(&mut ctx, "/cat1.png")?,
            graphics::Image::new(&mut ctx, "/cat2.png")?])};
    let bg = graphics::Image::new(&mut ctx, "/bg.png")?;
    let candy = graphics::Image::new(&mut ctx, "/candy_a.png")?;
    let font = graphics::Font::new(&mut ctx, "/Minecraftia.ttf")?;
    
    let collect_animation = Sprite {
        frames: Arc::new(vec![
            graphics::Image::new(&mut ctx, "/collect1.png")?,
            graphics::Image::new(&mut ctx, "/collect2.png")?,
            graphics::Image::new(&mut ctx, "/collect3.png")?,
            graphics::Image::new(&mut ctx, "/collect4.png")?,
            graphics::Image::new(&mut ctx, "/collect5.png")?,
        ])
    };
    let bgm = Source::new(&mut ctx, "/c.mp3")?;

    let bg_palette = Arc::new(vec![
        graphics::Image::new(&mut ctx, "/ground.png")?,
    ]);

    let lifebar = graphics::Image::new(&mut ctx, "/lifebar.png")?;
    let lifebar_bg = graphics::Image::new(&mut ctx, "/lifebar_bg.png")?;

    let assets: Assets = Assets {
        player,
        bg,
        candy,
        font,
        collect_animation,
        bgm,
        bg_palette,
        lifebar,
        lifebar_bg
    };

    // let _ = assets.bgm.play(&mut ctx)?;
    let game = MainState::new(&mut ctx, assets)?;
    event::run(ctx, events_loop, game)
}