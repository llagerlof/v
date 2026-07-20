# AGENTS.md

Guidance for agents working on the `v` project.

## Project summary

`v` is a Rust CLI binary that displays text files in the terminal. It supports:

- syntax highlighting by file extension (on by default)
- word wrapping before syntax highlighting
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
  main.rs       entry point, clap dispatch
  cli.rs        CLI argument definitions and parsing
  viewer.rs     read file, orchestrate render + output
  highlight.rs  syntect-based syntax highlighting
  wrap.rs       terminal width + plain-text word wrapping
  pager.rs      pipe rendered output to $PAGER
```

Data flow:

1. `Cli::parse()` reads arguments.
2. `viewer::run()` reads the file from disk.
3. Plain-text word wrapping in `wrap.rs`.
4. Optional highlighting in `highlight.rs` (includes ANSI reset at end).
5. Output to stdout, or through `pager.rs` when `--page` is set.

## Key behavior

- `--syntax=off` and `--syntax=0` disable highlighting.
- `--column=0` means "use terminal width".
- Effective wrap width is the requested column count, or terminal width when `--column=0`.
- Highlighted output ends with an ANSI reset (`\x1b[0m`) so terminal colors do not persist.
- `--page` respects `$PAGER`; default pager command is `less -R`.
- Unknown file extensions fall back to plain text (no highlighting).

## Dependencies

| Crate | Purpose |
| --- | --- |
| `clap` | CLI parsing |
| `syntect` | syntax highlighting |
| `terminal_size` | terminal column detection |

Prefer latest stable crate versions when adding or updating dependencies.

## Conventions

- Keep modules focused and small; avoid growing `main.rs` beyond bootstrapping.
- Wrap plain text before highlighting; do not wrap ANSI output.
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
./target/release/v examples/sample.rs
```

## Sample file

`examples/sample.rs` is a small Rust file useful for manual testing of highlighting and wrapping.
