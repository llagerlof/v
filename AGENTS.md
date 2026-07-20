# AGENTS.md

Guidance for agents working on the `v` project.

## Project summary

`v` is a Rust CLI binary that displays text files in the terminal. It supports:

- syntax highlighting by file extension (on by default)
- word wrapping with ANSI-aware logic
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
  wrap.rs       terminal width + ANSI-aware word wrapping
  pager.rs      pipe rendered output to $PAGER
```

Data flow:

1. `Cli::parse()` reads arguments.
2. `viewer::run()` reads the file from disk.
3. Optional highlighting in `highlight.rs`.
4. Wrapping in `wrap.rs`.
5. Output to stdout, or through `pager.rs` when `--page` is set.

## Key behavior

- `--syntax=off` and `--syntax=0` disable highlighting.
- `--column=0` means "use terminal width".
- Effective wrap width is `max(requested_column, terminal_width)` unless `requested_column == 0`.
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
- Preserve ANSI escape sequences when wrapping highlighted output.
- Add unit tests for wrapping edge cases and CLI flag parsing.
- Match existing error style: print `v: <message>` to stderr and exit with code `1`.
- Do not edit `README.md` or this file unless the user asks for documentation changes.

## Useful commands

```bash
cargo test
cargo clippy -- -D warnings
./target/release/v --help
./target/release/v examples/sample.rs
```

## Sample file

`examples/sample.rs` is a small Rust file useful for manual testing of highlighting and wrapping.
