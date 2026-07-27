# cpr

Fast file/directory copy CLI with `--exclude` support. Built in Rust with `clap` for argument parsing.

## Project Structure

- `src/main.rs` — single-file CLI: walk phase (sequential) + copy phase (parallel with rayon)
- `Cargo.toml` — dependencies: `clap` (args), `colored` (output), `rayon` (parallelism)

## Distribution

Open source on GitHub: https://github.com/CanManalp/cpr

Installable via Scoop (Windows) using a custom bucket: https://github.com/CanManalp/scoop-cpr

### Release Checklist

When publishing a new version:

1. Bump `version` in `Cargo.toml`
2. `cargo build --release`
3. `gh release create vX.Y.Z target/release/cpr.exe --title "vX.Y.Z"`
4. Get hash: `sha256sum target/release/cpr.exe`
5. Update `scoop-cpr/cpr.json` — set new `version`, `url`, and `hash`
   - Repo location: `C:\Users\User\AppData\Local\Temp\scoop-cpr\cpr.json`
6. Commit and push `scoop-cpr`

```bash
# Full release flow example (replace X.Y.Z):
cargo build --release
gh release create vX.Y.Z target/release/cpr.exe --title "vX.Y.Z"
sha256sum target/release/cpr.exe
# Copy hash, then update scoop manifest:
cd /c/Users/User/AppData/Local/Temp/scoop-cpr
# Edit cpr.json with new version, url, and hash
git add cpr.json && git commit -m "vX.Y.Z" && git push
```

## Build & Run

```bash
cargo build --release          # release binary
cargo run -- src dst -e .git   # quick test
```

## Roadmap

Improvements to make one by one (in priority order):

### ~~0. BUG — bare-name patterns (`-e bin`) only match at top level~~ (fixed 2026-07-27)
- ~~`-e bin,obj` silently fails to exclude nested `proj/bin`, `proj/obj`~~ Fixed with gitignore-style name-vs-path pattern split for both `-e` and `-i` (patterns without `/` match entry names at any depth). Tests added in `main.rs` (`#[cfg(test)]`). `FIX-bare-name-patterns.md` can be deleted after release.

### ~~1. Code cleanup~~ (done)

### ~~2. Fix silent error swallowing~~ (done)

### ~~3. Add `-y` / `--yes` flag~~ (done)

### ~~4. Add `--dry-run`~~ (done)

### ~~5. Better output~~ (done)

### ~~6. Parallel copying with `rayon`~~ (done)

### ~~7. Human-readable sizes + elapsed time~~ (done)
- ~~Show `1.38 GiB` instead of raw byte counts~~
- ~~Show elapsed time and throughput: `1.38 GiB in 4.2s (336 MB/s)`~~

### 8. Verbose mode (`-v`)
- Print each file path as it's copied
- Essential for debugging exclude patterns and verifying large copies
- Also fix dry-run verbosity: `-n` currently prints the FULL excluded-file list (213 KB of output on a ~2,200-file tree in real use). Default `-n` should print the files to be copied + an excluded **count** (like the real copy does); the full excluded list only with `-n -v`

### ~~9. Progress bar (`indicatif` crate)~~ (done)
- ~~Real-time progress: files copied / total, bytes / total, elapsed time~~
- ~~Auto-disable when stdout is piped (not a terminal)~~
- ~~The #1 most requested feature across all copy tool discussions~~

### 10. `.gitignore`-aware exclude (`--gitignore`)
- Read `.gitignore` files in the source tree and auto-exclude matching paths
- Use the `ignore` crate (same one ripgrep uses)
- Strong developer appeal — no more manually listing `node_modules,.git,target`

### ~~11. Better glob patterns~~ (done)
- ~~Support `**`, `*.{log,tmp}`, path-based patterns (`src/**/*.test.js`)~~
- ~~Consider `--include` patterns in addition to `--exclude`~~
- ~~Use the `globset` crate~~ (globset 0.4.18; `-i` + `**/memory/**` verified in real use 2026-07-19)

### 12. Skip existing / update mode ← highest-value open item
- `--skip-existing` — skip files already present at destination (by name + size)
- `--update` / `-u` — only copy if source is newer (compare size + mtime)
- Makes interrupted copies resumable without starting over
- Killer use case: re-runnable backup refresh — run the SAME full command again and only changed files copy (e.g. keeping a OneDrive backup of `.claude` memory current). Turns cpr from "better Copy-Item" into "the tool for keeping two folders in sync"
- Combo with dry-run: `cpr src dst -u -n` = "what's stale?" preview — a thing robocopy makes painful

### 13. Post-copy checksum verification (`--verify`)
- Hash each file after copying and compare to source
- Use `xxhash` for speed
- People worry about silent corruption on USB/network drives — TeraCopy charges for this

### 14. Sync / mirror mode
- `--sync` — copy new/updated files to destination
- `--mirror` — sync + delete files at destination that no longer exist at source
- Similar to `robocopy /MIR` and `rsync --delete`

### 15. Benchmark against `robocopy`
- `robocopy /MT` (multi-threaded, built into Windows) is the real competitor
- Create a proper benchmark: large datasets, cold cache, multiple machines
- Compare `cpr` vs `robocopy` vs `Copy-Item` and publish results in README
- Only add "fast" to README if benchmarks prove it

### 16. Zero-copy / OS-level optimizations
- Explore `CopyFile2` (Windows 10+) with progress callbacks
- Consider memory-mapped I/O for large files
- Reflink support (copy-on-write) for future cross-platform support

### 17. Machine-readable result (`--json`)
- One JSON object on stdout at the end: `{"copied": 227, "excluded": 1963, "bytes": 859146, "errors": 0}`
- For scripts and AI-agent callers (Claude runs cpr inside sessions now) — lets the caller verify the result from cpr's own output instead of re-listing the destination
- The `errors` field is the important one: today a partial failure isn't detectable from the summary line
