use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod args;
mod device;
mod engine;
mod input;
mod mappings;
mod profile;
mod uinput;
mod wrapper;

use args::parse_args;
use engine::Engine;
use input::{InputWatcher, run_event_loop};
use mappings::build_mappings;
use profile::{load_config, print_rebinds};

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;

    println!("Running steam-wrapper...");
    let config = load_config(&args.profile_path.to_string_lossy())?;

    if config.output.has_rebinds() { println!("Resolving rebinds..."); }
    let output = config.output.resolve(&config.input)?;
    let physical_inputs: std::collections::HashMap<String, evdev::KeyCode> = config.input.iter()
        .filter_map(|(name, def)| def.physical_key().map(|k| (name.clone(), k)))
        .collect();

    let mappings = build_mappings(config.mappings, &output.bindings)?;
    let all_keys = output.bindings.values().copied()
        .chain(physical_inputs.values().copied());
    let sink = uinput::UInputSink::new(all_keys)?;
    let engine = Arc::new(Mutex::new(Engine::new(
        output.bindings.clone(),
        physical_inputs,
        mappings,
        Box::new(sink),
    )));

    let mut device = config.device.open()?;
    if args.wrap_command.is_none() { device.grab()?; }
    let stream = device.into_event_stream()?;
    let mut watchers: Vec<Box<dyn InputWatcher>> = Vec::new();

    const AXIS_TIMEOUT_MS: u64 = 50;

    for (name, def) in config.input {
        let eng_down = engine.clone();
        let eng_up = engine.clone();
        let eng_axis = engine.clone();
        let name_down = name.clone();
        let name_up = name.clone();
        let name_axis = name.clone();
        watchers.push(def.into_watcher(
            name,
            move || eng_down.lock().unwrap().on_button_down(&name_down),
            move || eng_up.lock().unwrap().on_button_up(&name_up),
            move |axis, d| {
                let mut eng = eng_axis.lock().unwrap();
                if eng.on_axis_move(&name_axis) {
                    drop(eng);
                    let eng2 = eng_axis.clone();
                    let n2 = name_axis.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(AXIS_TIMEOUT_MS)).await;
                        eng2.lock().unwrap().on_axis_release_maybe(&n2, AXIS_TIMEOUT_MS);
                    });
                } else {
                    eng.relay_axis(axis, d);
                }
            },
            args.print_noisy,
        )?);
    }

    print_rebinds(&output);

    if let Some(command) = args.wrap_command {
        wrapper::run_wrapped(command, engine, stream, &mut watchers).await?;
    } else {
        println!("Listening for input events. Ctrl+C to exit.");
        run_event_loop(stream, &mut watchers).await?;
    }

    Ok(())
}
