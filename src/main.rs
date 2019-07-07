extern crate ggez;
use rand::distributions::Standard;
use core::f32::consts::E;
use ggez::graphics::DrawParam;
use ggez::graphics::{Point2, Vector2, Color};
use rand::{Rng, SeedableRng};
use ggez::*;

struct MainState {
    me: Instancedata,
    planets: Vec<Instancedata>,
    planet_sprite: graphics::Image,
    window_dims: Point2,
    sig_config: SigConfig,
}

#[derive(Debug, Copy, Clone)]
struct Instancedata {
    pos: Point2,
    scale: f32,
}


#[derive(Debug, Copy, Clone)]
struct SigConfig {
	a: f32,
}
impl Default for SigConfig {
	fn default() -> Self {
		Self {
			a: 0.04,
		}
	}
}



impl MainState {
	fn sig(&self, dist: f32) -> f32 {
		let SigConfig {a} = self.sig_config;
		(2. / (
			1.0 + E.powf(-2. - dist*a) // higher mult pushes more to edges
		) - 1.4621172-0.29947686) / 0.23840594
		// 2. / (
		// 	1.0 + E.powf(-dist*0.08)
		// ) - 1.
	}
    fn new(ctx: &mut Context) -> GameResult<MainState> {

        	// println!("sig {:?} to {:?} (should be 0.0 to 1.0)", self.sig(0.0), self.sig(999999999.0));


        let mut planets = vec![];
        const SQN: usize = 10;
        let mut rng = rand::rngs::StdRng::from_seed([0;32]);
        let mut clos = || rng.sample::<f32,_>(Standard).powf(3.0) * 600.0 * if rng.gen() {-1.} else {1.};

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
        let me = Instancedata {
            pos: Point2::new(0., 0.),
            scale: 1.,
        };
        let s = MainState {
            me,
            planets,
            planet_sprite,
            window_dims,
            sig_config: Default::default(),
        };
        Ok(s)
    }
}

impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
    	self.me.pos += Vector2::new(0.1, 0.1);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx);
        
        for i in self.planets.iter() {
            let rel: Point2 = i.pos - self.me.pos.coords;
            let dist = (rel[0].powf(2.0) + rel[1].powf(2.0)).sqrt();
         //    let sig = 2.0 / (
        	// 	1.0 + E.powf(-dist*0.08)
        	// ) - 1.0;
        	let sigged = self.sig(dist);
            let newdist: f32 = sigged * 250.0;
            let dest = rel / dist * newdist + (self.window_dims.coords / 2.0);
            let s = (i.scale * (1.0 - sigged)).max(0.005);
            let dp = DrawParam {
                dest,
                scale: Point2::new(s,s),
                offset: Point2::new(0.5, 0.5),
                ..Default::default()
            };
            graphics::draw_ex(ctx, &self.planet_sprite, dp)?;
        }

        graphics::present(ctx);
        Ok(())
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: i32, y: i32) {
    	if y < 0 {
    		self.sig_config.a *= 0.9
    	} else if y > 0 {
    		self.sig_config.a /= 0.9
    	}
    }
}



pub fn main() {
    let c = conf::Conf::new();
    let mut ctx = &mut Context::load_from_conf("super_simple", "ggez", c).unwrap();
    let state = &mut MainState::new(ctx).unwrap();
    ggez::graphics::set_background_color(&mut ctx, Color::from_rgba(0,0,0,0));
    event::run(ctx, state).unwrap();
}
