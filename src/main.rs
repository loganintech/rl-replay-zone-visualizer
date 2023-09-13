use std::collections::HashMap;
use std::error;
use std::fs;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use boxcars::Attribute;
use boxcars::NewActor;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to replay file to visualize.
    #[arg(short, long)]
    replay: PathBuf,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Args::parse();
    let mut f = BufReader::new(fs::File::open(&args.replay)?);

    let mut replay_data = vec![];
    let _read_bytes = f.read_to_end(&mut replay_data)?;
    let replay = boxcars::ParserBuilder::new(&replay_data)
        .must_parse_network_data()
        .parse()?;

    // let json_data = serde_json::to_string_pretty(&replay)?;
    // let mut outf = fs::File::create("output.json")?;
    // outf.write_all(json_data.as_bytes())?;

    println!("Entities: {:?}", replay.class_indices);
    let objects_by_class_index = replay.class_indices.iter().fold(HashMap::new(), |mut map, index| {
        map.insert(index.index, index.class.as_str());
        map
    });

    println!("Player: {:?}", replay.objects[147]);
    let mut player_actors: HashMap<i32, String> = HashMap::new();
    let mut all_actors: HashMap<i32, boxcars::UpdatedAttribute> = HashMap::new();

    // Unwrap is safe because of parser's `.must_parse_network_data`
    let mut i = 0;
    for frame in replay.network_frames.unwrap().frames {
        // for actor in frame.new_actors {
        //     println!("Actor: {:?}", actor);
        //     println!("Object: {:?}", replay.objects[actor.object_id.0 as usize]);
        //     if let Some(name_id) = actor.name_id {
        //         println!("Name: {:?}", replay.names[name_id as usize]);
        //     }
        // }
        for actor in frame.updated_actors {
            if actor.object_id.0 == 168 {
                if let Attribute::String(player_name) = &actor.attribute {
                    println!("Adding Actor to Map: {:?}", actor);
                    player_actors.insert(actor.actor_id.0,player_name.clone());
                }
            }

            if let Attribute::ActiveActor(active) = &actor.attribute {
                if !all_actors.contains_key(&actor.actor_id.0) {
                    all_actors.insert(actor.actor_id.0, actor.clone());
                }
            }


            // if player_actors.get(&actor.actor_id.0).is_some() {
            //     println!("Actor: {:?}", actor);
            // }

            if let Attribute::RigidBody(body) = &actor.attribute {
                // match player_actors.get(&actor.actor_id.0) {
                //     Some(name) => println!("Player Moved: {:?}", body),
                //     None => println!("Unknown Actor Moved: {:?}", actor),
                // }
                match all_actors.get(&actor.actor_id.0) {
                    Some(&boxcars::UpdatedAttribute { attribute: Attribute::ActiveActor(actor_id), .. }) => {
                        if let Some(player) = player_actors.get(&actor_id.actor.0) {
                            println!("{} Moved: {:?}", player, actor.attribute)
                        }
                    },
                    _ => {},
                    // _ => println!("Unknown Actor Moved: {:?}", actor),
                }


            }
            // println!("Actor: {:?}", actor);
            // println!("Object: {:?}", replay.objects[actor.object_id.0 as usize]);
        }
    }

    Ok(())
}
