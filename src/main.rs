use std::collections::HashMap;
use std::error;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use boxcars::{ActorId, Attribute, Replay, RigidBody};
use clap::Parser;
use glutin_window::{GlutinWindow, OpenGL};
use graphics::ellipse::circle;
use opengl_graphics::GlGraphics;
use piston::{Button, ButtonEvent, ButtonState, EventLoop, Events, EventSettings, Key, RenderArgs, RenderEvent, UpdateArgs, UpdateEvent, WindowSettings};

const STANDARD_MAP_HEIGHT: f64 = 10280.0;
const STANDARD_MAP_WIDTH: f64 = 8240.0;
const SCALE_FACTOR: f64 = 10.;
const STANDARD_GOAL_SIZE: f64 = 0.;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to replay file to visualize.
    #[arg(short, long)]
    replay: PathBuf,
}

enum Entity {
    Player(Team),
    Ball,
}

struct ActiveActor {
    rigid_body: boxcars::RigidBody,
    entity: Entity,
    actor_id: ActorId,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Team {
    Orange,
    Blue,
}

#[derive(Debug)]
struct PlayerDetails {
    name: String,
    color: [f32; 4],
    car: RigidBody,
}

struct ReplayVis {
    gl: GlGraphics,
    replay: Replay,
    frame_index: usize,

    orange_team: HashMap<ActorId, PlayerDetails>,
    blue_team: HashMap<ActorId, PlayerDetails>,
    ball: Option<RigidBody>,
}

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const GREY: [f32; 4] = [0.0, 153.0/256.0, 51.0/256.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const PURPLE: [f32; 4] = [0.5, 0.0, 0.5, 1.0];

const ORANGE: [[f32; 4]; 4] = [
    [245.0 / 256.0, 146.0 / 256.0, 0.0, 1.0],
    [224.0 / 256.0, 81.0 / 256.0, 0.0, 1.0],
    [250.0 / 256.0, 40.0 / 256.0, 13.0 / 256.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
];
const BLUE: [[f32; 4]; 4] = [
    [0.0, 45.0 / 256.0, 245.0 / 256.0, 1.0],
    [68.0 / 256.0, 11.0 / 256.0, 222.0 / 256.0, 1.0],
    [0.0, 141.0 / 256.0, 224.0 / 256.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
];



impl ReplayVis {
    fn new(gl: GlGraphics, replay: Replay) -> Self {
        let mut this = Self {
            gl,
            replay,
            frame_index: 0,

            orange_team: Default::default(),
            blue_team: Default::default(),
            ball: None,
        };
        this.prepare();
        this
    }
    fn prepare(&mut self) {

    }
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        self.gl.draw(args.viewport(), |c, gl| {
            clear(GREY, gl);

            for player in self.orange_team.values() {
                let entity_location = circle(
                    (player.car.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                    (player.car.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                    6.0,
                );

                rectangle(player.color, entity_location, c.transform, gl);
            }
        })
    }

    fn move_frame(&mut self, frame: i32) {
        let total_frames =  self.replay.network_frames.as_ref().unwrap().frames.len();
        if frame < 0 && self.frame_index < frame.unsigned_abs() as usize {
            self.frame_index = total_frames - (frame.unsigned_abs() as usize - self.frame_index);
            return;
        }

        if frame > 0 && self.frame_index + frame as usize > total_frames {
            self.frame_index = (self.frame_index + frame as usize) - total_frames;
            return;
        }

        if frame < 0 {
            self.frame_index -= frame.unsigned_abs() as usize;
            return;
        }

        if frame > 0 {
            self.frame_index += frame as usize;
        }
    }

    fn update(&mut self, _args: &UpdateArgs) {
        let frames = &self.replay.network_frames.as_ref().unwrap().frames;
        if self.frame_index >= frames.len() {
            self.frame_index = 0;
            self.orange_team.clear();
            self.blue_team.clear();
        }
        let frame = &frames[self.frame_index];

        for actor in &frame.new_actors {

        }

        for actor in &frame.updated_actors {


            if let Attribute::DemolishFx(demo) = &actor.attribute {
                let victim = demo.victim;

            }
            if let Attribute::Demolish(demo) = &actor.attribute {
                let victim = demo.victim;

            }
        }

        for actor in &frame.deleted_actors {

        }

        self.frame_index += 1;
    }
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Args::parse();
    let mut f = BufReader::new(fs::File::open(&args.replay)?);

    let mut replay_data = vec![];
    let _read_bytes = f.read_to_end(&mut replay_data)?;
    let replay = boxcars::ParserBuilder::new(&replay_data)
        .always_check_crc()
        .must_parse_network_data()
        .parse()?;

    let opengl = OpenGL::V4_5;
    let mut window: GlutinWindow = WindowSettings::new(
        "Replay",
        [
            STANDARD_MAP_WIDTH / SCALE_FACTOR,
            (STANDARD_MAP_HEIGHT + STANDARD_GOAL_SIZE) / SCALE_FACTOR,
        ],
    )
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()?;


    let mut viz = ReplayVis::new(GlGraphics::new(opengl), replay);

    let mut ups = 120;
    let mut events = Events::new(EventSettings::new().max_fps(60).ups(ups));
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            viz.render(&args);
        }

        if let Some(args) = e.update_args() {
            viz.update(&args);
        }

        if let Some(args) = e.button_args() {
            if args.state != ButtonState::Press {
                continue;
            }

            match args.button {
                Button::Keyboard(Key::Space) if ups > 0 => {
                    events.set_ups(0);
                    ups = 0;
                }
                Button::Keyboard(Key::Space) if ups == 0 => {
                    events.set_ups(120);
                    ups = 120;
                }
                Button::Keyboard(Key::Left) => {
                    viz.move_frame(-150)
                }
                Button::Keyboard(Key::Right) => {
                    viz.move_frame(150)
                }
                Button::Keyboard(Key::Up) => {
                    ups += 10;
                    events.set_ups(ups);
                }
                Button::Keyboard(Key::Down) => {
                    ups -= 10;
                    events.set_ups(ups);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
