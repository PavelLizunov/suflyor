use super::time::now_unix_ms;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Append a user-bookmarked Q/A pair to `%APPDATA%\suflyor\bookmarks.md`.
pub fn append_bookmark(question: &str, answer: &str) -> Result<PathBuf> {
    let dir = crate::paths::data_root().context("no config dir")?;
    std::fs::create_dir_all(&dir).context("create data dir")?;
    let path = dir.join("bookmarks.md");
    let is_new = !path.exists();
    let mut opts = OpenOptions::new();
    opts.create(true).append(true);
    // SECURITY: Restrict bookmarks file permissions to owner-only (0o600) on POSIX
    // so bookmarked session Q&A contents are protected from other system users.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&path).context("open bookmarks.md")?;
    if is_new {
        writeln!(
            f,
            "# suflyor bookmarks\n\nQ/A snippets bookmarked from the overlay bar chip.\n"
        )
        .context("write bookmarks header")?;
    }
    let stamp = now_unix_ms();
    writeln!(
        f,
        "---\n\n## {stamp}\n\n**Q:** {q}\n\n**A:**\n\n{a}\n",
        q = question.trim(),
        a = answer.trim()
    )
    .context("write bookmark entry")?;
    Ok(path)
}
