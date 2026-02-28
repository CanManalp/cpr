# cpr

Fast file/directory copy CLI with `--exclude` support. Built in Rust with `clap` for argument parsing.

## Project Structure

- `src/main.rs` — single-file CLI, iterative (stack-based) directory traversal
- `Cargo.toml` — only dependency is `clap` with derive feature

## Distribution

Open source on GitHub: https://github.com/CanManalp/cpr

Installable via Scoop (Windows) using a custom bucket: https://github.com/CanManalp/scoop-cpr

### Release Checklist

When publishing a new version:

1. Bump `version` in `Cargo.toml`
2. `cargo build --release`
3. `gh release create vX.Y.Z target/release/cpr.exe --title "vX.Y.Z"`
4. Get hash: `sha256sum target/release/cpr.exe`
5. Update `scoop-cpr/cpr.json` — set new `version` and `hash`
6. Commit and push `scoop-cpr`

## Build & Run

```bash
cargo build --release          # release binary
cargo run -- src dst -e .git   # quick test
```

## Roadmap

Improvements to make one by one (in priority order):

### 1. Code cleanup
- Remove commented-out code (lines 31-41 in main.rs)
- Remove unused `use std::vec` import
- Move `copy_dir` and `matches_pattern` out of `main()` to top-level functions

### 2. Fix silent error swallowing
- `copy_dir` prints individual file errors but returns `Ok` with partial byte count
- Caller thinks the operation succeeded — should track and report failures properly

### 3. Add `-y` / `--yes` flag
- Skip the "y/n" confirmation prompt for directory copies
- Essential for scripting and automation

### 4. Add `--dry-run`
- Preview what would be copied/excluded without actually copying
- Helps users verify their exclude patterns are correct

### 5. Better output
- Show file count (copied / skipped / errored), not just total bytes
- Optionally list files as they're copied

### 6. Parallel copying with `rayon`
- Copy multiple files concurrently for better throughput on many small files
- Big win for directories with thousands of small files on SSDs
- Consider detecting HDD vs SSD — parallel can hurt on spinning disks due to seek times

### 7. Zero-copy / OS-level optimizations
- Explore Windows-specific APIs beyond `CopyFileExW` (e.g., `CopyFile2` with progress callbacks)
- Consider memory-mapped I/O for large files to reduce syscall overhead
- Use Rust's zero-cost abstractions: iterators, no GC pauses, no runtime overhead

### 8. Benchmark against `robocopy`
- `robocopy /MT` (multi-threaded, built into Windows) is the real competitor, not Explorer
- Create a benchmark script: generate test directories (many small files, few large files, mixed)
- Compare `cpr` vs `robocopy` vs `Copy-Item` and publish results in README
- Only add "fast" back to README if benchmarks prove it
