# v

A small CLI program for viewing text files in the terminal with custom width word wrapping, optional syntax highlighting and optional pagination. Made for readers.

## Features

- Print a file's contents to the terminal (non-paginated by default).
- Syntax highlighting based on the file extension (enabled by default).
- Word wrapping at a configurable column width (default 80).
- Optional pagination.
- Persistent settings above can be set in a TOML config file.

## Usage

```bash
$ v <file>
```

### Examples

**View a markdown file with syntax highlighting and default wrapping**:

```bash
$ v papers/thiotimoline.md
```

**View a markdown file without syntax highlighting**:

```bash
$ v --syntax=off README.md
```

**Wrap text at 80 columns**:

```bash
$ v --width=80 notes.txt
```

**Use the terminal width for wrapping**:

```bash
$ v --column=0 annotations.txt
```

__Note:__ `--width` and `--column` are equivalent.

**Paginate output**:

```bash
$ v -p manual.md
```

**Combine options** (syntax off, width 100, paginated):

```bash
$ v -s off -w 100 -p src/index.php
```

## Options

| Option | Default | Description |
| --- | --- | --- |
| `-c`, `-w`, `--column=<N>`, `--width=<N>` | `80` (from config) | Wrap lines at `N` columns by word. `0` uses the terminal width. |
| `-s`, `--syntax[=<on\|off>]` | `on` (from config) | Enable or disable syntax highlighting. Bare `-s` is equivalent to `-s on`. |
| `-p`, `--page[=<on\|off>]` | off (from config) | Enable or disable pagination using `$PAGER` (defaults to `less -R`). Bare `-p` is equivalent to `-p on`. |
| `-h`, `--help` | | Print help information. |
| `-v`, `--version` | | Print version information. |

Running `v` or `v --help` with no file prints a custom help page with the program version, usage example, options, and configuration file path. Use `-v` to print the version alone.

## Configuration

On first run, `v` creates a TOML config file with default settings:

- `$XDG_CONFIG_HOME/v/v.conf`
- If `XDG_CONFIG_HOME` is not set, `~/.config/v/v.conf`

**Example**:

```toml
syntax = "on"
column = 90
page = false
```

Command-line options override values from the config file. Edit the config file to change defaults for future runs.

## Compiling

Clone or enter the repository, then build:

```bash
$ cargo build --release
```

The executable is written to `target/release/v`.

Run it directly without installing:

```bash
$ ./target/release/v README.md
```

### Requirements

- Rust 1.85+ (2024 edition)
- For pagination: `less` or another pager available on your system

## Installation

### Current user

Install the binary into `~/.local/bin` (make sure that directory is on your `PATH`):

```bash
$ cargo install --path . --force
```

Or copy the built binary manually:

```bash
$ mkdir -p ~/.local/bin
$ cp target/release/v ~/.local/bin/
```

### All users (system-wide)

Copy the binary into a directory on the system `PATH`, for example `/usr/local/bin`:

```bash
$ sudo cp target/release/v /usr/local/bin/
```

Alternatively, build and install with Cargo using a custom target directory:

```bash
$ cargo build --release
$ sudo install -m 755 target/release/v /usr/local/bin/v
```

## License

MIT
