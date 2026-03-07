use clap::Parser;
use std::{
    fs::{self, create_dir_all},
    io,
    path::Path,
};

#[derive(Parser)]
#[command(
    name = "cpr",
    about = "A file and directory copy tool with --exclude support",
    after_help = "Examples:\n  cpr report.pdf D:\\backup\\\n  cpr C:\\project\\ D:\\backup\\project\\ -e node_modules,.git,*.log -y"
)]
struct Args {
    /// Source file or directory
    source: String,
    /// Destination path
    destination: String,
    /// Comma-separated patterns to exclude
    #[arg(short, long)]
    exclude: Option<String>,
    /// Skip confirmation prompt for directory copies
    #[arg(short, long)]
    yes: bool,
}

fn main() {
    let args = Args::parse();
    let source_path = Path::new(&args.source);
    let dest_path = Path::new(&args.destination);

    let excludes: Vec<&str> = match &args.exclude {
        Some(e) => e.split(',').collect(),
        None => vec![],
    };
    let yes = args.yes;

    if source_path.is_dir() {
        let mut input = String::new();
        if !yes {
            println!("Copy directory '{}' and all contents? (y/n)", args.source);
            io::stdin().read_line(&mut input).unwrap();
        }

        if input.trim() == "y" || yes {
            match copy_dir(source_path, dest_path, &excludes) {
                Ok(copy_result) => {
                    println!("Total Bytes Copied = {}", copy_result.bytes_copied);
                    println!("Total Files Copied = {}", copy_result.files_copied);
                    println!("Total Files Excluded = {}", copy_result.files_excluded);
                    if !copy_result.errors.is_empty() {
                        for err in copy_result.errors {
                            println!("Error: {}", err)
                        }
                    }
                }
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
}

struct CopyResult {
    bytes_copied: u64,
    files_copied: u64,
    files_excluded: u64,
    errors: Vec<String>,
}

fn copy_dir(
    source: &Path,
    destination: &Path,
    exclude: &[&str],
) -> Result<CopyResult, std::io::Error> {
    create_dir_all(destination)?;
    let mut result = CopyResult {
        bytes_copied: 0,
        files_copied: 0,
        files_excluded: 0,
        errors: Vec::new(),
    };
    let mut stack = vec![source.to_path_buf()];

    while let Some(current_path) = stack.pop() {
        for entry in fs::read_dir(&current_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if exclude.iter().any(|c| matches_pattern(*c, &name)) {
                result.files_excluded += 1;
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
                    Ok(bytes) => {
                        result.bytes_copied += bytes;
                        result.files_copied += 1;
                    }
                    Err(e) => result
                        .errors
                        .push(format!("{}: {}", entry.path().display(), e)),
                }
            }
        }
    }
    Ok(result)
}
fn matches_pattern(pattern: &str, name: &str) -> bool {
    if pattern.starts_with("*.") {
        name.ends_with(&pattern[1..])
    } else {
        name == pattern
    }
}
