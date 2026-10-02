use std::cell::OnceCell;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::text::file_extension;

#[derive(Debug)]
pub(super) struct ProjectFiles {
    names: HashSet<String>,
    /// Dependency names from `package.json`, read on first use.
    node_dependencies: OnceCell<Vec<String>>,
}

impl ProjectFiles {
    pub(super) fn read(project_root: &Path) -> Option<Self> {
        let entries = std::fs::read_dir(project_root).ok()?;
        let names = entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();

        Some(Self {
            names,
            node_dependencies: OnceCell::new(),
        })
    }

    pub(super) fn contains_exact(&self, target: &str) -> bool {
        self.names.contains(target)
    }

    pub(super) fn contains_prefix(&self, prefix: &str) -> bool {
        self.names
            .iter()
            .any(|name| matches_config_name_prefix(name, prefix))
    }

    pub(super) fn any_exact(&self, targets: &[&str]) -> bool {
        targets.iter().any(|target| self.contains_exact(target))
    }

    pub(super) fn contains_extension(&self, target_extension: &str) -> bool {
        self.names
            .iter()
            .any(|name| file_extension(name) == Some(target_extension))
    }

    /// Whether `relative` exists in the project root.
    ///
    /// Plain names use the cached directory listing. Nested paths such as
    /// `bin/rails` are checked on the filesystem without opening the file.
    pub(super) fn contains_path(&self, project_root: &Path, relative: &str) -> bool {
        if relative.contains('/') {
            project_root.join(relative).is_file()
        } else {
            self.contains_exact(relative)
        }
    }

    /// Read the first [`MAX_SCAN_BYTES`] of a project file as text.
    ///
    /// Returns `None` when the name is not in the listing, is not a regular
    /// file, or cannot be read. Undecodable bytes become U+FFFD instead of
    /// failing the read, so one Latin-1 comment or a UTF-8 character split at
    /// the cap does not hide the rest of the file.
    pub(super) fn read_text(&self, project_root: &Path, file_name: &str) -> Option<String> {
        if !self.contains_exact(file_name) {
            return None;
        }

        read_text_file(&project_root.join(file_name))
    }

    /// Whether `package.json` lists any of `packages` under
    /// `"dependencies"` or `"devDependencies"`.
    pub(super) fn has_node_dependency(&self, project_root: &Path, packages: &[&str]) -> bool {
        self.node_dependencies
            .get_or_init(|| {
                self.read_text(project_root, "package.json")
                    .map(|json| super::node::dependency_names(&json))
                    .unwrap_or_default()
            })
            .iter()
            .any(|name| packages.contains(&name.as_str()))
    }

    /// Like [`read_text`](Self::read_text), but `None` when the file is
    /// longer than [`MAX_SCAN_BYTES`], for callers that must see every line.
    pub(super) fn read_complete_text(
        &self,
        project_root: &Path,
        file_name: &str,
    ) -> Option<String> {
        if !self.contains_exact(file_name) {
            return None;
        }

        read_regular_file_prefix(&project_root.join(file_name))
            .filter(|(_, complete)| *complete)
            .map(|(bytes, _)| decode_text(bytes))
    }
}

/// Suffixes accepted after a config name prefix: `next.config` matches
/// `next.config`, `next.config.js`, `next.config.mjs`, and so on.
const COMMON_CONFIG_SUFFIXES: &[&str] = &["", ".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];

fn matches_config_name_prefix(name: &str, pattern: &str) -> bool {
    name.strip_prefix(pattern)
        .is_some_and(|suffix| COMMON_CONFIG_SUFFIXES.contains(&suffix))
}

/// Read the first [`MAX_SCAN_BYTES`] of a regular file as text, with the same
/// safety and decoding rules as [`ProjectFiles::read_text`].
pub(super) fn read_text_file(path: &Path) -> Option<String> {
    read_regular_file_prefix(path).map(|(bytes, _)| decode_text(bytes))
}

/// Maximum number of bytes read from one project file.
const MAX_SCAN_BYTES: u64 = 64 * 1024;

/// Read up to [`MAX_SCAN_BYTES`] from `path` only when it is a regular file,
/// with whether that covered the whole file.
///
/// A FIFO (or a symlink to `/dev/tty`) under a scanned name such as `app.py`
/// would block a plain `open` forever. The type is checked before opening, the
/// open is non-blocking on Unix, and the opened handle is checked again, which
/// closes the race where the path is swapped between the two checks.
///
/// Windows has no FIFOs in directory listings and refuses to open a directory
/// as a file, so the check after opening is enough there and the extra
/// metadata call is skipped.
fn read_regular_file_prefix(path: &Path) -> Option<(Vec<u8>, bool)> {
    #[cfg(not(windows))]
    if !std::fs::metadata(path).ok()?.is_file() {
        return None;
    }

    let file = open_for_scan(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() {
        return None;
    }

    let mut bytes = Vec::new();
    file.take(MAX_SCAN_BYTES).read_to_end(&mut bytes).ok()?;
    Some((bytes, metadata.len() <= MAX_SCAN_BYTES))
}

#[cfg(unix)]
fn open_for_scan(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    // O_NONBLOCK makes opening a FIFO return at once; it has no effect on
    // reads from regular files.
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
}

#[cfg(not(unix))]
fn open_for_scan(path: &Path) -> io::Result<File> {
    File::open(path)
}

/// Decode file bytes to text without failing.
///
/// UTF-16 files with a byte-order mark (for example `pip freeze >` output from
/// Windows PowerShell 5.1) are decoded as UTF-16. Everything else is decoded
/// as UTF-8, replacing invalid sequences.
fn decode_text(bytes: Vec<u8>) -> String {
    match bytes.as_slice() {
        [0xFF, 0xFE, rest @ ..] => decode_utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => decode_utf16(rest, u16::from_be_bytes),
        _ => String::from_utf8(bytes)
            .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned()),
    }
}

fn decode_utf16(bytes: &[u8], unit_from_bytes: fn([u8; 2]) -> u16) -> String {
    let (pairs, _odd_trailing_byte) = bytes.as_chunks::<2>();
    let units = pairs.iter().copied().map(unit_from_bytes);

    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_with_bom(text: &str, bom: [u8; 2], to_bytes: fn(u16) -> [u8; 2]) -> Vec<u8> {
        bom.into_iter()
            .chain(text.encode_utf16().flat_map(to_bytes))
            .collect()
    }

    #[test]
    fn decode_text_reads_utf16_with_byte_order_mark() {
        let little = utf16_with_bom("Flask==3.0\r\n", [0xFF, 0xFE], u16::to_le_bytes);
        assert_eq!(decode_text(little), "Flask==3.0\r\n");

        let big = utf16_with_bom("fastapi\n", [0xFE, 0xFF], u16::to_be_bytes);
        assert_eq!(decode_text(big), "fastapi\n");
    }

    #[test]
    fn decode_text_replaces_invalid_utf8_instead_of_failing() {
        assert_eq!(decode_text(b"caf\xe9 flask".to_vec()), "caf\u{fffd} flask");
        assert_eq!(
            decode_text(vec![b'a', 0xC3]),
            "a\u{fffd}",
            "a character split at the read cap"
        );
        assert_eq!(decode_text(b"plain".to_vec()), "plain");
    }

    #[cfg(unix)]
    fn make_fifo(path: &Path) {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let c_path = CString::new(path.as_os_str().as_bytes()).expect("path has no nul byte");
        // Safety: `c_path` is a valid nul-terminated string for the whole call.
        let status = unsafe { libc::mkfifo(c_path.as_ptr(), 0o644) };
        assert_eq!(status, 0, "mkfifo {}", path.display());
    }

    #[cfg(unix)]
    #[test]
    fn fifos_with_project_file_names_are_skipped_without_blocking() {
        use std::sync::mpsc;
        use std::time::Duration;

        let dir = tempfile::TempDir::new().expect("temp dir");
        make_fifo(&dir.path().join("app.py"));
        make_fifo(&dir.path().join("requirements.txt"));

        let root = dir.path().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let files = ProjectFiles::read(&root).expect("read project dir");
            let text = files.read_text(&root, "app.py");
            let label = crate::detect_from_config(&root);
            sender.send((text, label)).expect("receiver is waiting");
        });

        let (text, label) = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("reading a FIFO must not block");
        assert_eq!(text, None, "a FIFO is not a regular file");
        assert_eq!(label.expect("Python markers are still listed"), "Python");
    }
}
