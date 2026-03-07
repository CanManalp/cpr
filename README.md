# cpr

A file and directory copy tool with `--exclude` support. Built because PowerShell's `Copy-Item` doesn't have one.

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

# Copy with exclude patterns and skip confirmation
cpr C:\project\ D:\backup\project\ -e node_modules,.git,*.log -y
```

### Options

| Flag | Description |
|---|---|
| `-e, --exclude <PATTERNS>` | Comma-separated patterns to exclude |
| `-y, --yes` | Skip confirmation prompt for directory copies |

### Exclude patterns

| Pattern | Matches |
|---|---|
| `node_modules` | Exact directory/file name |
| `.git` | Exact directory/file name |
| `*.log` | Any file ending in `.log` |

## License

MIT
