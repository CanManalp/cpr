use clap::Parser;
use colored::*;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::{
    fs::{self, create_dir_all},
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    sync::Mutex,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Parser)]
#[command(
    about = "A fast file and directory copy tool with glob pattern filtering",
    after_help = "Examples:\n  cpr report.pdf D:\\backup\\\n  cpr src dst -e .git,node_modules,target -y\n  cpr src dst -e *.log,*.tmp -n\n  cpr src dst -i *.rs                        # only .rs files\n  cpr src dst -i src/**                      # only the src folder\n  cpr src dst -i *.rs,*.toml -e tests/**     # .rs and .toml files, skip tests\n\nGlob patterns:\n  *.log         matches files by extension\n  *.rs,*.toml   matches multiple patterns\n  **/*.test.js  matches in any subdirectory\n  src/**        matches everything inside src"
)]
struct Args {
    /// Source file or directory
    source: PathBuf,
    /// Destination path
    destination: PathBuf,
    /// Patterns to exclude (supports globs like *.log, **/*.tmp)
    #[arg(short, long, value_delimiter = ',')]
    exclude: Vec<String>,
    /// Patterns to include — only matching files are copied (same glob syntax as exclude)
    #[arg(short, long, value_delimiter = ',')]
    include: Vec<String>,
    /// Skip confirmation prompt for directory copies
    #[arg(short, long)]
    yes: bool,
    /// Shows what would be copied without actually copying.
    #[arg(short = 'n', long)]
    dry_run: bool,
}

fn main() {
    let args = Args::parse();
    let yes = args.yes;
    let dry = args.dry_run;
    let exclude_set = match build_glob_set(&args.exclude) {
        Ok(set) => set,
        Err(e) => {
            println!("Invalid exclude pattern: {}", e.to_string().red());
            return;
        }
    };
    let include_set = match build_glob_set(&args.include) {
        Ok(set) => set,
        Err(e) => {
            println!("Invalid include pattern: {}", e.to_string().red());
            return;
        }
    };
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

        let show_progress = !dry && io::stderr().is_terminal();
        match copy_dir(
            &args.source,
            &args.destination,
            &exclude_set,
            &include_set,
            dry,
            show_progress,
        ) {
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
                    let elapsed = copy_result.elapsed.unwrap();
                    let secs = elapsed.as_secs_f64();
                    let throughput = if secs > 0.0 {
                        (copy_result.bytes_copied as f64 / secs) as u64
                    } else {
                        copy_result.bytes_copied
                    };
                    println!(
                        "{} {} {} {} {}",
                        "Copied".green(),
                        HumanBytes(copy_result.bytes_copied).to_string().green(),
                        format!("in {}", HumanDuration(elapsed)).green(),
                        format!("({}/s)", HumanBytes(throughput)).green(),
                        format!("[{} files]", copy_result.files_copied).green(),
                    );
                    if copy_result.files_excluded > 0 {
                        println!(
                            "{} {}",
                            "Files Excluded =".yellow(),
                            copy_result.files_excluded.to_string().yellow()
                        );
                    }
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
    elapsed: Option<std::time::Duration>,
    errors: Vec<String>,
}

fn copy_dir(
    src_root: &Path,
    dest_root: &Path,
    exclude: &GlobSet,
    include: &GlobSet,
    dry_run: bool,
    show_progress: bool,
) -> Result<CopyResult, std::io::Error> {
    if !dry_run {
        create_dir_all(dest_root)?;
    }
    let mut result = CopyResult {
        bytes_copied: 0,
        files_copied: 0,
        files_excluded: 0,
        files_to_be_copied: Vec::new(),
        files_to_be_excluded: Vec::new(),
        elapsed: None,
        errors: Vec::new(),
    };

    // Phase 1: Walk — collect files and dirs (sequential)
    let mut files_to_copy: Vec<(PathBuf, PathBuf)> = Vec::new(); // (source, dest)
    let mut stack = vec![src_root.to_path_buf()];
    let mut total_bytes: u64 = 0;

    while let Some(current_path) = stack.pop() {
        for dir_entry in fs::read_dir(&current_path)? {
            let dir_entry = dir_entry?;
            let src_path = dir_entry.path();
            let relative_path = src_path.strip_prefix(src_root).unwrap();
            let rel_str = relative_path.to_string_lossy();

            // Exclude applies to both files and directories
            if !exclude.is_empty() && exclude.is_match(relative_path) {
                if dry_run {
                    result.files_to_be_excluded.push(rel_str.to_string());
                } else {
                    result.files_excluded += 1;
                }
                continue;
            }

            let filetype = dir_entry.file_type()?;
            let dest_path = dest_root.join(relative_path);

            if filetype.is_dir() {
                // Always walk into non-excluded directories
                if !dry_run {
                    create_dir_all(&dest_path)?;
                }
                stack.push(src_path);
            } else if !include.is_empty() && !include.is_match(relative_path) {
                // Include only filters files — unmatched files are excluded
                if dry_run {
                    result.files_to_be_excluded.push(rel_str.to_string());
                } else {
                    result.files_excluded += 1;
                }
            } else if dry_run {
                result.files_to_be_copied.push(rel_str.to_string());
            } else {
                total_bytes += dir_entry.metadata()?.len();
                files_to_copy.push((src_path, dest_path));
            }
        }
    }

    // Phase 2: Copy — parallel file copies
    if !dry_run {
        let pb = if show_progress {
            let pb = ProgressBar::new(total_bytes);
            pb.set_style(
                ProgressStyle::with_template(
                    "{bar:40.green} {bytes}/{total_bytes} ({bytes_per_sec}) [{elapsed}]",
                )
                .unwrap(),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(100));
            pb
        } else {
            ProgressBar::hidden()
        };

        let bytes_copied = AtomicU64::new(0);
        let files_copied = AtomicU64::new(0);
        let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let start = Instant::now();
        files_to_copy
            .par_iter()
            .for_each(|(src, dest)| match std::fs::copy(src, dest) {
                Ok(bytes) => {
                    bytes_copied.fetch_add(bytes, Ordering::Relaxed);
                    files_copied.fetch_add(1, Ordering::Relaxed);
                    pb.inc(bytes);
                }

                Err(e) => errors.lock().unwrap().push(e.to_string()),
            });
        pb.finish_and_clear();
        result.elapsed = Some(start.elapsed());
        result.bytes_copied = bytes_copied.load(Ordering::Relaxed);
        result.files_copied = files_copied.load(Ordering::Relaxed);
        result.errors = errors.into_inner().unwrap();
    }
    Ok(result)
}
fn build_glob_set(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder: GlobSetBuilder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    builder.build()
}
