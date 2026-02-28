use clap::Parser;
use std::{
    fs::{self, create_dir_all},
    io,
    path::Path,
    vec,
};

#[derive(Parser)]
#[command(name = "cpr", about = "A better copy tool")]
struct Args {
    /// Source path
    source: String,
    /// Destination path
    destination: String,
    /// Comma-seperated patterns to exclude
    #[arg(short, long)]
    exclude: Option<String>,
}

fn main() {
    // let args: Vec<String> = env::args().collect();
    let args = Args::parse();
    let source_path = Path::new(&args.source);
    let dest_path = Path::new(&args.destination);

    let excludes: Vec<&str> = match &args.exclude {
        Some(e) => e.split(',').collect(),
        None => vec![],
    };
    // let Some(source) = args.get(1) else {
    //     println!("Usage: cpr <source> <destination>");
    //     return;
    // };
    // let Some(destination) = args.get(2) else {
    //     println!("Usage: cpr <source> <destination>");
    //     return;
    // };
    //
    // let dest_path = Path::new(destination);
    // let source_path = Path::new(source);

    if source_path.is_dir() {
        println!("Copy directory '{}' and all contents? (y/n)", args.source);
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim() == "y" {
            match copy_dir(source_path, dest_path, &excludes) {
                Ok(bytes) => println!("Total Bytes Copied = {}", bytes),
                Err(e) => println!("Error: {}", e),
            }
        } else {
            return;
        }
    } else if source_path.is_file() {
        let final_dest = if dest_path.is_dir() {
            dest_path.join(source_path.file_name().unwrap())
        } else {
            dest_path.to_path_buf()
        };

        match std::fs::copy(source_path, final_dest) {
            Ok(bytes) => println!("Copied {} bytes", bytes),
            Err(e) => println!("Error: {}", e),
        }
    } else {
        println!("Source not found: {}", args.source);
    }

    fn copy_dir(
        source: &Path,
        destination: &Path,
        exclude: &[&str],
    ) -> Result<u64, std::io::Error> {
        create_dir_all(destination)?;
        let mut total_bytes: u64 = 0;
        let mut stack = vec![source.to_path_buf()];

        while let Some(current_path) = stack.pop() {
            for entry in fs::read_dir(&current_path)? {
                let entry = entry?;
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy();
                if exclude.iter().any(|c| matches_pattern(*c, &name)) {
                    continue;
                }
                let filetype = entry.file_type()?;
                let entry_path = entry.path();
                let relative = entry_path.strip_prefix(source).unwrap();
                let new_dest = destination.join(relative);
                if filetype.is_dir() {
                    create_dir_all(&new_dest)?;
                    stack.push(entry_path);
                } else {
                    match std::fs::copy(entry.path(), &new_dest) {
                        Ok(bytes) => total_bytes += bytes,
                        Err(e) => println!("Error: {}", e),
                    }
                }
            }
        }
        Ok(total_bytes)
    }
    fn matches_pattern(pattern: &str, name: &str) -> bool {
        if pattern.starts_with("*.") {
            name.ends_with(&pattern[1..])
        } else {
            name == pattern
        }
    }
}
