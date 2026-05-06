use std::fs;
use std::path::Path;

fn main() {
    let profiles_dir = Path::new("profiles");
    let out_dir = Path::new("target/debug");

    // Tell Cargo to re-run this script if anything in profiles/ changes
    println!("cargo:rerun-if-changed=profiles/");

    if !profiles_dir.exists() {
        eprintln!("cargo:warning=profiles/ directory not found, skipping copy");
        return;
    }

    let entries = fs::read_dir(profiles_dir)
        .expect("Failed to read profiles/ directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json5") {
            let file_name = path.file_name().expect("File has no name");
            let dest = out_dir.join(file_name);

            fs::copy(&path, &dest).unwrap_or_else(|e| {
                panic!("Failed to copy {} to {}: {}", path.display(), dest.display(), e);
            });

            println!(
                "cargo:warning=Copied {} -> {}",
                path.display(),
                dest.display()
            );
        }
    }
}
