# Sample Markdown Document

## A subtitle with *emphasis* in it

This is a long paragraph that should wrap at one hundred columns by word boundaries rather than at the terminal edge or by breaking individual characters in the middle of words when displayed in the terminal.

## Code

```rust
fn main() {
    println!("hello");
}
```

**Bold text**, *italic text*, ***both***, and **nested *emphasis* inside** with more words to test wrapping behavior across multiple highlighted spans in a single line of markdown content here.

## Table

| Option | Default | Description |
| :--- | :---: | ---: |
| `--column` | 80 | Wrap width in columns, where `0` means the full terminal width |
| `--syntax` | on | Enable or disable syntax highlighting |
| `--table` | on | Enable or disable this table formatting |
