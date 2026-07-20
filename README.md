# v

A small Rust CLI for viewing text files in the terminal with syntax highlighting, word wrapping, and optional pagination.

## Features

- Print a file's contents to the terminal (non-paginated by default)
- Syntax highlighting based on the file extension (enabled by default)
- Word wrapping at a configurable column width
- Optional pagination through `$PAGER` (defaults to `less -R`)

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
v --column=80 notes.txt
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
v --syntax=0 --column=100 --page src/lib.rs
```

## Options

| Option | Default | Description |
| --- | --- | --- |
| `--syntax=<on\|off>` | `on` | Enable or disable syntax highlighting. `off`, `0`, `false`, and `no` turn it off. |
| `--column=<N>` | `120` | Wrap lines at `N` columns by word. `0` uses the terminal width. |
| `--page` | off | Paginate output using `$PAGER` (defaults to `less -R`). |

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

- Default wrap width is 120 columns.
- Lines are wrapped by word before syntax highlighting is applied.
- `--column=0` uses the current terminal width.
- Words longer than the wrap width are broken character-by-character as a fallback.

## Syntax highlighting

Highlighting is driven by [syntect](https://github.com/trishume/syntect) and Sublime Text syntax definitions. If no syntax matches the file extension, the file is shown as plain text.

## Environment variables

| Variable | Description |
| --- | --- |
| `PAGER` | Command used when `--page` is passed. Defaults to `less -R`. |

## License

MIT
