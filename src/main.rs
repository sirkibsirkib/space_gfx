extern crate ggez;
use ggez::graphics::Vector2;
use std::path;
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
const PINK2: Color = Color {
    r: 0.9,
    g: 0.3,
    b: 0.3,
    a: 1.0,
};
const PINK3: Color = Color {
    r: 0.6,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};
const PINK4: Color = Color {
    r: 0.4,
    g: 0.12,
    b: 0.12,
    a: 1.0,
};

struct Me {
    data: Instancedata,
    velocity: Point2,
}


const ACCEL: f32 = 0.0004;
const TRAJ_PTS_CAP: usize = 5000;
const TRAJ_SCALING: usize = 30;


struct MainState {
    me: Me,
    planets: Vec<Instancedata>,
    planet_sprite: graphics::Image,
    window_dims: Point2,
    sig_config: SigConfig,
    x_pressed: Option<bool>,
    y_pressed: Option<bool>,
    press_accel: Point2,
    scrolling_index: usize,
    scrolling_text: Vec<Option<ggez::graphics::Text>>,
    trajectory: [Point2; TRAJ_PTS_CAP],
    trajjer: Trajjer,
}

const NUM_VARS: usize = 6;

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
            x: [0.02, -1.0, 1., 0.3, 1., 200.],
        }
    }
}
impl SigConfig {
    fn traj_len(&self) -> usize {
        (self.x[5] as usize).min(TRAJ_PTS_CAP-100).max(8)
    }
    fn sig(&self, dist: f32) -> f32 {
        let q = 2. / (1.0 + E.powf(-self.x[1])) - 1.;
        let divisor = 1.0 + E.powf(-self.x[1] - dist * self.x[0]);
        let intermediate = 2. / divisor - 1. - q;
        intermediate / (1. - q)
    }
    fn project(&self, origin: Point2, pt: Point2, window_dims: Point2) -> (Point2, f32) {
        let rel: Point2 = pt - origin.coords;
        let dist = length(rel);
        let sigged = self.sig(dist);
        let newdist: f32 = sigged * 250.0 * self.x[4];
        let projected = rel / dist * newdist + (window_dims.coords / 2.0);
        (projected, sigged)
    }
}

fn length(p: Point2) -> f32 {
    (p[0].powf(2.0) + p[1].powf(2.0)).sqrt()
}

impl MainState {
    fn recompute_press_accel(&mut self) {
        self.trajjer.clear_traj();
        self.press_accel = Point2::new(self.x_pressed.val(), self.y_pressed.val());
        if self.x_pressed.is_some() && self.y_pressed.is_some() {
            self.press_accel /= 1.4;
        }
    }
    /// given a point in space, returns a vector representing its acceleration
    fn grav_accel<'a>(planets: impl Iterator<Item=&'a Instancedata>, pt: Point2) -> Vector2 {
        let mut v = Vector2::new(0., 0.);
        for p in planets {
            let impulse = p.pos - pt.coords;
            let len = length(impulse) + 3.0;
            let new_len = 1.0 / (len*len);
            v += impulse.coords / len * new_len * (p.scale*2.);
        }
        v * 0.3
    }
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let mut planets: Vec<Instancedata> = vec![];
        const SQN: usize = 5;
        use rand::rngs::StdRng;
        let mut rng = StdRng::from_seed([0; 32]);
        let rng = &mut rng;
        let clos =
            |rng: &mut StdRng| rng.sample::<f32, _>(Standard).powf(3.0) * 900.0 * if rng.gen() { -1. } else { 1. };

        for _i in 0..SQN {
            for _j in 0..SQN {
                let colliding =
                    |pt1: Point2| planets.iter().any(|pt2| length(pt1 - pt2.pos.coords) < 60.);
                let pos = loop {
                    let pos = Point2::new(clos(rng), clos(rng));
                    if !colliding(pos) {
                        break pos;
                    }
                };
                let dp = Instancedata { pos, scale: rng.gen::<f32>() + 0.1 };
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
            trajjer: Trajjer::new(),
            trajectory: [Point2::new(0., 0.); TRAJ_PTS_CAP],
            me,
            planets,
            planet_sprite,
            window_dims,
            sig_config: Default::default(),
            x_pressed: None,
            y_pressed: None,
            press_accel: Point2::new(0., 0.),
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

impl Joystick for Option<bool> {
    fn val(self) -> f32 {
        match self {
            Some(false) => ACCEL,
            None => 0.0,
            Some(true) => -ACCEL,
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

struct Trajjer {
    buf: Vec<Point2>,
    head: usize,
    last_vel: Point2,

    buf_projected: Vec<Point2>,
    cycler: usize,
}
impl Trajjer {
    fn new() -> Self {
        Self {
            buf: vec![Point2::new(0., 0.); TRAJ_PTS_CAP],
            buf_projected: vec![Point2::new(0., 0.); TRAJ_PTS_CAP],
            last_vel: Point2::new(0., 0.),
            cycler: TRAJ_SCALING-1,
            head: TRAJ_PTS_CAP,
        }
    }
    fn project_slice(&mut self, sig_config: &SigConfig, window_dims: Point2, me_pos: Point2){
        let l = sig_config.traj_len();
        let range = self.head..(self.head + l);
        let src = &self.buf[range.clone()];
        let dest = &mut self.buf_projected[range];
        for (s, d) in src.iter().zip(dest.iter_mut()) {
            *d = sig_config.project(*s, me_pos, window_dims).0;
        }
    }
    fn get_projected_slice(&self, sig_config: &SigConfig) -> &[Point2] {
        let l = sig_config.traj_len();
        let range = self.head..(self.head + l);
        &self.buf_projected[range]
    }
    fn clear_traj(&mut self) {
         // trigger recompute
        self.head = TRAJ_PTS_CAP;
        self.cycler = TRAJ_SCALING-1;
    }
    fn tick(&mut self, sig_config: &SigConfig, planets: &Vec<Instancedata>, press_accel: Point2, me: &Me) {
        println!("TICK");
        self.cycler += 1;
        if self.cycler == TRAJ_SCALING {
            self.cycler = 0;
            self.head += 1;
        }
        let min_len = sig_config.traj_len();
        let valuable_range = self.head..self.buf.len();
        if valuable_range.len() < min_len {
            let mut prev_;
            let mut vel_;

            let left = self.head.checked_sub(min_len*2).unwrap_or(0);
            let compute_from: usize = if valuable_range.len() > 0 {
                self.buf.copy_within(valuable_range.clone(), left);
                prev_ = *self.buf.as_slice().last().unwrap();
                vel_ = self.last_vel;
                valuable_range.len() + left
            } else {
                self.buf[left] = me.data.pos;
                prev_ = self.buf[left];
                vel_ = me.velocity;
                left+1
            };
            for p in &mut self.buf[compute_from..] {
                for _ in 0..TRAJ_SCALING {
                    // 1. reuse keypress acceleration
                    // 2. update velocity (keypress + gravity)
                    vel_ += press_accel.coords + MainState::grav_accel(planets.iter(), prev_);

                    // 3. update position
                    prev_ += vel_.coords;
                }
                *p = prev_;
            }
            self.head = left;
            self.last_vel = vel_;
        }
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // 1. compute keypress acceleration
        // 2. update velocity (keypress + gravity)
        self.me.velocity += self.press_accel.coords + Self::grav_accel(self.planets.iter(), self.me.data.pos);

        // 3. update position
        self.me.data.pos += self.me.velocity.coords;

        self.trajjer.tick(&self.sig_config, &self.planets, self.press_accel, &self.me);
        self.trajjer.project_slice(&self.sig_config, self.window_dims, self.me.data.pos);

        {
            let mut prev_ = self.me.data.pos;
            let mut vel_ = self.me.velocity;
            self.trajectory[0] = self.window_dims * 0.5;
            let tu = self.sig_config.traj_len();
            for p in self.trajectory.iter_mut().skip(1).take(tu) {
                for _ in 0..TRAJ_SCALING {
                    // 1. reuse keypress acceleration
                    // 2. update velocity (keypress + gravity)
                    vel_ += self.press_accel.coords + Self::grav_accel(self.planets.iter(), prev_);

                    // 3. update position
                    prev_ += vel_.coords;
                }
                *p = self.sig_config.project(prev_, self.me.data.pos, self.window_dims).0;
            } 
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);

        let x: &[f32; NUM_VARS] = &self.sig_config.x;

        ggez::graphics::set_color(ctx, WHITE)?;
        for i in self.planets.iter() {
            let (dest, sigged) = self.sig_config.project(i.pos, self.me.data.pos, self.window_dims);
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

        let tu = self.sig_config.traj_len();
        // let ts = &self.trajectory[..];
        let ts = self.trajjer.get_projected_slice(&self.sig_config);
        ggez::graphics::set_color(ctx, PINK)?;
        graphics::line(ctx, &ts[0..=tu/4], 1.0)?;
        ggez::graphics::set_color(ctx, PINK2)?;
        graphics::line(ctx, &ts[tu*1/4..=tu*2/4], 1.0)?;
        ggez::graphics::set_color(ctx, PINK3)?;
        graphics::line(ctx, &ts[tu*2/4..=tu*3/4], 1.0)?;
        ggez::graphics::set_color(ctx, PINK4)?;
        graphics::line(ctx, &ts[tu*3/4..tu], 1.0)?;

        graphics::present(ctx);
        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: Keycode, _keymod: Mod, repeat: bool) {
        if repeat {
            return;
        }
        match keycode {
            Keycode::D => {self.x_pressed.bump_left(); self.recompute_press_accel()},
            Keycode::A => {self.x_pressed.bump_right(); self.recompute_press_accel()},
            Keycode::S => {self.y_pressed.bump_left(); self.recompute_press_accel()},
            Keycode::W => {self.y_pressed.bump_right(); self.recompute_press_accel()},
            _ => (),
        }
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, repeat: bool) {
        if repeat {
            return;
        }
        match keycode {
            Keycode::Escape => ctx.quit().unwrap(),
            Keycode::Space => self.me.velocity = Point2::new(0., 0.),
            Keycode::Backspace => self.me.data.pos = Point2::new(0., 0.),
            Keycode::A => {self.x_pressed.bump_left(); self.recompute_press_accel()},
            Keycode::D => {self.x_pressed.bump_right(); self.recompute_press_accel()},
            Keycode::W => {self.y_pressed.bump_left(); self.recompute_press_accel()},
            Keycode::S => {self.y_pressed.bump_right(); self.recompute_press_accel()},
            Keycode::F4 => {
                let q = ggez::graphics::is_fullscreen(ctx);
                ggez::graphics::set_fullscreen(ctx, !q).unwrap()
            },
            Keycode::PrintScreen => match graphics::screenshot(ctx) {
                Ok(img) => for i in 0.. {
                    let f = format!("space_screencap_{}.png", i);
                    let path = path::Path::new(&f);
                    if !path.exists() {
                        img.encode(ctx, graphics::ImageFormat::Png, path).expect("K")
                    }
                },
                Err(_) => println!("print screen failed!"),
            },
            Keycode::Num1 => self.scrolling_index = 0,
            Keycode::Num2 => self.scrolling_index = 1,
            Keycode::Num3 => self.scrolling_index = 2,
            Keycode::Num4 => self.scrolling_index = 3,
            Keycode::Num5 => self.scrolling_index = 4,
            Keycode::Num6 => self.scrolling_index = 5,
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
         use ESCAPE to exit\n\
         use SPACE to come to a stop\n\
         use BACKSPACE to jump to (0,0)\n\
         use number keys 1,2,3... to select a variable\n\
         use mousewheel to scale selected variable up and down\n\
         --------------------------------------\n\
         For example, try: (0.09, -3, 1.8, 3.9, 1)\n\
         and (0.01, -1, 1.8, 0.3, 1.11)\n"
    );
    let c = conf::Conf::new();
    let mut ctx = &mut Context::load_from_conf("super_simple", "ggez", c).unwrap();
    let state = &mut MainState::new(ctx).unwrap();
    ggez::graphics::set_background_color(&mut ctx, BLACK);
    event::run(ctx, state).unwrap();
}
