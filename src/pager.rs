use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Display content through a pager.
pub fn page_output(content: &str) -> io::Result<()> {
    let pager = std::env::var("PAGER").unwrap_or_else(|_| "less -R".to_string());
    let mut parts = pager.split_whitespace();
    let program = parts.next().unwrap_or("less");
    let mut args: Vec<&str> = parts.collect();

    if program == "less" && !args.iter().any(|arg| *arg == "-R" || arg.starts_with("-") && arg.contains('R'))
    {
        args.insert(0, "-R");
    }

    let mut child = Command::new(program)
        .args(&args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to launch pager `{program}`: {err}"),
            )
        })?;

    {
        child
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "pager stdin unavailable"))?
            .write_all(content.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() || status.code() == Some(1) {
        // `less` exits with 1 when the user quits before EOF.
        return Ok(());
    }

    Err(io::Error::other(format!("pager exited with {status}")))
}
