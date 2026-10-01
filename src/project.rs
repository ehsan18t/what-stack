//! Project-root detection.
//!
//! Project detection walks upward from process paths and looks for marker
//! files such as `package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`, and
//! project-file extensions such as `.csproj` and `.fsproj`.

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::CStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use crate::ProjectInput;

const PROJECT_MARKERS: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "go.work",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "Pipfile",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "composer.json",
    "Gemfile",
    "mix.exs",
    "deno.json",
    "deno.jsonc",
    "bun.lockb",
    "bun.lock",
];

const PROJECT_MARKER_EXTENSIONS: &[&str] = &["csproj", "fsproj"];

/// Maximum number of directories tested during one upward project walk.
///
/// The starting directory counts as the first. This cap is a safety net for
/// unusual paths and very deep directory trees. The value is intentionally high
/// enough for common monorepos while bounding worst-case filesystem work.
pub const MAX_WALK_DEPTH: usize = 64;

/// Resolve a project root from process-like path inputs without caching.
///
/// The fallback order is:
///
/// 1. Walk upward from the working directory set by [`ProjectInput::cwd`].
/// 2. Walk upward from the parent directory of the executable set by
///    [`ProjectInput::exe`].
/// 3. Walk upward from the parent directory of each absolute path in the
///    arguments set by [`ProjectInput::cmd`].
///
/// The first marker hit wins. Relative command-line paths are ignored because
/// they are ambiguous without a reliable process working directory. `home` is
/// the same optional ceiling as in [`find_project_root`].
///
/// Use [`StackDetector::detect_project_root`](crate::StackDetector::detect_project_root)
/// for repeated lookups; it applies the same order with caching and its own
/// configured home ceiling.
///
/// # Examples
///
/// ```
/// use what_stack::{ProjectInput, resolve_project_root};
///
/// assert_eq!(resolve_project_root(ProjectInput::new(), None), None);
/// ```
#[must_use]
pub fn resolve_project_root(input: ProjectInput<'_>, home: Option<&Path>) -> Option<PathBuf> {
    project_root_candidates(input).find_map(|start| find_project_root(start, home))
}

/// Starting directories for a project walk, in fallback order.
pub fn project_root_candidates(input: ProjectInput<'_>) -> impl Iterator<Item = &Path> + '_ {
    input
        .cwd
        .into_iter()
        .chain(input.exe.and_then(Path::parent))
        .chain(absolute_cmd_parents(input.cmd))
}

/// Walk upward from `start` looking for project marker files.
///
/// `home` is an optional ceiling. When the walk reaches `home`, it stops before
/// testing that directory for markers. This avoids accidental matches from
/// marker files stored directly in a user's home directory. On Windows the
/// ceiling comparison ignores ASCII case, matching the file system.
///
/// At most [`MAX_WALK_DEPTH`] directories are tested, starting with `start`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use what_stack::find_project_root;
///
/// assert_eq!(find_project_root(Path::new("."), Some(Path::new("."))), None);
/// ```
#[must_use]
pub fn find_project_root(start: &Path, home: Option<&Path>) -> Option<PathBuf> {
    walk_ancestors(start, home).find(|dir| has_marker(dir))
}

/// Return the display name for a project root path.
///
/// The returned label is derived only from the final path component. It does
/// not read package manifests or normalize workspace names.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use what_stack::project_name;
///
/// assert_eq!(project_name(Path::new("/workspace/api")).as_deref(), Some("api"));
/// ```
#[must_use]
pub fn project_name(root: &Path) -> Option<Cow<'_, str>> {
    root.file_name().map(OsStr::to_string_lossy)
}

pub fn walk_ancestors<'a>(
    start: &'a Path,
    home: Option<&'a Path>,
) -> impl Iterator<Item = PathBuf> + 'a {
    let mut current = Some(start.to_path_buf());
    let mut depth = 0;

    std::iter::from_fn(move || {
        let dir = current.as_ref()?.clone();

        if depth >= MAX_WALK_DEPTH {
            current = None;
            return None;
        }

        if let Some(home_dir) = home
            && paths_equal(&dir, home_dir)
        {
            current = None;
            return None;
        }

        depth += 1;

        let mut next = dir.clone();
        if next.pop() && next != dir {
            current = Some(next);
        } else {
            current = None;
        }

        Some(dir)
    })
}

fn absolute_cmd_parents(cmd: &[OsString]) -> impl Iterator<Item = &Path> + '_ {
    cmd.iter().filter_map(|arg| {
        let path = Path::new(arg.as_os_str());
        path.is_absolute().then(|| path.parent()).flatten()
    })
}

pub fn has_marker(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            return false;
        };

        PROJECT_MARKERS.contains(&name)
            || Path::new(name)
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| PROJECT_MARKER_EXTENSIONS.contains(&extension))
    })
}

/// Compare two paths component by component.
///
/// Windows file systems are case-insensitive, so components are compared
/// ignoring ASCII case there. Other platforms compare exactly.
pub fn paths_equal(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        let mut left = left.components();
        let mut right = right.components();
        loop {
            match (left.next(), right.next()) {
                (None, None) => return true,
                (Some(a), Some(b)) if components_equal(a, b) => {}
                _ => return false,
            }
        }
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

/// Return whether `path` lies at or below `prefix`, comparing whole components
/// with the same case rules as [`paths_equal`].
pub fn path_starts_with(path: &Path, prefix: &Path) -> bool {
    #[cfg(windows)]
    {
        let mut path = path.components();
        prefix.components().all(|expected| {
            path.next()
                .is_some_and(|actual| components_equal(actual, expected))
        })
    }
    #[cfg(not(windows))]
    {
        path.starts_with(prefix)
    }
}

#[cfg(windows)]
fn components_equal(left: std::path::Component<'_>, right: std::path::Component<'_>) -> bool {
    left.as_os_str().eq_ignore_ascii_case(right.as_os_str())
}

/// Return the current user's home directory, when it can be determined.
///
/// On Unix, this prefers passwd-database lookup for the invoking user. During
/// `sudo` sessions it uses `SUDO_UID` to find the original user's home, falling
/// back to `SUDO_HOME` and then `HOME` when passwd lookup is unavailable. On
/// Windows, it reads `USERPROFILE`. On other targets (for example `wasm32`), it
/// returns `None`.
///
/// [`StackDetector::new`](crate::StackDetector::new) calls this once to set its
/// home ceiling. Callers of [`find_project_root`] or [`resolve_project_root`]
/// should call it once and reuse the result.
#[must_use]
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        select_home_dir(
            preferred_home_uid().and_then(home_dir_from_uid),
            sudo_home_dir(),
            std::env::var_os("HOME").map(PathBuf::from),
        )
    }
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[cfg(unix)]
fn select_home_dir(
    passwd_home: Option<PathBuf>,
    sudo_home: Option<PathBuf>,
    env_home: Option<PathBuf>,
) -> Option<PathBuf> {
    passwd_home.or(sudo_home).or(env_home)
}

#[cfg(unix)]
fn sudo_home_dir() -> Option<PathBuf> {
    (current_effective_uid() == 0)
        .then(|| std::env::var_os("SUDO_HOME"))
        .flatten()
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

#[cfg(unix)]
fn preferred_home_uid() -> Option<libc::uid_t> {
    preferred_home_uid_from_env(
        std::env::var_os("SUDO_UID").as_deref(),
        current_effective_uid(),
    )
}

#[cfg(unix)]
fn preferred_home_uid_from_env(
    sudo_uid: Option<&OsStr>,
    current_euid: libc::uid_t,
) -> Option<libc::uid_t> {
    if current_euid == 0 {
        sudo_uid
            .and_then(OsStr::to_str)
            .and_then(|value| value.parse::<libc::uid_t>().ok())
            .or(Some(current_euid))
    } else {
        Some(current_euid)
    }
}

#[cfg(unix)]
fn current_effective_uid() -> libc::uid_t {
    // Safety: `geteuid` has no preconditions and only returns the caller's euid.
    unsafe { libc::geteuid() }
}

#[cfg(unix)]
fn home_dir_from_uid(uid: libc::uid_t) -> Option<PathBuf> {
    let mut buffer = vec![0_u8; passwd_buffer_len()];
    let mut passwd = std::mem::MaybeUninit::<libc::passwd>::zeroed();
    let mut result = std::ptr::null_mut();
    // Safety: all pointers reference valid stack/heap storage for this call,
    // and `buffer` remains alive until `passwd.pw_dir` has been copied.
    let status = unsafe {
        libc::getpwuid_r(
            uid,
            passwd.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &raw mut result,
        )
    };

    if status != 0 || result.is_null() {
        return None;
    }

    // Safety: `getpwuid_r` succeeded and initialized `passwd`.
    let passwd = unsafe { passwd.assume_init() };
    if passwd.pw_dir.is_null() {
        return None;
    }

    // Safety: successful passwd records expose a nul-terminated directory path.
    let home = unsafe { CStr::from_ptr(passwd.pw_dir) };
    Some(Path::new(OsStr::from_bytes(home.to_bytes())).to_path_buf())
}

#[cfg(unix)]
fn passwd_buffer_len() -> usize {
    const DEFAULT_PASSWD_BUFFER_LEN: usize = 1024;

    // Safety: `sysconf` has no preconditions for this constant.
    match unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) } {
        size if size > 0 => usize::try_from(size).unwrap_or(DEFAULT_PASSWD_BUFFER_LEN),
        _ => DEFAULT_PASSWD_BUFFER_LEN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_helpers_match_exact_paths_and_whole_component_prefixes() {
        let root = Path::new("/workspace/app");
        assert!(paths_equal(root, Path::new("/workspace/app")));
        assert!(!paths_equal(root, Path::new("/workspace/app/src")));
        assert!(!paths_equal(root, Path::new("/workspace")));
        assert!(path_starts_with(Path::new("/workspace/app/bin/x"), root));
        assert!(path_starts_with(root, root));
        assert!(!path_starts_with(Path::new("/workspace/application"), root));
        assert!(!path_starts_with(Path::new("/workspace"), root));
    }

    #[cfg(windows)]
    #[test]
    fn path_helpers_ignore_ascii_case_on_windows() {
        let root = Path::new(r"C:\Users\Dev\App");
        assert!(paths_equal(root, Path::new(r"c:\users\dev\app")));
        assert!(paths_equal(root, Path::new("c:/USERS/dev/app")));
        assert!(path_starts_with(
            Path::new(r"c:\USERS\dev\app\bin\x.exe"),
            root
        ));
        assert!(!path_starts_with(Path::new(r"c:\users\dev\apps"), root));
    }

    #[cfg(not(windows))]
    #[test]
    fn path_helpers_are_case_sensitive_off_windows() {
        let root = Path::new("/home/dev/App");
        assert!(!paths_equal(root, Path::new("/home/dev/app")));
        assert!(!path_starts_with(Path::new("/home/dev/app/bin"), root));
    }
}
