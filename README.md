# v

A *small CLI* program for viewing text files in the terminal with custom width word wrapping, optional syntax highlighting and optional pagination. Made for readers.

## Features

- *Print a file's contents to the terminal* (non-paginated by default).
- Syntax highlighting based on the file extension (enabled by default).
- Word wrapping at a configurable column width (default 80).
- Markdown tables redrawn as ASCII grid tables (enabled by default).
- Markdown emphasis rendered with real terminal styling: `*italic*`, `**bold**` and `***both***`, with dimmed asterisks.
- Markdown headings shown in bold, with dimmed `#` marks.
- Markdown list item markers (`-` and `*`) shown in bold.
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

**View a markdown file with its tables left as-is**:

```bash
$ v --table=off CHANGELOG.md
```

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
| `-c`, `-w`, `--column=<N>`, `--width=<N>` | `80` (or from config) | Wrap lines at `N` columns by word. `0` uses the terminal width. |
| `-s`, `--syntax[=<on\|off>]` | `on` (from config) | Enable or disable syntax highlighting. Bare `-s` is equivalent to `-s on`. |
| `-p`, `--page[=<on\|off>]` | off (from config) | Enable or disable pagination using `$PAGER` (defaults to `less -R`). Bare `-p` is equivalent to `-p on`. |
| `-t`, `--table[=<on\|off>]` | `on` (from config) | Enable or disable markdown table formatting. Bare `-t` is equivalent to `-t on`. |
| `-h`, `--help` | | Print help information. |
| `-v`, `--version` | | Print version information. |

Running `v` or `v --help` with no file prints a custom help page with the program version, usage example, options, and configuration file path. Use `-v` to print the version alone.

## Markdown tables

In markdown files (`.md`, `.markdown`, `.mdown`, `.mkd`, `.mdx`), pipe tables are redrawn as ASCII grid tables so columns line up:

```text
| Crate    | Purpose               |    +----------+-----------------------+
| -------- | --------------------- | -> | Crate    | Purpose               |
| clap     | CLI parsing           |    +----------+-----------------------+
| syntect  | syntax highlighting   |    | clap     | CLI parsing           |
                                        | syntect  | syntax highlighting   |
                                        +----------+-----------------------+
```

- Column alignment from the delimiter row (`:---`, `:---:`, `---:`) is respected.
- Tables are laid out to fit the wrap width; long cells wrap inside their column, and rows are separated by a rule when that happens.
- Tables inside fenced or indented code blocks are left untouched, as is a table too wide to fit even at its minimum column width.

Use `-t off` or `table = "off"` in the config to print the original markdown instead.

## Markdown emphasis and headings

In markdown files, asterisk emphasis is printed with the terminal's own styling: `*one asterisk*` is
italic, `**two**` is bold and `***three***` is both. The asterisks are kept, but carry no emphasis
of their own and are painted a dim gray, so the emphasized words stand out from their markers. Every
delimiter fades to the same gray, whatever color the syntax highlighter gave it.

- Emphasis nests: in `**bold *and italic* again**`, the inner span is both.
- A span may cross a wrapped line, but not a blank line.
- A delimiter must hug its text, so `2 * 3`, `*.rs` globs and `* list item` markers open nothing.
- Fenced code blocks, indented code blocks and inline code spans are left as written, so `**kwargs`
  in a code sample stays plain. Escaped markers (`\*\*`) are left alone too.
- Styling is applied after wrapping, so it adds no width and leaves table columns aligned.

Headings get the same treatment: the text after `#`, `##`, `###` and so on is bold, and the `#`
marks are dimmed to the same gray as the asterisks.

- Emphasis inside a heading still applies, on top of the heading's bold.
- A heading long enough to wrap stays bold across the lines it wrapped onto.
- `#hashtag`, `C#` and a `#` inside a code block are left alone, as are more than six `#` marks.

List item markers are bold too: the `-` or `*` that opens a list item, at any indentation, keeps its
color but is printed bold so bullets stand out from the text they introduce.

- Only a marker followed by a space, a tab or the end of the line counts, so `well-known`, `2 * 3`
  and an `em -- dash` stay plain.
- Thematic breaks (`---`, `* * *`) and markers inside code blocks are left alone.

Terminals without italic support fall back to their own substitute (often the normal or the inverse
style); bold and color are supported everywhere.

## Configuration

On first run, `v` creates a TOML config file with default settings:

- `$XDG_CONFIG_HOME/v/v.conf`
- If `XDG_CONFIG_HOME` is not set, it uses `~/.config/v/v.conf`

**Example**:

```toml
syntax = "on"
column = 90
page = false
table = "on"
```

Command-line options override values from the config file. Edit the config file to change defaults for future runs.

## Installation

Download a compressed release file from releases page (https://github.com/llagerlof/v/releases), extract the binary `v` an copy it to `/usr/local/bin/` or `~/.local/bin/`.

## Compiling

Clone or enter the repository, then build:

```bash
$ cargo build --release
```

The executable is written to `target/release/v`.

## License

MIT
