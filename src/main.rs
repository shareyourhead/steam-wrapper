use anyhow::Result;

mod args;
mod device;
mod input;
mod mappings;
mod profile;

use args::parse_args;
use input::{InputWatcher, run_event_loop};
use mappings::build_mappings;
use profile::{load_config, print_rebinds};

#[tokio::main]
async fn main() -> Result<()> {

    let args = parse_args()?;
    
    println!("Running steam-wrapper...");
    let config = load_config(&args.profile_path.to_string_lossy())?;

    if config.output.has_rebinds() { println!("Resolving rebinds..."); }
    let output = config.output.resolve()?;
    let _mappings = build_mappings(config.mappings, &output.bindings)?;
    let device = config.device.open()?;
    let stream = device.into_event_stream()?;
    let mut watchers: Vec<Box<dyn InputWatcher>> = Vec::new();

    for (name, def) in config.input {
        watchers.push(def.into_watcher(name, args.print_noisy)?);
    }

    print_rebinds(&output);

    println!("Listening for input events. Ctrl+C to exit.");
    run_event_loop(stream, &mut watchers).await?;

    Ok(())
}
