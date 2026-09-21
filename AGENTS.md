# AGENTS.md

Guidance for agents working on the `v` project.

## Project summary

`v` is a Rust CLI binary that displays text files in the terminal. It supports:

- syntax highlighting by file extension (on by default)
- word wrapping before syntax highlighting
- markdown tables redrawn as ASCII grid tables (on by default)
- markdown emphasis rendered with ANSI styling: `*italic*`, `**bold**`, `***both***`, with dimmed asterisks
- markdown headings rendered bold, with dimmed `#` marks
- markdown list item markers (`-`, `*`) rendered bold
- optional pagination via `$PAGER`

The binary name and crate name are both `v`.

## Compiling

From the repository root:

```bash
cargo build          # debug build
cargo build --release
cargo test
```

Release binary path:

```text
target/release/v
```

Run without installing:

```bash
./target/release/v path/to/file.rs
```

Install for the current user:

```bash
cargo install --path . --force
```

## Architecture

```text
src/
  main.rs       entry point, config load, clap dispatch
  cli.rs        CLI argument definitions and parsing
  config.rs     TOML config file path, load, and first-run creation
  viewer.rs     read file, orchestrate render + output
  highlight.rs  syntect-based syntax highlighting
  markdown.rs   markdown table detection and ASCII grid rendering
  emphasis.rs   markdown emphasis, heading and list marker styling over plain or highlighted text
  wrap.rs       terminal width + plain-text word wrapping
  pager.rs      pipe rendered output to $PAGER
```

Data flow:

1. `Config::ensure()` loads or creates `$XDG_CONFIG_HOME/v/v.conf` (or `~/.config/v/v.conf`).
2. CLI arguments are parsed; explicit flags override config values.
3. `viewer::run()` reads the file from disk.
4. Markdown table reformatting in `markdown.rs` (markdown extensions only).
5. Plain-text word wrapping in `wrap.rs`.
6. Optional highlighting in `highlight.rs` (includes ANSI reset at end).
7. Emphasis, heading and list marker styling in `emphasis.rs` (markdown extensions only).
8. Output to stdout, or through `pager.rs` when `-p` or `--page` is set.

## Key behavior

- `-s` / `--syntax=<on|off>` enables or disables highlighting; overrides config. Bare `-s` is equivalent to `-s on`.
- `-c` / `-w` / `--column=<N>` / `--width=<N>` set wrap width; `0` uses the terminal width.
- Default wrap width is 80 columns; overridable via config or `-c` / `--column` / `-w` / `--width`.
- Effective wrap width is the requested column count, or terminal width when column/width is `0`.
- Highlighted output ends with an ANSI reset (`\x1b[0m`) so terminal colors do not persist.
- `-p` / `--page[=<on|off>]` enables or disables pagination; overrides config. Bare `-p` is equivalent to `-p on`. When enabled, respects `$PAGER`; default pager command is `less -R`.
- `-t` / `--table[=<on|off>]` enables or disables markdown table formatting; overrides config. Bare `-t` is equivalent to `-t on`. Only applies to markdown extensions (`md`, `markdown`, `mdown`, `mkd`, `mdx`).
- Markdown tables are laid out to fit the effective wrap width, so word wrapping leaves them intact. Cells wrap inside their column; rows get separating rules when any row wraps. Tables in fenced or indented code blocks, and tables that cannot fit even at the minimum column width, are left as written.
- Markdown emphasis is printed with ANSI styling: `*italic*` (`\x1b[3m`), `**bold**` (`\x1b[1m`), `***both***`; delimiters carry no emphasis themselves and are painted a fixed dim gray (`\x1b[38;2;96;96;96m`), not faint (`\x1b[2m`), which terminals dim by wildly different amounts. The foreground in effect is tracked while scanning and restored after each delimiter. Markdown extensions only, and not configurable.
- ATX heading text (`#` through `######`, up to three spaces of indent, `#` run followed by a space or the line end) is bold, with the `#` run dimmed like an emphasis delimiter. Emphasis inside a heading nests on top of the bold. A heading that word wrapping split stays bold on the lines it wrapped onto, which is why `style_emphasis` takes the wrap width.
- List item markers are bold: a `-` or a single `*` at the start of a line (after any indent), followed by a space, a tab or the end of the line. The marker keeps the foreground it had; only the intensity changes. Thematic breaks (`---`, `* * *`) are not bullets, and text already bold (heading text) is left as is.
- Emphasis styling runs last, on plain or highlighted text, so it adds only zero-width escapes to already wrapped lines. Spans nest, and may cross wrapped lines but not blank lines; fenced code, indented code, inline code spans and `\*` escapes are skipped. Opening and closing runs must match in length and hug their text, so `2 * 3` and `* list item` open no span.
- Unknown file extensions fall back to plain text (no highlighting).
- Config file: `$XDG_CONFIG_HOME/v/v.conf` or `~/.config/v/v.conf` (TOML). Created on first run.
- Command-line flags override config file values.
- `v` and `v --help` (without a file) print a custom help page with version, usage example, options, and config path.
- `-v` print version information (`v v<version>`).

## Dependencies

| Crate | Purpose |
| --- | --- |
| `clap` | CLI parsing |
| `serde` | config (de)serialization |
| `syntect` | syntax highlighting |
| `terminal_size` | terminal column detection |
| `toml` | TOML config file format |

Prefer latest stable crate versions when adding or updating dependencies.

## Conventions

- Keep modules focused and small; avoid growing `main.rs` beyond bootstrapping.
- Wrap plain text before highlighting; do not wrap ANSI output.
- Reformat markdown before wrapping, so the wrapper sees final line widths.
- Add ANSI styling only after wrapping; escape sequences must never change display width.
- Add unit tests for wrapping edge cases and CLI flag parsing.
- Match existing error style: print `v: <message>` to stderr and exit with code `1`.
- When a new implementation or change is made:
  - Check if README.md needs to be updated.
  - Check if AGENTS.md needs to be updated.
  - Bump the project version following the SEMVER 2.0.0 conventions.

## Useful commands

```bash
cargo test
cargo clippy -- -D warnings
./target/release/v --help
./target/release/v examples/sample.php
```

## Sample files

`examples/sample.php` is a small PHP file useful for manual testing of highlighting and wrapping.
`examples/sample.md` covers markdown wrapping, bold styling, lists, and table formatting (including alignment and wrapped cells).
