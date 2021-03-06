extern crate ggez;
use core::f32::consts::E;
use ggez::graphics::DrawMode;
use ggez::graphics::Font;
use ggez::graphics::MeshBuilder;
use ggez::graphics::TextFragment;
use ggez::input::keyboard::KeyCode;
use ggez::input::keyboard::KeyMods;

// use ggez::event::Mod;
use ggez::graphics::Color;
use ggez::graphics::DrawParam;
use ggez::graphics::{BLACK, WHITE};
use ggez::*;
use itertools::izip;
use rand::distributions::Standard;
use rand::{Rng, SeedableRng};
use std::path;

type Point2 = ggez::nalgebra::Point2<f32>;
type Vector2 = ggez::nalgebra::Vector2<f32>;

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
const TRAJ_PTS_CAP: usize = 8_000;
const TRAJ_SCALING: usize = 20;

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
    trajjer: Trajjer,
    is_fullscreen: bool,
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
            x: [0.02, -0.04, 0.6, 0.22, 1.5, 100.],
        }
    }
}
impl SigConfig {
    fn traj_len(&self) -> usize {
        (self.x[5] as usize).min(TRAJ_PTS_CAP - 100).max(8)
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

fn rand_pt<R: Rng>(rng: &mut R) -> Point2 {
    let mut clos =
        || rng.sample::<f32, _>(Standard).powf(3.0) * 900.0 * if rng.gen() { -1. } else { 1. };
    Point2::new(clos(), clos())
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
    fn grav_accel<'a>(planets: impl Iterator<Item = &'a Instancedata>, pt: Point2) -> Vector2 {
        let mut v = Vector2::new(0., 0.);
        for p in planets {
            let impulse = p.pos - pt.coords;
            let len = length(impulse) + 2.0;
            let new_len = 1.0 / (len * len);
            v += impulse.coords / len * new_len * (p.scale * 2.);
        }
        v * 0.5
    }
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let mut planets: Vec<Instancedata> = vec![];
        const SQN: usize = 5;
        use rand::rngs::StdRng;
        let mut rng = StdRng::from_seed([0; 32]);
        let rng = &mut rng;
        for _i in 0..SQN {
            for _j in 0..SQN {
                let colliding =
                    |pt1: Point2| planets.iter().any(|pt2| length(pt1 - pt2.pos.coords) < 40.);
                let pos = loop {
                    let pos = rand_pt(rng);
                    if !colliding(pos) {
                        break pos;
                    }
                };
                let dp = Instancedata {
                    pos,
                    scale: rng.gen::<f32>() + 0.05,
                };
                planets.push(dp);
            }
        }

        let mut planet_sprite = graphics::Image::new(ctx, "/ball.png")?;
        planet_sprite.set_filter(graphics::FilterMode::Nearest);
        let d = {
            let rect = ggez::graphics::screen_coordinates(ctx);
            [rect.w, rect.h]
        };
        let window_dims = Point2::new(d[0] as f32, d[1] as f32);
        let me = Me {
            data: Instancedata {
                pos: Point2::new(0., 0.),
                scale: 1.,
            },
            velocity: Point2::new(0., 0.),
        };
        let s = MainState {
            is_fullscreen: false,
            trajjer: Trajjer::new(),
            me,
            planets,
            planet_sprite,
            window_dims,
            sig_config: Default::default(),
            x_pressed: None,
            y_pressed: None,
            press_accel: Point2::new(0., 0.),
            scrolling_index: 6,
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

/*
buf data persists long term. contains UNPROJECTED trajectory in slice. this
useful region is called "Valuable".
head..buf.len(). first 0..head are junk data.

every 1/TRAJ_SCALING ticks, the head advances 1 step, shrinking the valuable slice.
when valuable slice becomes too small, it is shifted left, and its tail is extended
to buf.len(). head is usually NOT shifted to zero, because this would make
the tail super long.

every time the pressed_velocity is changed, you should call buf_clear such
that the buffer contents are entirely recomputed.
--------------------
buf_projected is a reused allocation but its overwritten every tick.
its indices correspond with the other buf, and its just the visual projection
applied to those, since the projection changes, even when the trajectory coords
do not.

after every tick, you should call project_slice() to update the projections.
this call will compute sequentially or in parallel (with rayon) depending
on the length of the slice
*/
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
            cycler: TRAJ_SCALING - 1,
            head: TRAJ_PTS_CAP,
        }
    }
    fn project_slice(&mut self, sig_config: &SigConfig, window_dims: Point2, me_pos: Point2) {
        let l = sig_config.traj_len();
        let range = self.head..(self.head + l);
        let src = &self.buf[range.clone()];
        let dest = &mut self.buf_projected[range];
        // let a = std::time::Instant::now();
        if sig_config.traj_len() > 900 {
            use rayon::prelude::*;
            src.par_iter().zip(dest.par_iter_mut()).for_each(|(s, d)| {
                *d = sig_config.project(*s, me_pos, window_dims).0;
            });
        } else {
            src.iter().zip(dest.iter_mut()).for_each(|(s, d)| {
                *d = sig_config.project(*s, me_pos, window_dims).0;
            });
        }
        // println!("{:?}", a.elapsed());
    }
    fn get_projected_slice(&self, sig_config: &SigConfig) -> &[Point2] {
        let l = sig_config.traj_len();
        let range = self.head..(self.head + l);
        &self.buf_projected[range]
    }
    fn clear_traj(&mut self) {
        // trigger recompute
        self.head = TRAJ_PTS_CAP;
    }
    fn tick(
        &mut self,
        sig_config: &SigConfig,
        planets: &Vec<Instancedata>,
        press_accel: Point2,
        me: &Me,
    ) {
        // println!("TICK");
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

            let left = self.head.checked_sub(min_len * 2 + 100).unwrap_or(0);
            let compute_from: usize = if valuable_range.len() > 0 {
                self.buf.copy_within(valuable_range.clone(), left);
                prev_ = *self.buf.as_slice().last().unwrap();
                vel_ = self.last_vel;
                valuable_range.len() + left
            } else {
                self.buf[left] = me.data.pos;
                prev_ = self.buf[left];
                vel_ = me.velocity;
                left + 1
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

impl ggez::event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // 1. compute keypress acceleration
        // 2. update velocity (keypress + gravity)
        self.me.velocity +=
            self.press_accel.coords + Self::grav_accel(self.planets.iter(), self.me.data.pos);

        // 3. update position
        self.me.data.pos += self.me.velocity.coords;

        self.trajjer
            .tick(&self.sig_config, &self.planets, self.press_accel, &self.me);
        self.trajjer
            .project_slice(&self.sig_config, self.window_dims, self.me.data.pos);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, BLACK);

        let x: &[f32; NUM_VARS] = &self.sig_config.x;

        // ggez::graphics::set_color(ctx, WHITE)?;
        for i in self.planets.iter() {
            let (dest, sigged) = self
                .sig_config
                .project(i.pos, self.me.data.pos, self.window_dims);
            let s = (i.scale * x[3] * (1.0 - sigged).powf(x[2])).max(0.005);
            let dp = DrawParam {
                dest: dest.into(),
                scale: [s; 2].into(),
                offset: [0.5; 2].into(),
                ..Default::default()
            };
            graphics::draw(ctx, &self.planet_sprite, dp)?;
        }

        for (i, value, font) in izip!(
            0..,
            self.sig_config.x.iter(),
            self.scrolling_text.iter_mut()
        ) {
            let f = font.get_or_insert_with(|| {
                let s = format!("{}", value);
                ggez::graphics::Text::new(TextFragment::from(s))
            });
            let col = if i == self.scrolling_index {
                PINK
            } else {
                WHITE
            };
            let dp = DrawParam {
                dest: [20., (i * 30) as f32 + 20.].into(),
                scale: [1.; 2].into(),
                color: col,
                ..Default::default()
            };
            ggez::graphics::draw(ctx, f, dp)?;
        }

        let tu = self.sig_config.traj_len();
        // let ts = &self.trajectory[..];
        let drawmode = DrawMode::stroke(1.);
        let ts = self.trajjer.get_projected_slice(&self.sig_config);
        let m = MeshBuilder::new()
            .polyline(drawmode, &ts[0..=tu / 4], PINK)?
            .build(ctx)?;
        ggez::graphics::draw(ctx, &m, DrawParam::default())?;
        let m = MeshBuilder::new()
            .polyline(drawmode, &ts[tu * 1 / 4..=tu * 2 / 4], PINK2)?
            .build(ctx)?;
        ggez::graphics::draw(ctx, &m, DrawParam::default())?;
        let m = MeshBuilder::new()
            .polyline(drawmode, &ts[tu * 2 / 4..=tu * 3 / 4], PINK3)?
            .build(ctx)?;
        ggez::graphics::draw(ctx, &m, DrawParam::default())?;
        let m = MeshBuilder::new()
            .polyline(drawmode, &ts[tu * 3 / 4..tu], PINK4)?
            .build(ctx)?;
        ggez::graphics::draw(ctx, &m, DrawParam::default())?;

        graphics::present(ctx)
    }

    fn key_up_event(&mut self, _ctx: &mut Context, keycode: KeyCode, _keymod: KeyMods) {
        match keycode {
            KeyCode::D => {
                self.x_pressed.bump_left();
                self.recompute_press_accel()
            }
            KeyCode::A => {
                self.x_pressed.bump_right();
                self.recompute_press_accel()
            }
            KeyCode::S => {
                self.y_pressed.bump_left();
                self.recompute_press_accel()
            }
            KeyCode::W => {
                self.y_pressed.bump_right();
                self.recompute_press_accel()
            }
            _ => (),
        }
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        _keymod: KeyMods,
        repeat: bool,
    ) {
        if repeat {
            return;
        }
        match keycode {
            KeyCode::Escape => ggez::event::quit(ctx),
            KeyCode::Space => {
                self.me.velocity = Point2::new(0., 0.);
                self.recompute_press_accel()
            }
            KeyCode::Back => {
                self.me.data.pos = rand_pt(&mut rand::thread_rng());
                self.recompute_press_accel()
            }
            KeyCode::A => {
                self.x_pressed.bump_left();
                self.recompute_press_accel()
            }
            KeyCode::D => {
                self.x_pressed.bump_right();
                self.recompute_press_accel()
            }
            KeyCode::W => {
                self.y_pressed.bump_left();
                self.recompute_press_accel()
            }
            KeyCode::S => {
                self.y_pressed.bump_right();
                self.recompute_press_accel()
            }
            KeyCode::F4 => {
                let set_to = match self.is_fullscreen {
                    false => ggez::conf::FullscreenType::Windowed,
                    true => ggez::conf::FullscreenType::Desktop,
                };
                self.is_fullscreen = !self.is_fullscreen;
                ggez::graphics::set_fullscreen(ctx, set_to).unwrap()
            }
            KeyCode::Snapshot => match graphics::screenshot(ctx) {
                Ok(img) => {
                    for i in 0.. {
                        let f = format!("space_screencap_{}.png", i);
                        let path = path::Path::new(&f);
                        if !path.exists() {
                            img.encode(ctx, graphics::ImageFormat::Png, path)
                                .expect("K")
                        }
                    }
                }
                Err(_) => println!("print screen failed!"),
            },
            KeyCode::Key0 => self.scrolling_index = 0,
            KeyCode::Key1 => self.scrolling_index = 1,
            KeyCode::Key2 => self.scrolling_index = 2,
            KeyCode::Key3 => self.scrolling_index = 3,
            KeyCode::Key4 => self.scrolling_index = 4,
            KeyCode::Key5 => self.scrolling_index = 5,
            KeyCode::Key6 => self.scrolling_index = 6,
            _ => (),
        }
        // assert!(self.scrolling_index < NUM_VARS);
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) {
        if self.scrolling_index == 6 {
            // special case
            if y < 0. {
                self.sig_config.x[0] *= 0.9;
                self.sig_config.x[3] *= 0.9;
            } else {
                self.sig_config.x[0] /= 0.9;
                self.sig_config.x[3] /= 0.9;
            }
            self.scrolling_text[0] = None;
            self.scrolling_text[3] = None;
            return;
        }
        let var: &mut f32 = &mut self.sig_config.x[self.scrolling_index];
        if y < 0. {
            *var *= 0.9
        } else if y > 0. {
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
    let (mut ctx, mut event_loop) = ContextBuilder::new("my_game", "Cool Game Author")
        .build()
        .expect("aieee, could not create ggez context!");

    // Create an instance of your event handler.
    // Usually, you should provide it with the Context object to
    // use when setting your game up.
    let mut my_game = MainState::new(&mut ctx).unwrap();

    // Run!
    println!("HERE WE GO");
    match event::run(&mut ctx, &mut event_loop, &mut my_game) {
        Ok(_) => println!("Exited cleanly."),
        Err(e) => println!("Error occured: {}", e),
    }
}
