use anyhow::Result;
use std::env;
use std::path::PathBuf;

pub struct CliArgs {
    pub profile_path: PathBuf,
    pub print_noisy: bool,
}

pub fn parse_args() -> Result<CliArgs> {
    let mut profile_name: Option<String> = None;
    let mut print_noisy = false;

    for arg in env::args().skip(1) {
        if arg == "--print-noisy" {
            print_noisy = true;
        } else if arg.starts_with('-') {
            return Err(anyhow::anyhow!("Unknown flag: {}", arg));
        } else if profile_name.is_none() {
            profile_name = Some(arg);
        }
    }

    let profile_name = profile_name
        .ok_or_else(|| anyhow::anyhow!("Usage: steam-wrapper <profile> [--print-noisy]"))?;

    let profile_file = if profile_name.ends_with(".json5") {
        profile_name
    } else {
        format!("{}.json5", profile_name)
    };

    let binary_path = env::current_exe()?;
    let binary_dir = binary_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get binary directory"))?
        .to_path_buf();
    let full_path = binary_dir.join(&profile_file);

    if !full_path.exists() {
        return Err(anyhow::anyhow!("{} not found", profile_file));
    }

    Ok(CliArgs { profile_path: full_path, print_noisy })
}
