extern crate ggez;
use core::f32::consts::E;
use ggez::event::Keycode;
use ggez::event::Mod;
use ggez::graphics::DrawParam;
use ggez::graphics::{Color, Point2};
use ggez::graphics::{BLACK, WHITE};
use ggez::*;
use itertools::izip;
use rand::distributions::Standard;
use rand::{Rng, SeedableRng};

const PINK: Color = Color {
    r: 1.0,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

struct Me {
    data: Instancedata,
    velocity: Point2,
}
struct MainState {
    me: Me,
    planets: Vec<Instancedata>,
    planet_sprite: graphics::Image,
    window_dims: Point2,
    sig_config: SigConfig,
    x_pressed: Option<bool>,
    y_pressed: Option<bool>,
    scrolling_index: usize,
    scrolling_text: Vec<Option<ggez::graphics::Text>>,
}

const NUM_VARS: usize = 5;

#[derive(Debug, Copy, Clone)]
struct Instancedata {
    pos: Point2,
    scale: f32,
}

#[derive(Debug, Clone)]
struct SigConfig {
    x: [f32; NUM_VARS],
}
impl Default for SigConfig {
    fn default() -> Self {
        Self {
            x: [0.04, -2.0, 1., 1., 1.],
        }
    }
}

fn length(p: Point2) -> f32 {
    (p[0].powf(2.0) + p[1].powf(2.0)).sqrt()
}

impl MainState {
    fn sig(&self, dist: f32) -> f32 {
        let x: &[f32; NUM_VARS] = &self.sig_config.x;

        let q = 2. / (1.0 + E.powf(-x[1])) - 1.;
        // println!("q {}", q);

        (2. / (
            1.0 + E.powf(-x[1] - dist * x[0])
            // higher mult pushes more to edges
        ) - 1.
            - q)
            / (1. - q)

        // 2. / (
        // 	1.0 + E.powf(-dist*0.08)
        // ) - 1.

        // (2. / (
        // 	1.0 + E.powf(-2. - dist*a) // higher mult pushes more to edges
        // ) - 1.4621172-0.29947686) / 0.23840594
    }
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let mut planets = vec![];
        const SQN: usize = 10;
        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        let mut clos =
            || rng.sample::<f32, _>(Standard).powf(3.0) * 600.0 * if rng.gen() { -1. } else { 1. };

        for _i in 0..SQN {
            for _j in 0..SQN {
                let dp = Instancedata {
                    pos: Point2::new(clos(), clos()),
                    scale: 0.4,
                };
                planets.push(dp);
            }
        }

        let mut planet_sprite = graphics::Image::new(ctx, "/ball.png")?;
        planet_sprite.set_filter(graphics::FilterMode::Nearest);
        let d = ggez::graphics::get_size(ctx);
        let window_dims = Point2::new(d.0 as f32, d.1 as f32);
        let me = Me {
            data: Instancedata {
                pos: Point2::new(0., 0.),
                scale: 1.,
            },
            velocity: Point2::new(0., 0.),
        };
        let s = MainState {
            me,
            planets,
            planet_sprite,
            window_dims,
            sig_config: Default::default(),
            x_pressed: None,
            y_pressed: None,
            scrolling_index: 0,
            scrolling_text: (0..NUM_VARS).map(|_| None).collect(),
        };
        Ok(s)
    }
}

trait Joystick: Copy {
    fn bump_left(&mut self);
    fn bump_right(&mut self);
    fn val(self) -> f32;
}

const ACCEL: f32 = 0.01;
impl Joystick for Option<bool> {
    fn val(self) -> f32 {
        match self {
            Some(false) => -ACCEL,
            None => 0.0,
            Some(true) => ACCEL,
        }
    }
    fn bump_left(&mut self) {
        match self {
            Some(false) => (),
            Some(true) => *self = None,
            None => *self = Some(false),
        }
    }
    fn bump_right(&mut self) {
        match self {
            Some(true) => (),
            Some(false) => *self = None,
            None => *self = Some(true),
        }
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        let mut acc = Point2::new(self.x_pressed.val(), self.y_pressed.val());
        if self.x_pressed.is_some() && self.y_pressed.is_some() {
            acc /= 1.4;
        }
        self.me.data.pos += self.me.velocity.coords;
        self.me.velocity += acc.coords;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);

        let x: &[f32; NUM_VARS] = &self.sig_config.x;

        ggez::graphics::set_color(ctx, WHITE)?;
        for i in self.planets.iter() {
            let rel: Point2 = i.pos - self.me.data.pos.coords;
            let dist = length(rel);
            //    let sig = 2.0 / (
            // 	1.0 + E.powf(-dist*0.08)
            // ) - 1.0;
            let sigged = self.sig(dist);
            let newdist: f32 = sigged * 250.0 * x[4];
            let dest = rel / dist * newdist + (self.window_dims.coords / 2.0);
            let s = (i.scale * x[3] * (1.0 - sigged).powf(x[2])).max(0.005);
            let dp = DrawParam {
                dest,
                scale: Point2::new(s, s),
                offset: Point2::new(0.5, 0.5),
                ..Default::default()
            };
            graphics::draw_ex(ctx, &self.planet_sprite, dp)?;
        }

        for (i, value, font) in izip!(
            0..,
            self.sig_config.x.iter(),
            self.scrolling_text.iter_mut()
        ) {
            let f = font.get_or_insert_with(|| {
                let font = ggez::graphics::Font::default_font().unwrap();
                let s = format!("{}", value);
                ggez::graphics::Text::new(ctx, &s, &font).expect("K")
            });
            let col = if i == self.scrolling_index {
                PINK
            } else {
                WHITE
            };
            let dp = DrawParam {
                dest: Point2::new(20., (i * 30) as f32 + 20.),
                scale: Point2::new(1., 1.),
                ..Default::default()
            };
            ggez::graphics::set_color(ctx, col)?;
            ggez::graphics::draw_ex(ctx, f, dp)?;
        }

        graphics::present(ctx);
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        match keycode {
            Keycode::D => self.x_pressed.bump_left(),
            Keycode::A => self.x_pressed.bump_right(),
            Keycode::S => self.y_pressed.bump_left(),
            Keycode::W => self.y_pressed.bump_right(),
            _ => (),
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        match keycode {
            Keycode::Escape => ctx.quit().unwrap(),
            Keycode::Space => self.me.velocity = Point2::new(0., 0.),
            Keycode::Backspace => self.me.data.pos = Point2::new(0., 0.),
            Keycode::A => self.x_pressed.bump_left(),
            Keycode::D => self.x_pressed.bump_right(),
            Keycode::W => self.y_pressed.bump_left(),
            Keycode::S => self.y_pressed.bump_right(),
            Keycode::Num1 => self.scrolling_index = 0,
            Keycode::Num2 => self.scrolling_index = 1,
            Keycode::Num3 => self.scrolling_index = 2,
            Keycode::Num4 => self.scrolling_index = 3,
            Keycode::Num5 => self.scrolling_index = 4,
            _ => (),
        }
        assert!(self.scrolling_index < NUM_VARS);
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: i32, y: i32) {
        let var: &mut f32 = &mut self.sig_config.x[self.scrolling_index];
        if y < 0 {
            *var *= 0.9
        } else if y > 0 {
            *var /= 0.9
        }
        self.scrolling_text[self.scrolling_index] = None;
    }
}

pub fn main() {
    println!(
        "Welcome to space! Fly around and play with your view projection\n\
         --------------------------------------\n\
         use WASD to accelerate your viewpoint through space.\n\
         use SPACE to come to a stop\n\
         use BACKSPACE to jump to (0,0)\n\
         use number keys 1,2,3... to select a variable\n\
         use mousewheel to scale selected variable up and down\n"
    );
    let c = conf::Conf::new();
    let mut ctx = &mut Context::load_from_conf("super_simple", "ggez", c).unwrap();
    let state = &mut MainState::new(ctx).unwrap();

    // println!("sig {:?} to {:?} (should be 0.0 to 1.0)", state.sig(0.0), state.sig(999999999.0));
    ggez::graphics::set_background_color(&mut ctx, BLACK);
    event::run(ctx, state).unwrap();
}
