#![feature(if_let_guard)]
#![feature(let_chains)]

use std::collections::HashMap;
use std::error;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use boxcars::{ActorId, Attribute, ObjectId, Replay, RigidBody, UniqueId};
use clap::{Parser, ValueEnum};
use glutin_window::{GlutinWindow, OpenGL};
use graphics::ellipse::circle;
use graphics::{Context, Graphics};
use opengl_graphics::GlGraphics;
use piston::{
    Button, ButtonEvent, ButtonState, EventLoop, EventSettings, Events, Key, RenderArgs,
    RenderEvent, UpdateArgs, UpdateEvent, WindowSettings,
};
use voronoice::VoronoiBuilder;

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

    /// Count of network frames to process per second. Defaults to 120, which is the same speed a RL server will process a game
    #[arg(short, long)]
    ups: Option<u64>,

    /// What kind of display to show, whether it's points to show a point for each player, or voronoi to show a voronoi diagram
    #[arg(value_enum, short, long, default_value_t=DisplayType::POINTS)]
    display: DisplayType
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, ValueEnum)]
enum DisplayType {
    #[default]
    POINTS,
    VORONOI,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
enum Team {
    #[default]
    Orange,
    Blue,
}

#[derive(Debug, Default, Clone)]
struct PlayerDetails {
    platform_id: Option<UniqueId>,
    name: String,
    color: [f32; 4],
    car_actor_id: Option<ActorId>,
    team: Team,
}

struct ReplayVis<'a> {
    args: &'a Args,
    gl: GlGraphics,
    replay: Replay,
    frame_index: usize,

    player_actors: HashMap<ActorId, PlayerDetails>,
    car_actors: HashMap<ActorId, Option<RigidBody>>,
    ball: Option<RigidBody>,

    blue_team_count: usize,
    orange_team_count: usize,

    // Semi-Stable Actor IDs
    ball_actor_id: Option<ActorId>,
    orange_team_actor_id: Option<ActorId>,
    blue_team_actor_id: Option<ActorId>,

    // Object IDs
    ball_actor_object_id: Option<ObjectId>,
    blue_team_actor_object_id: Option<ObjectId>,
    orange_team_actor_object_id: Option<ObjectId>,
    player_car_object_id: Option<ObjectId>,
    player_name_object_id: Option<ObjectId>,
    player_id_object_id: Option<ObjectId>,
    player_team_object_id: Option<ObjectId>,
    car_object_id: Option<ObjectId>,
    player_object_id: Option<ObjectId>,
    rigid_body_moved_object_id: Option<ObjectId>,
}

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const GREY: [f32; 4] = [0.0, 153.0 / 256.0, 51.0 / 256.0, 1.0];
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

impl<'a> ReplayVis<'a> {
    fn new(args: &'a Args, gl: GlGraphics, replay: Replay) -> Self {
        let mut this = Self {
            args,
            gl,
            replay,
            frame_index: 0,

            player_actors: Default::default(),
            car_actors: Default::default(),
            ball: None,

            blue_team_count: 0,
            orange_team_count: 0,

            ball_actor_id: None,
            ball_actor_object_id: None,
            blue_team_actor_object_id: None,
            orange_team_actor_object_id: None,
            orange_team_actor_id: None,
            blue_team_actor_id: None,

            player_car_object_id: None,
            player_name_object_id: None,
            player_id_object_id: None,
            player_team_object_id: None,
            car_object_id: None,
            player_object_id: None,
            rigid_body_moved_object_id: None,
        };
        this.prepare();
        this
    }
    fn prepare(&mut self) {
        for (index, object_name) in self.replay.objects.iter().enumerate() {
            let id = Some(ObjectId(index as i32));
            match object_name.as_str() {
                "Archetypes.Ball.Ball_Default" => {
                    self.ball_actor_object_id = id;
                }
                "Archetypes.Teams.Team0" => {
                    self.orange_team_actor_object_id = id;
                }
                "Archetypes.Teams.Team1" => {
                    self.blue_team_actor_object_id = id;
                }
                "Engine.Pawn:PlayerReplicationInfo" => {
                    self.player_car_object_id = id;
                }
                "Engine.PlayerReplicationInfo:Team" => {
                    self.player_team_object_id = id;
                }
                "Engine.PlayerReplicationInfo:PlayerName" => {
                    self.player_name_object_id = id;
                }
                "Engine.PlayerReplicationInfo:PlayerID" => {
                    self.player_id_object_id = id;
                }
                "Archetypes.Car.Car_Default" => {
                    self.car_object_id = id;
                }
                "TAGame.Default__PRI_TA" => {
                    self.player_object_id = id;
                }
                "TAGame.RBActor_TA:ReplicatedRBState" => self.rigid_body_moved_object_id = id,
                _ => {}
            }
        }
    }

    fn render_dots(
        player_actors: &HashMap<ActorId, PlayerDetails>,
        car_actors: &HashMap<ActorId, Option<RigidBody>>,
        c: &Context,
        gl: &mut GlGraphics,
    ) {
        use graphics::*;

        for player in player_actors.values() {
            if let Some(car) = player.car_actor_id {
                if let Some(Some(r)) = car_actors.get(&car) {
                    let entity_location = circle(
                        (r.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                        (r.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                        6.0,
                    );

                    rectangle(player.color, entity_location, c.transform, gl);
                }
            }
        }
    }

    fn render_voronoi_naive(
        player_actors: &HashMap<ActorId, PlayerDetails>,
        car_actors: &HashMap<ActorId, Option<RigidBody>>,
        c: &Context,
        gl: &mut GlGraphics,
    ) {
        use graphics::*;
        use voronoice::*;

        #[derive(Hash, Copy, Clone, Eq, PartialEq)]
        struct HashablePoint {
            x: [u8; 8], y: [u8; 8]
        }

        let mut colors = HashMap::new();
        let mut pts = vec![];
        for player in player_actors.values() {
            if let Some(car) = player.car_actor_id {
                if let Some(Some(r)) = car_actors.get(&car) {
                    pts.push(Point {
                        x: r.location.x as f64,
                        y: r.location.y as f64,
                    });
                    colors.insert(HashablePoint{x: (r.location.x as f64).to_be_bytes(), y: (r.location.y as f64).to_be_bytes()}, player.color);
                }
            }
        }

        let voronoi = if let Some(builder) = VoronoiBuilder::default()
            .set_sites(pts)
            .set_bounding_box(BoundingBox::new_centered(
                STANDARD_MAP_WIDTH,
                STANDARD_MAP_HEIGHT,
            ))
            .build()
        {
            builder
        } else {
            return;
        };

        for cell in voronoi.iter_cells() {
            let mut vertices: Vec<[f64; 2]> = vec![];
            for point in cell.iter_vertices() {
                vertices.push([
                    (point.x + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                    (point.y + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                ]);
            }
            polygon(colors[&HashablePoint {x: cell.site_position().x.to_be_bytes(), y: cell.site_position().y.to_be_bytes()}], &vertices, c.transform, gl);
        }

        for player in player_actors.values() {
            if let Some(car) = player.car_actor_id {
                if let Some(Some(r)) = car_actors.get(&car) {
                    let entity_location = circle(
                        (r.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                        (r.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                        6.0,
                    );
                    let entity_background = circle(
                        (r.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                        (r.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                        10.0,
                    );

                    rectangle([0.0, 0.0, 0.0, 1.0], entity_background, c.transform, gl);
                    rectangle(player.color, entity_location, c.transform, gl);
                }
            }
        }
    }

    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let player_actors = self.player_actors.clone();
        let car_actors = self.car_actors.clone();
        self.gl.draw(args.viewport(), |c, gl| {
            clear(GREY, gl);

            match self.args.display {
                DisplayType::POINTS => {
                    ReplayVis::render_dots(&player_actors, &car_actors, &c, gl);
                },
                DisplayType::VORONOI => {
                    ReplayVis::render_voronoi_naive(&player_actors, &car_actors, &c, gl);
                }
            }

            if let Some(ball) = self.ball {
                let entity_location = circle(
                    (ball.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                    (ball.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                    6.0,
                );

                if self.args.display == DisplayType::VORONOI {
                    let entity_background = circle(
                        (ball.location.x as f64 + (STANDARD_MAP_WIDTH / 2.0)) / SCALE_FACTOR,
                        (ball.location.y as f64 + (STANDARD_MAP_HEIGHT / 2.0)) / SCALE_FACTOR,
                        10.0,
                    );

                    rectangle([0.0, 0.0, 0.0, 1.0], entity_background, c.transform, gl);
                }

                rectangle(PURPLE, entity_location, c.transform, gl);
            }
        })
    }

    fn move_frame(&mut self, frame: i32) {
        let total_frames = self.replay.network_frames.as_ref().unwrap().frames.len();
        if frame < 0 && self.frame_index < frame.unsigned_abs() as usize {
            self.frame_index = total_frames - (frame.unsigned_abs() as usize - self.frame_index);
            return;
        }

        if frame < 0 {
            self.frame_index -= frame.unsigned_abs() as usize;
            return;
        }

        if frame > 0 {
            for _ in 0..frame {
                self.update(&UpdateArgs { dt: 0.0 });
            }
        }
    }

    fn update(&mut self, _args: &UpdateArgs) {
        let frames = &self.replay.network_frames.as_ref().unwrap().frames;
        if self.frame_index >= frames.len() {
            self.frame_index = 0;
        }
        let frame = &frames[self.frame_index];

        for actor in &frame.new_actors {
            // When a ball is created
            if let Some(ball_actor_object_id) = self.ball_actor_object_id && actor.object_id == ball_actor_object_id {
                self.ball_actor_id = Some(actor.actor_id);
            }

            // When a car is created
            if let Some(car_actor_object_id) = self.car_object_id && actor.object_id == car_actor_object_id {
                self.car_actors.insert(actor.actor_id, None);
            }

            // When a team is created
            if let Some(team_actor_object_id) = self.blue_team_actor_object_id && actor.object_id == team_actor_object_id {
                self.blue_team_actor_id = Some(actor.actor_id);
            }

            // When a team is created
            if let Some(team_actor_object_id) = self.orange_team_actor_object_id && actor.object_id == team_actor_object_id {
                self.orange_team_actor_id = Some(actor.actor_id);
            }

            // When a player is created
            if let Some(player_actor_object_id) = self.player_object_id && actor.object_id == player_actor_object_id && !self.player_actors.contains_key(&actor.actor_id) {
                self.player_actors.insert(actor.actor_id, PlayerDetails {
                    platform_id: None,
                    name: "Unknown".to_string(),
                    color: PURPLE,
                    car_actor_id: None,
                    team: Team::Blue,
                });
            }
        }

        for actor in &frame.updated_actors {
            match actor.object_id {
                // When a player team is set or changed
                object_id if let Some(team_id) = self.player_team_object_id && object_id == team_id => {
                    if let Some(player) = self.player_actors.get_mut(&actor.actor_id) {
                        match actor.attribute {
                            Attribute::ActiveActor(actor) if self.orange_team_actor_id.is_some() && actor.actor.0 == self.orange_team_actor_id.unwrap().0 => {
                                player.team = Team::Orange;
                                if player.color == PURPLE {
                                    player.color = ORANGE[self.orange_team_count];
                                    self.orange_team_count += 1;
                                }
                            }
                            Attribute::ActiveActor(actor) if self.blue_team_actor_id.is_some() && actor.actor.0 == self.blue_team_actor_id.unwrap().0 => {
                                player.team = Team::Blue;
                                if player.color == PURPLE {
                                    player.color = BLUE[self.blue_team_count];
                                    self.blue_team_count += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // When a player name is set or changed
                object_id if let Some(player_name_id) = self.player_name_object_id && object_id == player_name_id => {
                    if let Some(player) = self.player_actors.get_mut(&actor.actor_id) {
                        if let Attribute::String(name) = &actor.attribute {
                            player.name = name.clone();
                        }
                    }
                }
                // When a player car is set or changed
                object_id if let Some(player_car_id) = self.player_car_object_id && object_id == player_car_id => {
                    if let Attribute::ActiveActor(player_actor_id) = &actor.attribute {
                        if let Some(player) = self.player_actors.get_mut(&player_actor_id.actor) {
                            player.car_actor_id = Some(actor.actor_id);
                        }
                    }
                }
                // When a player car is set or changed
                object_id if let Some(rigid_body_moved) = self.rigid_body_moved_object_id && object_id == rigid_body_moved => {
                    if let Some(car_body) = self.car_actors.get_mut(&actor.actor_id) {
                        if let Attribute::RigidBody(rigid_body) = &actor.attribute {
                            car_body.replace(*rigid_body);
                        }
                    }

                    if let Some(ball) = self.ball_actor_id && actor.actor_id == ball {
                        if let Attribute::RigidBody(rb) = &actor.attribute {
                            self.ball = Some(*rb);
                        }
                    }
                }
                _ => {}
            }

            if let Attribute::DemolishFx(demo) = &actor.attribute {
                let victim = demo.victim;
                self.car_actors.remove(&victim);
            }
            if let Attribute::Demolish(demo) = &actor.attribute {
                let victim = demo.victim;
                self.car_actors.remove(&victim);
            }
        }

        for actor in &frame.deleted_actors {
            // Handle if a player was removed from a team
            if let Some(player) = self.player_actors.remove(actor) {
                if let Some(car) = player.car_actor_id {
                    self.car_actors.remove(&car);
                }
            }

            // Handle if a car was removed for another reason not already handled
            self.car_actors.remove(actor);
        }

        self.frame_index += 1;
    }
}

fn run(args: &Args, replay: Replay) -> Result<(), Box<dyn error::Error>> {
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

    let mut viz = ReplayVis::new(args, GlGraphics::new(opengl), replay);

    let mut ups = args.ups.unwrap_or(120);
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
                Button::Keyboard(Key::Left) => viz.move_frame(-150),
                Button::Keyboard(Key::Right) => viz.move_frame(150),
                Button::Keyboard(Key::Up) => {
                    ups = ups.wrapping_add(10);
                    events.set_ups(ups);
                }
                Button::Keyboard(Key::Down) => {
                    ups = ups.wrapping_sub(10);
                    events.set_ups(ups);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn dump(replay: Replay) -> Result<(), Box<dyn error::Error>> {
    let mut actors: HashMap<ActorId, NewActorResolved> = Default::default();

    let mut f = fs::File::create("./frames.txt")?;
    for frame in replay.network_frames.unwrap().frames {
        f.write_all("=====================\n".as_bytes())?;
        f.write_all(format!("Time: {:?}\n", frame.time).as_bytes())?;
        f.write_all(format!("Delt: {:?}\n", frame.delta).as_bytes())?;
        f.write_all("--------\n".as_bytes())?;
        f.write_all("New Actors\n".as_bytes())?;
        f.write_all("---\n".as_bytes())?;
        for actor in &frame.new_actors {
            let actor = NewActorResolved {
                actor_id: actor.actor_id,
                name: if let Some(name_id) = actor.name_id {
                    replay.names[name_id as usize].clone()
                } else {
                    "Unknown".to_string()
                },
                object: replay.objects[actor.object_id.0 as usize].clone(),
                trajectory: actor.initial_trajectory,
            };
            actors.insert(actor.actor_id, actor.clone());

            f.write_all(format!("Actor: {:?}\n", actor).as_bytes())?;
        }
        f.write_all("--------\n".as_bytes())?;
        f.write_all("Updated Actors\n".as_bytes())?;
        f.write_all("---\n".as_bytes())?;
        for actor in &frame.updated_actors {
            let actor = UpdatedActorResolved {
                actor_id: actor.actor_id,
                actor: actors.get(&actor.actor_id).unwrap().name.clone(),
                object: replay.objects[actor.object_id.0 as usize].clone(),
                attribute: actor.attribute.clone(),
                stream_id: actor.stream_id,
            };

            f.write_all(format!("Actor: {:?}\n", actor).as_bytes())?;
        }
        f.write_all("--------\n".as_bytes())?;
        f.write_all("Deleted Actors\n".as_bytes())?;
        f.write_all("---\n".as_bytes())?;
        for actor in &frame.deleted_actors {
            f.write_all(format!("Actor: {:?}\n", actor).as_bytes())?;
        }
        f.write_all("--------\n".as_bytes())?;
        f.write_all("=====================\n".as_bytes())?;
    }
    Ok(())
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

    run(&args, replay)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct NewActorResolved {
    actor_id: ActorId,
    name: String,
    object: String,
    trajectory: boxcars::Trajectory,
}

#[derive(Debug, Clone)]
struct UpdatedActorResolved {
    actor_id: ActorId,
    actor: String,
    attribute: Attribute,
    object: String,
    stream_id: boxcars::StreamId,
}
