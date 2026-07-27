use clap::Parser;
use colored::*;
use globset::{Glob, GlobSet, GlobSetBuilder};
use indicatif::{HumanBytes, HumanDuration, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::{
    ffi::OsStr,
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
    after_help = "Examples:\n  cpr report.pdf D:\\backup\\\n  cpr src dst -e .git,node_modules,target -y\n  cpr src dst -e *.log,*.tmp -n\n  cpr src dst -i *.rs                        # only .rs files\n  cpr src dst -i src/**                      # only the src folder\n  cpr src dst -i *.rs,*.toml -e tests/**     # .rs and .toml files, skip tests\n\nGlob patterns:\n  bin           bare name: matches files/dirs named 'bin' at ANY depth\n  *.log         matches files by extension at any depth\n  *.rs,*.toml   matches multiple patterns\n  **/*.test.js  matches in any subdirectory\n  src/**        path pattern (contains '/'): matches relative to the source root"
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
    let exclude_filters = match build_filters(&args.exclude) {
        Ok(filters) => filters,
        Err(e) => {
            println!("Invalid exclude pattern: {}", e.to_string().red());
            return;
        }
    };
    let include_filters = match build_filters(&args.include) {
        Ok(filters) => filters,
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
            &exclude_filters,
            &include_filters,
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
                    if copy_result.files_excluded > 0 || copy_result.dirs_excluded > 0 {
                        println!(
                            "{} {}",
                            "Excluded =".yellow(),
                            format!(
                                "{} files, {} dirs",
                                copy_result.files_excluded, copy_result.dirs_excluded
                            )
                            .yellow()
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
    dirs_excluded: u64,
    files_to_be_copied: Vec<String>,
    files_to_be_excluded: Vec<String>,
    elapsed: Option<std::time::Duration>,
    errors: Vec<String>,
}

fn copy_dir(
    src_root: &Path,
    dest_root: &Path,
    exclude: &Filters,
    include: &Filters,
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
        dirs_excluded: 0,
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
            let filetype = dir_entry.file_type()?;

            // Exclude applies to both files and directories
            if !exclude.is_empty() && is_excluded(&dir_entry.file_name(), relative_path, exclude) {
                if dry_run {
                    result.files_to_be_excluded.push(rel_str.to_string());
                } else if filetype.is_dir() {
                    result.dirs_excluded += 1;
                } else {
                    result.files_excluded += 1;
                }
                continue;
            }

            let dest_path = dest_root.join(relative_path);

            if filetype.is_dir() {
                // Always walk into non-excluded directories
                if !dry_run {
                    create_dir_all(&dest_path)?;
                }
                stack.push(src_path);
            } else if !include.is_empty() && !is_included(relative_path, include) {
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
/// Gitignore-style pattern split:
/// - path patterns (contain a separator, or `**`) match against the relative path
/// - name patterns (bare names like `bin`, `*.log`) match against an entry's own
///   file name, so they apply at any depth
struct Filters {
    path_set: GlobSet,
    name_set: GlobSet,
}

impl Filters {
    fn is_empty(&self) -> bool {
        self.path_set.is_empty() && self.name_set.is_empty()
    }
}

fn build_filters(patterns: &[String]) -> Result<Filters, globset::Error> {
    let mut path_builder = GlobSetBuilder::new();
    let mut name_builder = GlobSetBuilder::new();
    for pattern in patterns {
        // globset matches candidates with `/` separators even on Windows,
        // so a pattern typed as `proj\bin` must be normalized to `proj/bin`
        let pattern = pattern.replace('\\', "/");
        if pattern.contains('/') || pattern.contains("**") {
            path_builder.add(Glob::new(&pattern)?);
        } else {
            name_builder.add(Glob::new(&pattern)?);
        }
    }
    Ok(Filters {
        path_set: path_builder.build()?,
        name_set: name_builder.build()?,
    })
}

fn is_excluded(file_name: &OsStr, rel: &Path, filters: &Filters) -> bool {
    filters.path_set.is_match(rel) || filters.name_set.is_match(file_name)
}

/// Include is only tested for files (directories are always walked).
/// A file is included when the path_set matches its relative path, or when the
/// name_set matches its own file name OR any directory component of `rel` —
/// that component rule is what makes `-i src` mean "everything under any
/// folder named src" instead of the useless "files literally named src".
fn is_included(rel: &Path, filters: &Filters) -> bool {
    filters.path_set.is_match(rel)
        || rel
            .components()
            .any(|c| filters.name_set.is_match(c.as_os_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filters(patterns: &[&str]) -> Filters {
        let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        build_filters(&owned).unwrap()
    }

    fn excluded(f: &Filters, rel: &str) -> bool {
        let rel = Path::new(rel);
        is_excluded(rel.file_name().unwrap(), rel, f)
    }

    #[test]
    fn bare_name_excludes_at_any_depth() {
        let f = filters(&["bin"]);
        assert!(excluded(&f, "bin"));
        assert!(excluded(&f, "proj/bin")); // the original bug
    }

    #[test]
    fn bare_name_is_exact_not_substring() {
        let f = filters(&["bin"]);
        assert!(!excluded(&f, "binary"));
        assert!(!excluded(&f, "cabin.txt"));
    }

    #[test]
    fn extension_pattern_still_matches_nested_files() {
        let f = filters(&["*.user"]);
        assert!(excluded(&f, "proj/app.csproj.user"));
    }

    #[test]
    fn path_pattern_behavior_unchanged() {
        let f = filters(&["src/**"]);
        assert!(excluded(&f, "src/x"));
        assert!(!excluded(&f, "other/src/x"));
    }

    #[test]
    fn backslash_pattern_treated_as_path_pattern() {
        let f = filters(&[r"proj\bin"]);
        assert!(excluded(&f, "proj/bin"));
        assert!(!excluded(&f, "other/proj/bin"));
    }

    #[test]
    fn double_star_alone_is_path_pattern() {
        let f = filters(&["**"]);
        assert!(excluded(&f, "anything/at/all"));
    }

    #[test]
    fn include_name_matches_any_dir_component() {
        let f = filters(&["src"]);
        assert!(is_included(Path::new("src/main.rs"), &f));
        assert!(is_included(Path::new("deep/src/lib.rs"), &f));
        assert!(!is_included(Path::new("srcs/file.rs"), &f));
    }

    #[test]
    fn include_extension_matches_at_depth() {
        let f = filters(&["*.rs"]);
        assert!(is_included(Path::new("a/b/c.rs"), &f));
        assert!(!is_included(Path::new("a/b/c.txt"), &f));
    }

    #[test]
    fn include_path_pattern_matches_relative_path() {
        let f = filters(&["src/**"]);
        assert!(is_included(Path::new("src/main.rs"), &f));
        assert!(!is_included(Path::new("other/main.rs"), &f));
    }
}
