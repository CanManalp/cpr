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

```bash
# Copy a file
cpr source.txt destination.txt

# Copy a file into a directory
cpr report.pdf ./backup/

# Copy a directory (prompts for confirmation)
cpr ./my-project ./backup/my-project

# Copy a directory, excluding patterns
cpr ./my-project ./backup/my-project --exclude node_modules,.git,*.log

# Short flag
cpr ./src ./dist -e target,*.tmp
```

### Exclude patterns

Comma-separated, passed via `--exclude` / `-e`:

| Pattern | Matches |
|---|---|
| `node_modules` | Exact directory/file name |
| `.git` | Exact directory/file name |
| `*.log` | Any file ending in `.log` |
| `*.tmp` | Any file ending in `.tmp` |

## License

MIT
