use std::collections::HashMap;
use std::error;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use boxcars::{Attribute, Replay};
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
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Team {
    Orange,
    Blue,
}

#[derive(Debug)]
struct PlayerDetails {
    name: String,
    actor_id: boxcars::ActorId,
    team_id: Team,
}

struct ReplayVis {
    gl: GlGraphics,
    replay: Replay,
    frame_index: usize,

    player_actors: HashMap<boxcars::ActorId, PlayerDetails>,
    active_actors_map: HashMap<boxcars::ActorId, boxcars::UpdatedAttribute>,
    active_actor_locations: HashMap<boxcars::ActorId, ActiveActor>,

    active_actor_object_id: usize,
    active_actor_team_id: usize,
    player_car_object_id: usize,

    team_0_object_id: usize,
    team_1_object_id: usize,

    team_0_actor_id: usize,
    team_1_actor_id: usize,
}

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const PURPLE: [f32; 4] = [0.5, 0.0, 0.5, 1.0];

const ORANGE: [[f32; 4]; 3] = [
    [245.0 / 256.0, 146.0 / 256.0, 0.0, 1.0],
    [224.0 / 256.0, 81.0 / 256.0, 0.0, 1.0],
    [250.0 / 256.0, 40.0 / 256.0, 13.0 / 256.0, 1.0],
];
const BLUE: [[f32; 4]; 3] = [
    [0.0, 45.0 / 256.0, 245.0 / 256.0, 1.0],
    [68.0 / 256.0, 11.0 / 256.0, 222.0 / 256.0, 1.0],
    [0.0, 141.0 / 256.0, 224.0 / 256.0, 1.0],
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
            team_0_object_id: 0,
            team_1_object_id: 0,
            team_1_actor_id: 0,
            team_0_actor_id: 0,
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
                "Archetypes.Teams.Team0" => {
                    self.team_0_object_id = id;
                }
                "Archetypes.Teams.Team1" => {
                    self.team_1_object_id = id;
                }
                _ => {}
            }
        }
        println!("Data: {:?}", self.replay.objects);
        println!("Data: {:?}", self.replay.names);
        println!("Data: {:?}", self.replay.class_indices);
        println!("Data: {:?}", self.replay.packages);
        println!("Data: {:?}", self.replay.properties);
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
                    Entity::Player(Team::Orange) => ORANGE[0],
                    Entity::Player(Team::Blue) => BLUE[0],
                    Entity::Ball => PURPLE,
                };
                rectangle(color, entity_location, c.transform, gl);
            }
        })
    }

    fn move_frame(&mut self, frame: i32) {
        // println!("Frame Changing: {:?}, {:?}", self.frame_index, frame);

        let total_frames =  self.replay.network_frames.as_ref().unwrap().frames.len();
        if frame < 0 && self.frame_index < frame.abs() as usize {
            self.frame_index = total_frames - (frame.abs() as usize - self.frame_index);
            return;
        }

        if frame > 0 && self.frame_index + frame as usize > total_frames {
            self.frame_index = (self.frame_index + frame as usize) - total_frames;
            return;
        }

        if frame < 0 {
            self.frame_index -= frame.abs() as usize;
            return;
        }

        if frame > 0 {
            self.frame_index += frame as usize;
            return;
        }
    }

    fn update(&mut self, _args: &UpdateArgs) {
        let frames = &self.replay.network_frames.as_ref().unwrap().frames;
        if self.frame_index >= frames.len() {
            self.frame_index = 0;
            self.player_actors.clear();
            self.active_actor_locations.clear();
            self.active_actors_map.clear();
        }
        let frame = &frames[self.frame_index];

        for actor in &frame.new_actors {
            // if actor.actor_id.0 == 2 || actor.actor_id.0 == 10 {
            //     println!("Actor: {:?}", actor);
            //     println!("Object: {:?}", self.replay.objects[actor.object_id.0 as usize]);
            // }
            if actor.object_id.0 as usize == self.team_0_object_id {
                self.team_0_actor_id = actor.actor_id.0 as usize
            }
            if actor.object_id.0 as usize == self.team_1_object_id {
                self.team_1_actor_id = actor.actor_id.0 as usize
            }
            if actor.object_id.0 as usize == self.player_car_object_id {
                self.player_actors.insert(actor.actor_id, PlayerDetails {
                    actor_id: actor.actor_id,

                    team_id: Team::Orange,
                    name: "".to_string(),
                });
            }
            // println!("Actor: {:?}\nObj: {:?}", actor, self.replay.objects[actor.object_id.0 as usize]);
        }

        for actor in &frame.updated_actors {
            if actor.actor_id.0 == 2 || actor.actor_id.0 == 10 {
                println!("Actor: {:?}", actor);
                println!("Object: {:?}", self.replay.objects[actor.object_id.0 as usize]);
            }
            // if self.replay.objects[actor.object_id.0 as usize].starts_with("Engine.PlayerReplicationInfo") {
            //     println!("{} {}: {:?}", actor.object_id.0, self.replay.objects[actor.object_id.0 as usize], actor);
            // }
            if actor.object_id.0 as usize == self.active_actor_team_id {
                if let Attribute::ActiveActor(boxcars::ActiveActor{ actor: id, ..}) = actor.attribute {
                    self.player_actors.entry(actor.actor_id).and_modify(|f| {


                        if id.0 as usize == self.team_0_actor_id {
                            f.team_id = Team::Blue;
                        } else {
                            f.team_id == Team::Orange;
                        }
                    }).or_insert(PlayerDetails {
                        name: "".to_string(),
                        actor_id: actor.actor_id,
                        team_id: Team::Orange,
                    });
                }

            }
            if actor.object_id.0 as usize == self.active_actor_object_id {
                if let Attribute::String(player_name) = &actor.attribute {
                    self.player_actors.entry(actor.actor_id).and_modify(|f| {
                        f.name = player_name.to_string()
                    }).or_insert(PlayerDetails {
                        name: player_name.to_string(),
                        actor_id: actor.actor_id,
                        team_id: Team::Orange,
                    });
                }
            }

            if let Attribute::ActiveActor(_active) = &actor.attribute {
                if !self.active_actors_map.contains_key(&actor.actor_id) {
                    self.active_actors_map.insert(actor.actor_id, actor.clone());
                }
            }

            if let Attribute::RigidBody(body) = &actor.attribute {
                match self.active_actors_map.get(&actor.actor_id) {
                    Some(&boxcars::UpdatedAttribute { attribute: Attribute::ActiveActor(_actor_id), .. }) => {
                        // println!("{} {}: {:?}", actor.object_id, self.replay.objects[actor.object_id as usize], actor);


                        self.active_actor_locations.insert(actor.actor_id, ActiveActor {
                            rigid_body: *body,
                            entity: self
                                .player_actors
                                .get(&_actor_id.actor)
                                .map(|player_name| Entity::Player(player_name.team_id))
                                .unwrap_or(Entity::Ball),
                        });
                    }
                    _ => {}
                }
            }

            if let Attribute::DemolishFx(demo) = &actor.attribute {
                let victim = demo.victim;
                self.player_actors.remove(&victim);
                self.active_actor_locations.remove(&victim);
                self.active_actors_map.remove(&victim);
            }
            if let Attribute::Demolish(demo) = &actor.attribute {
                let victim = demo.victim;
                self.player_actors.remove(&victim);
                self.active_actor_locations.remove(&victim);
                self.active_actors_map.remove(&victim);
            }
        }

        for actor in &frame.deleted_actors {
            self.player_actors.remove(actor);
            self.active_actor_locations.remove(actor);
            self.active_actors_map.remove(actor);

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
