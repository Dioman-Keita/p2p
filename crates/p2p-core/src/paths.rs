//! Validation of file names coming from the network, and naming of written files.

use std::path::{Component, Path, PathBuf};

use crate::{Error, Result};

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Turns a `/`-separated name received from a peer into a relative path that is guaranteed
/// to stay inside the destination directory, on every platform.
pub(crate) fn safe_relative_path(name: &str) -> Result<PathBuf> {
    let unsafe_name = || Error::UnsafeName(name.to_owned());
    if name.is_empty() || name.len() > 4096 {
        return Err(unsafe_name());
    }
    let mut out = PathBuf::new();
    for part in name.split('/') {
        let invalid_char = part
            .chars()
            .any(|c| c.is_control() || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'));
        let stem = part.split('.').next().unwrap_or("").trim_end();
        let reserved = WINDOWS_RESERVED
            .iter()
            .any(|r| r.eq_ignore_ascii_case(stem));
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with('.')
            || part.ends_with(' ')
            || invalid_char
            || reserved
        {
            return Err(unsafe_name());
        }
        out.push(part);
    }
    // Defense in depth: the result must consist of normal components only.
    if !out.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(unsafe_name());
    }
    Ok(out)
}

/// Returns `path`, or `name (1).ext`, `name (2).ext`... if it already exists.
pub(crate) fn non_existing(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_owned();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|i| path.with_file_name(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .expect("unbounded iterator")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_names() {
        assert_eq!(
            safe_relative_path("photos/2026/img.jpg").unwrap(),
            Path::new("photos").join("2026").join("img.jpg")
        );
        assert!(safe_relative_path(".hidden").is_ok());
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        for name in [
            "",
            "..",
            "../x",
            "a/../../x",
            "/etc/passwd",
            "a//b",
            "./a",
            "C:/Windows",
            "C:x",
            "a\\..\\b",
            "\\\\server\\share",
        ] {
            assert!(
                safe_relative_path(name).is_err(),
                "{name:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_windows_specific_names() {
        for name in [
            "CON",
            "nul.txt",
            "dir/com1.log",
            "a.",
            "a ",
            "a?b",
            "a\u{0}b",
        ] {
            assert!(
                safe_relative_path(name).is_err(),
                "{name:?} should be rejected"
            );
        }
    }

    #[test]
    fn picks_a_free_name() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.txt");
        assert_eq!(non_existing(&target), target);
        std::fs::write(&target, b"x").unwrap();
        assert_eq!(non_existing(&target), dir.path().join("a (1).txt"));
    }
}
