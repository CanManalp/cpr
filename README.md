# cpr

A fast file and directory copy tool with glob pattern filtering and parallel copying. Built because PowerShell's `Copy-Item` doesn't have those.

- **Glob pattern filtering** — `-e node_modules,.git,*.log` or `-i *.rs` instead of robocopy's `/XD node_modules /XF *.log`
- **Parallel by default** — copies files concurrently using all available cores
- **Progress bar** — real-time progress with bytes, throughput, and elapsed time (auto-hides when piped)
- **Human-readable output** — `Copied 2.44 GiB in 32 seconds (75.65 MiB/s) [70 files]`
- **Copy only what changed** — re-run the same command and only changed files are copied (`-u`); or continue an interrupted copy (`--skip-existing`)
- **Minimal overhead** — no ACLs, no retries, no attribute preservation, just copies bytes

![cpr progress bar](cpr_progress_bar.png)

## Installation

### Scoop (Windows)

```powershell
scoop bucket add cpr https://github.com/canmanalp/scoop-cpr
scoop install cpr
```

### Cargo

```bash
cargo install --git https://github.com/canmanalp/cpr
```

### Download binary

Grab the latest `cpr.exe` from [GitHub Releases](https://github.com/canmanalp/cpr/releases) and place it somewhere in your PATH.

### Build from source

```bash
git clone https://github.com/canmanalp/cpr
cd cpr
cargo build --release
# binary is at target/release/cpr.exe
```

## Usage

```
cpr <SOURCE> <DESTINATION> [OPTIONS]
```

```powershell
# Copy a file
cpr report.pdf D:\backup\

# Copy a directory (prompts for confirmation)
cpr C:\project\ D:\backup\project\

# Exclude patterns — skip files and folders that match
cpr C:\project\ D:\backup\project\ -e node_modules,.git,target -y

# Include patterns — copy only files that match
cpr C:\project\ D:\backup\project\ -i *.rs,*.toml

# Copy only a specific folder
cpr C:\project\ D:\backup\project\ -i src/**

# Combine include and exclude
cpr C:\project\ D:\backup\project\ -i *.rs,*.toml -e tests/**

# Preview what would be copied (no files are written)
cpr C:\project\ D:\backup\project\ -e node_modules -n

# Copy the same folder again — only files that changed since the last copy are copied
cpr C:\project\ D:\backup\project\ -y -u

# Continue an interrupted copy — files already there are not copied again
cpr C:\project\ D:\backup\project\ -y --skip-existing

# Preview which files a real run with -u would copy
cpr C:\project\ D:\backup\project\ -u -n
```

### Options

| Flag | Description |
|---|---|
| `-e, --exclude <PATTERNS>` | Patterns to exclude (files and directories) |
| `-i, --include <PATTERNS>` | Patterns to include (only matching files are copied) |
| `-y, --yes` | Skip confirmation prompt for directory copies |
| `-n, --dry-run` | Preview what would be copied without copying |
| `-u, --update` | Only copy files that are newer than the destination |
| `-s, --skip-existing` | Skip files that already exist at the destination |

Patterns can be comma-separated (`-e *.log,*.tmp`) or passed as multiple flags (`-e *.log -e *.tmp`).

### Glob patterns

Patterns follow the gitignore rule: a pattern **without** `/` matches entry names at any depth; a pattern **with** `/` matches against the path relative to the source root.

| Pattern | What it does |
|---|---|
| `node_modules` | Matches files/dirs named `node_modules` at any depth |
| `*.log` | Matches files by extension, at any depth |
| `*.rs,*.toml` | Matches multiple extensions |
| `**/*.test.js` | Matches in any subdirectory |
| `src/**` | Matches everything inside the top-level `src` |
| `-i src` | Include: copies every file under any folder named `src` |

### `--update` vs `--skip-existing`

Both flags skip files that are already at the destination, but they answer different questions:

- **`-u, --update`** asks: *"is my copy out of date?"*
  A file is skipped only when the destination has the same size **and** is at least as new as the source. If the source file changed since the last copy, it is copied again.
  Use it when you copy the same folder to the same destination again and again: each run copies only the files you changed since the last run.

- **`-s, --skip-existing`** asks: *"is the file already there?"*
  A file is skipped when the destination has a file with the same name and size. Modified times are ignored completely. A half-copied file (wrong size) is copied again.
  Use it to continue a copy that was interrupted — the finished files are left alone, the broken and missing ones are copied.

Rule of thumb: **copy stopped halfway → `--skip-existing`. Copying the same folder again to pick up changes → `-u`.**

The difference in one example: you edit `notes.txt` and it stays the same size. `-u` copies it again (the source is newer). `--skip-existing` does not (same name, same size — good enough). That makes `--skip-existing` faster to trust after a crash, but only `-u` notices your edits.

In both modes, files missing from the destination are always copied, and the summary shows how many files were skipped. Combine with `-n` to preview: `cpr src dst -u -n` lists exactly which files a real run would copy.

### How include and exclude interact

- **Exclude** applies to both files and directories — excluded directories are skipped entirely
- **Include** only filters files — directories are always walked so matching files inside them can be found
- When both are used, exclude is checked first

## License

MIT
