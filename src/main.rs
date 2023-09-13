use std::collections::{HashMap, HashSet};
use std::error;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use boxcars::{Attribute, Frame, Replay};
use clap::Parser;
use glutin_window::{GlutinWindow, OpenGL};
use graphics::ellipse::circle;
use opengl_graphics::GlGraphics;
use piston::{Button, ButtonArgs, ButtonEvent, ButtonState, Event, EventLoop, Events, EventSettings, GenericEvent, Input, Key, RenderArgs, RenderEvent, UpdateArgs, UpdateEvent, Window, WindowSettings};
use piston::Key::P;

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
    Player,
    Ball,
}

struct ActiveActor {
    rigid_body: boxcars::RigidBody,
    entity: Entity,
}

enum Team {
    Orange,
    Blue,
}

struct PlayerDetails {
    name: String,
    actor_id: boxcars::ActorId,
    team_id: Team,
}

struct ReplayVis {
    gl: GlGraphics,
    replay: Replay,
    frame_index: usize,

    player_actors: HashSet<boxcars::ActorId>,
    active_actors_map: HashMap<i32, boxcars::UpdatedAttribute>,
    active_actor_locations: HashMap<i32, ActiveActor>,

    active_actor_object_id: usize,
    active_actor_team_id: usize,
    player_car_object_id: usize,
}

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const PURPLE: [f32; 4] = [0.5, 0.0, 0.5, 1.0];

const ORANGE: [[f32; 4]; 3] = [
    [245.0 / 256.0, 146.0 / 256.0, 0.0, 1.0],
    [224.0 / 256.0, 81.0 / 256.0, 0.0, 1.0],
    [250.0 / 256.0, 40.0 / 256.0, 13.0 / 256.0, 1.0],
];

impl ReplayVis {
    fn new(gl: GlGraphics, replay: Replay) -> Self {
        let mut this = Self {
            gl,
            replay,
            frame_index: 0,
            player_actors: Default::default(),
            active_actors_map: Default::default(),
            active_actor_locations: Default::default(),
            active_actor_object_id: 0,
            active_actor_team_id: 0,
            player_car_object_id: 0,
        };
        this.prepare();
        this
    }
    fn prepare(&mut self) {
        for (id, obj) in self.replay.objects.iter().enumerate() {
            match obj.as_ref() {
                "Engine.PlayerReplicationInfo:Team" => {
                    self.active_actor_team_id = id;
                }
                "Engine.PlayerReplicationInfo:PlayerName" => {
                    self.active_actor_object_id = id;
                }
                "Archetypes.Car.Car_Default" => {
                    self.player_car_object_id = id;
                }
                _ => {}
            }
        }
    }
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        self.gl.draw(args.viewport(), |c, gl| {
            clear(GREEN, gl);

            for (_actor, actor) in &self.active_actor_locations {
                let entity_location = circle(
                    (actor.rigid_body.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                    (actor.rigid_body.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                    6.0,
                );

                let color = match actor.entity {
                    Entity::Player => RED,
                    Entity::Ball => PURPLE,
                };
                rectangle(color, entity_location, c.transform, gl);
            }
        })
    }

    fn update(&mut self, _args: &UpdateArgs) -> bool {
        let frames = &self.replay.network_frames.as_ref().unwrap().frames;
        if self.frame_index >= frames.len() {
            return false;
        }
        let frame = &frames[self.frame_index];

        for actor in &frame.new_actors {
            if actor.object_id.0 as usize == self.player_car_object_id {
                self.player_actors.insert(actor.actor_id);
            }
            // println!("Actor: {:?}\nObj: {:?}", actor, self.replay.objects[actor.object_id.0 as usize]);
        }

        for actor in &frame.updated_actors {
            if self.replay.objects[actor.object_id.0 as usize].starts_with("Engine.PlayerReplicationInfo") {
                println!("{} {}: {:?}", actor.object_id.0, self.replay.objects[actor.object_id.0 as usize], actor);
            }
            if actor.object_id.0 as usize == self.active_actor_team_id {}
            if actor.object_id.0 as usize == self.active_actor_object_id {
                if let Attribute::String(player_name) = &actor.attribute {
                    self.player_actors.insert(actor.actor_id);
                }
            }

            if let Attribute::ActiveActor(_active) = &actor.attribute {
                if !self.active_actors_map.contains_key(&actor.actor_id.0) {
                    self.active_actors_map.insert(actor.actor_id.0, actor.clone());
                }
            }

            if let Attribute::RigidBody(body) = &actor.attribute {
                match self.active_actors_map.get(&actor.actor_id.0) {
                    Some(&boxcars::UpdatedAttribute { attribute: Attribute::ActiveActor(_actor_id), .. }) => {
                        println!("{} {}: {:?}", actor.object_id.0, self.replay.objects[actor.object_id.0 as usize], actor);

                        self.active_actor_locations.insert(actor.actor_id.0, ActiveActor {
                            rigid_body: *body,
                            entity: self
                                .player_actors
                                .get(&_actor_id.actor)
                                .map(|player_name| Entity::Player)
                                .unwrap_or(Entity::Ball),
                        });
                    }
                    _ => {}
                }
            }

            if let Attribute::DemolishFx(demo) = &actor.attribute {
                let victim = demo.victim;
                self.player_actors.remove(&victim);
                self.active_actor_locations.remove(&victim.0);
            }
            if let Attribute::Demolish(demo) = &actor.attribute {
                let victim = demo.victim;
                self.player_actors.remove(&victim);
                self.active_actor_locations.remove(&victim.0);
            }
        }

        self.frame_index += 1;
        return true;
    }
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Args::parse();
    let mut f = BufReader::new(fs::File::open(&args.replay)?);

    let mut replay_data = vec![];
    let _read_bytes = f.read_to_end(&mut replay_data)?;
    let replay = boxcars::ParserBuilder::new(&replay_data)
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
            if !viz.update(&args) {
                window.set_should_close(true);
            }
        }

        if let Some(ButtonArgs { button: Button::Keyboard(Key::Space), state: ButtonState::Press, .. }) = e.button_args() {
            if ups == 120 {
                events.set_ups(0);
                ups = 0;
            } else {
                events.set_ups(120);
                ups = 120;
            }
        }
    }

    Ok(())
}
