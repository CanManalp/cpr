use clap::Parser;
use colored::*;
use rayon::prelude::*;
use std::{
    fs::{self, create_dir_all},
    io,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    about = "A file and directory copy tool with --exclude support",
    after_help = "Examples:\n  cpr report.pdf D:\\backup\\\n  cpr C:\\project\\ D:\\backup\\project\\ -e node_modules,.git,*.log -y\n  cpr C:\\project\\ D:\\backup\\project\\ -e node_modules -n"
)]
struct Args {
    /// Source file or directory
    source: PathBuf,
    /// Destination path
    destination: PathBuf,
    /// Comma-separated patterns to exclude
    #[arg(short, long)]
    exclude: Option<String>,
    /// Skip confirmation prompt for directory copies
    #[arg(short, long)]
    yes: bool,
    /// Shows what would be copied without actually copying.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() {
    let args = Args::parse();

    let excludes: Vec<&str> = match &args.exclude {
        Some(e) => e.split(',').collect(),
        None => vec![],
    };
    let yes = args.yes;
    let dry = args.dry_run;

    if args.source.is_dir() {
        if !yes && !dry {
            println!(
                "Copy directory '{}' and all contents? (y/n)",
                args.source.display()
            );
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            if input.trim() != "y" {
                return;
            }
        }

        match copy_dir(&args.source, &args.destination, &excludes, dry) {
            Ok(copy_result) => {
                if dry {
                    if !copy_result.files_to_be_copied.is_empty() {
                        println!("{}", "Files To Be Copied :".green());
                        println!("----------------------------");
                        for file in copy_result.files_to_be_copied {
                            println!("{}", file)
                        }
                    }
                    if !copy_result.files_to_be_excluded.is_empty() {
                        println!("{}", "Files To Be Excluded :".yellow());
                        println!("----------------------------");
                        for exc in copy_result.files_to_be_excluded {
                            println!("{}", exc)
                        }
                    }
                } else {
                    println!(
                        "{} {}",
                        "Total Bytes Copied =".green(),
                        copy_result.bytes_copied.to_string().green()
                    );
                    println!(
                        "{} {}",
                        "Total Files Copied =".green(),
                        copy_result.files_copied.to_string().green()
                    );
                    println!(
                        "{} {}",
                        "Total Files Excluded =".yellow(),
                        copy_result.files_excluded.to_string().yellow()
                    );
                }
                if !copy_result.errors.is_empty() {
                    for err in copy_result.errors {
                        println!("Error: {}", err.red())
                    }
                }
            }
            Err(e) => println!("Error: {}", e.to_string().red()),
        }
    } else if args.source.is_file() {
        let final_dest = if args.destination.is_dir() {
            args.destination.join(args.source.file_name().unwrap())
        } else {
            args.destination
        };

        match std::fs::copy(args.source, final_dest) {
            Ok(bytes) => println!("Copied {} bytes", bytes),
            Err(e) => println!("Error: {}", e.to_string().red()),
        }
    } else {
        println!("Source not found: {}", args.source.display());
    }
}

struct CopyResult {
    bytes_copied: u64,
    files_copied: u64,
    files_excluded: u64,
    files_to_be_copied: Vec<String>,
    files_to_be_excluded: Vec<String>,
    errors: Vec<String>,
}

fn copy_dir(
    source: &Path,
    destination: &Path,
    exclude: &[&str],
    dry_run: bool,
) -> Result<CopyResult, std::io::Error> {
    if !dry_run {
        create_dir_all(destination)?;
    }
    let mut result = CopyResult {
        bytes_copied: 0,
        files_copied: 0,
        files_excluded: 0,
        files_to_be_copied: Vec::new(),
        files_to_be_excluded: Vec::new(),
        errors: Vec::new(),
    };

    // Phase 1: Walk — collect files and dirs (sequential)
    let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new(); // (source, dest)
    let mut stack = vec![source.to_path_buf()];

    while let Some(current_path) = stack.pop() {
        for entry in fs::read_dir(&current_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            let entry_path = entry.path();
            let relative = entry_path.strip_prefix(source).unwrap();
            if exclude.iter().any(|c| matches_pattern(c, &name)) {
                if dry_run {
                    result
                        .files_to_be_excluded
                        .push(relative.to_string_lossy().to_string());
                } else {
                    result.files_excluded += 1;
                }
                continue;
            }
            let filetype = entry.file_type()?;
            let new_dest = destination.join(relative);
            if filetype.is_dir() {
                if !dry_run {
                    create_dir_all(&new_dest)?;
                }
                stack.push(entry_path);
            } else if dry_run {
                result
                    .files_to_be_copied
                    .push(relative.to_string_lossy().to_string());
            } else {
                files_to_copy.push((entry_path, new_dest));
            }
        }
    }

    // Phase 2: Copy — parallel file copies
    if !dry_run {
        let bytes_copied = AtomicU64::new(0);
        let files_copied = AtomicU64::new(0);
        let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
        files_to_copy
            .par_iter()
            .for_each(|(src, dest)| match std::fs::copy(src, dest) {
                Ok(bytes) => {
                    bytes_copied.fetch_add(bytes, Ordering::Relaxed);
                    files_copied.fetch_add(1, Ordering::Relaxed);
                }

                Err(e) => errors.lock().unwrap().push(e.to_string()),
            });
        result.bytes_copied = bytes_copied.load(Ordering::Relaxed);
        result.files_copied = files_copied.load(Ordering::Relaxed);
        result.errors = errors.into_inner().unwrap();
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
