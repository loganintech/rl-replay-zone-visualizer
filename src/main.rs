use boxcars::{ParseError, Replay};
use std::error;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use clap::Parser;
use serde_json;

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
    let read_bytes = f.read_to_end(&mut replay_data)?;
    let replay = boxcars::ParserBuilder::new(&replay_data)
        .must_parse_network_data()
        .parse()?;

    // let json_data = serde_json::to_string_pretty(&replay)?;
    // let mut outf = fs::File::create("output.json")?;
    // outf.write_all(json_data.as_bytes())?;

    for frame in replay.network_frames?.frames {
        println!("Frame: {:?}", frame);
        break
    }

    Ok(())
}
