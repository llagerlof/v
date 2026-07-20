# v

A small Rust CLI for viewing text files in the terminal with syntax highlighting, word wrapping, and optional pagination.

## Features

- Print a file's contents to the terminal (non-paginated by default)
- Syntax highlighting based on the file extension (enabled by default)
- Word wrapping at a configurable column width (default 100)
- Optional pagination through `$PAGER` (defaults to `less -R`)
- Persistent settings in a TOML config file

## Usage

```bash
v <file>
```

### Examples

View a Rust source file with syntax highlighting and default wrapping:

```bash
v src/main.rs
```

Disable syntax highlighting:

```bash
v --syntax=off README.md
```

Wrap at 80 columns:

```bash
v --width=80 notes.txt
```

Use the terminal width for wrapping:

```bash
v --column=0 wide.txt
```

Paginate output:

```bash
v --page manual.md
```

Combine options:

```bash
v -s off -w 100 -p src/lib.rs
```

## Options

| Option | Default | Description |
| --- | --- | --- |
| `-c`, `-w`, `--column=<N>`, `--width=<N>` | `100` (from config) | Wrap lines at `N` columns by word. `0` uses the terminal width. |
| `-s`, `--syntax=<on\|off>` | `on` (from config) | Enable or disable syntax highlighting. `off`, `0`, `false`, and `no` turn it off. |
| `-p`, `--page` | off (from config) | Paginate output using `$PAGER` (defaults to `less -R`). |
| `-h`, `--help` | | Print help information. |
| `-v`, `--version` | | Print version information. |

Running `v` or `v --help` with no file prints a custom help page with the program version, usage example, options, and configuration file path. Use `-v` to print the version alone.

## Configuration

On first run, `v` creates a TOML config file with default settings:

- `$XDG_CONFIG_HOME/v/v.conf` when `XDG_CONFIG_HOME` is set
- otherwise `~/.config/v/v.conf`

Example:

```toml
syntax = "on"
column = 100
page = false
```

Command-line options override values from the config file. Edit the config file to change defaults for future runs.

## Requirements

- Rust 1.74+ (2021 edition)
- For pagination: `less` or another pager available on your system

## Compiling

Clone or enter the repository, then build:

```bash
cargo build --release
```

The executable is written to `target/release/v`.

Run it directly without installing:

```bash
./target/release/v README.md
```

## Installation

### Current user

Install the binary into `~/.local/bin` (make sure that directory is on your `PATH`):

```bash
cargo install --path . --force
```

Or copy the built binary manually:

```bash
mkdir -p ~/.local/bin
cp target/release/v ~/.local/bin/
```

### All users (system-wide)

Copy the binary into a directory on the system `PATH`, for example `/usr/local/bin`:

```bash
sudo cp target/release/v /usr/local/bin/
```

Alternatively, build and install with Cargo using a custom target directory:

```bash
cargo build --release
sudo install -m 755 target/release/v /usr/local/bin/v
```

## How wrapping works

- Default wrap width is 100 columns (configurable in `v.conf`).
- Lines are wrapped by word before syntax highlighting is applied.
- `--column=0` and `--width=0` use the current terminal width.
- Words longer than the wrap width are broken character-by-character as a fallback.

## Syntax highlighting

Highlighting is driven by [syntect](https://github.com/trishume/syntect) and Sublime Text syntax definitions. If no syntax matches the file extension, the file is shown as plain text.

## Environment variables

| Variable | Description |
| --- | --- |
| `XDG_CONFIG_HOME` | Base directory for the config file (`$XDG_CONFIG_HOME/v/v.conf`). |
| `PAGER` | Command used when `-p` or `--page` is passed. Defaults to `less -R`. |

## License

MIT
