//! Project-root detection.
//!
//! Project detection walks upward from process paths and looks for marker
//! files such as `package.json`, `Cargo.toml`, `go.mod`, `pyproject.toml`, and
//! project-file extensions such as `.csproj`, `.fsproj`, `.sln`, and `.slnx`.

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};

#[cfg(unix)]
use std::ffi::CStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use crate::process::find_process_rule_by_names;
use crate::text::file_extension;
use crate::{ProjectInput, StackKind};

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
    "settings.gradle",
    "settings.gradle.kts",
    "composer.json",
    "Gemfile",
    "mix.exs",
    "deno.json",
    "deno.jsonc",
    "bun.lockb",
    "bun.lock",
];

const PROJECT_MARKER_EXTENSIONS: &[&str] = &["csproj", "fsproj", "sln", "slnx"];

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
///    [`ProjectInput::exe`], unless the executable is a known runtime or tool
///    such as `node`, `python`, or `cargo`. A known runtime inside a Python
///    virtual environment (a directory with `pyvenv.cfg` one or two levels
///    above the executable, as in `app/.venv/bin/python` or
///    `app\.venv\Scripts\python.exe`) walks from the directory holding the
///    environment instead.
/// 3. Walk upward from the parent directory of each absolute path in the
///    arguments set by [`ProjectInput::cmd`].
///
/// The first accepted marker hit wins. Relative command-line paths are ignored
/// because they are ambiguous without a reliable process working directory.
/// `home` is the same optional ceiling as in [`find_project_root`]. A root
/// found from the executable path is rejected when it lies inside a dot
/// directory directly under `home` (`~/.nvm`, `~/.cargo`, `~/.local`): those
/// hold installed toolchains and packages, not the process's project.
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
    project_root_candidates(input).find_map(|(start, from_exe)| {
        find_project_root(start, home).filter(|root| accepts_root(root, from_exe, home))
    })
}

/// Starting directories for a project walk, in fallback order, each with
/// whether it came from the executable path.
pub fn project_root_candidates(
    input: ProjectInput<'_>,
) -> impl Iterator<Item = (&Path, bool)> + '_ {
    input
        .cwd
        .into_iter()
        .map(|cwd| (cwd, false))
        .chain(exe_walk_start(input.exe).map(|start| (start, true)))
        .chain(absolute_cmd_parents(input.cmd).map(|start| (start, false)))
}

/// The executable parent to walk from, or `None` when the executable is a
/// known runtime or tool. An installed `node` or `python` says nothing about
/// the project it runs, and walking up from its directory finds the install
/// tree instead (`~/.nvm/versions/node/v20/bin/node`).
///
/// A known runtime inside a Python virtual environment is the exception: the
/// environment usually belongs to the project around it (`app/.venv/bin/python`
/// or `app\.venv\Scripts\python.exe`), so the walk starts at the environment
/// directory itself. A separate `.venv` holds no project markers, so the walk
/// reaches `app`; an environment created in place (`python -m venv .`) is the
/// project directory and is found directly. Environments under a home dot directory
/// (`~/.virtualenvs`, `~/.pyenv`) still yield no root, because
/// [`accepts_root`] rejects roots found from the executable there.
fn exe_walk_start(exe: Option<&Path>) -> Option<&Path> {
    let exe = exe?;
    let known_host = exe
        .file_name()
        .and_then(OsStr::to_str)
        .and_then(|name| find_process_rule_by_names(name, None))
        .is_some_and(|(_, label, _)| matches!(label.kind(), StackKind::Runtime | StackKind::Tool));

    if known_host {
        virtual_env_dir(exe)
    } else {
        exe.parent()
    }
}

/// The Python virtual environment holding `exe`: its parent (`Scripts` on
/// Windows, `bin` elsewhere) or grandparent when that directory contains
/// `pyvenv.cfg`, which `venv`, `virtualenv`, and `uv` all write. Conda
/// environments have no `pyvenv.cfg` and are not matched.
fn virtual_env_dir(exe: &Path) -> Option<&Path> {
    exe.ancestors()
        .skip(1)
        .take(2)
        .find(|dir| dir.join("pyvenv.cfg").is_file())
}

/// Whether a root found from a walk start may be used. Roots found from the
/// executable path must not lie inside a dot directory directly under `home`.
pub fn accepts_root(root: &Path, from_exe: bool, home: Option<&Path>) -> bool {
    !from_exe || home.is_none_or(|home| !is_inside_home_dot_dir(root, home))
}

/// Whether `path` is at or below `home/.name` for some dot directory `.name`.
fn is_inside_home_dot_dir(path: &Path, home: &Path) -> bool {
    path_starts_with(path, home)
        && path
            .components()
            .nth(home.components().count())
            .is_some_and(|component| {
                matches!(component, Component::Normal(name) if name.as_encoded_bytes().starts_with(b"."))
            })
}

/// Walk upward from `start` looking for project marker files.
///
/// `home` is an optional ceiling. When the walk reaches `home`, it stops before
/// testing that directory for markers. This avoids accidental matches from
/// marker files stored directly in a user's home directory. On Windows the
/// ceiling comparison ignores ASCII case, matching the file system.
///
/// At most [`MAX_WALK_DEPTH`] directories are tested, starting with `start`.
/// A relative `start` is walked lexically and ends at the current directory,
/// so `src` tests `src` and then `.`, and a hit there is returned as `.`.
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
    Walk::new(start, home)
        .find(|dir| has_marker(dir))
        .map(Path::to_path_buf)
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

/// Upward walk over the directories tested for project markers, nearest
/// first.
///
/// The walk stops before `home` and after [`MAX_WALK_DEPTH`] directories. It
/// is lexical, so the parent of a relative single-name start such as `src` is
/// the empty path; that stands for the current directory and is tested as `.`.
///
/// After the walk returns `None`, [`hit_depth_cap`](Self::hit_depth_cap) tells
/// whether it ended because of the depth cap while untested ancestors
/// remained, rather than at the file system root or the home ceiling.
#[derive(Debug)]
pub struct Walk<'a> {
    ancestors: Option<std::path::Ancestors<'a>>,
    home: Option<&'a Path>,
    empty_means_current: bool,
    remaining: usize,
    hit_depth_cap: bool,
}

impl<'a> Walk<'a> {
    pub fn new(start: &'a Path, home: Option<&'a Path>) -> Self {
        Self {
            ancestors: Some(start.ancestors()),
            home,
            empty_means_current: matches!(start.components().next(), Some(Component::Normal(_))),
            remaining: MAX_WALK_DEPTH,
            hit_depth_cap: false,
        }
    }

    /// Whether the walk stopped at [`MAX_WALK_DEPTH`] with ancestors left to
    /// test. A walk that stopped there is not a complete answer for the
    /// directories it visited: a walk starting at one of them could reach
    /// further up.
    pub const fn hit_depth_cap(&self) -> bool {
        self.hit_depth_cap
    }

    fn next_ancestor(&mut self) -> Option<&'a Path> {
        let ancestors = self.ancestors.as_mut()?;
        let dir = ancestors.next()?;

        if !dir.as_os_str().is_empty() {
            Some(dir)
        } else if self.empty_means_current {
            Some(Path::new("."))
        } else {
            None
        }
    }
}

impl<'a> Iterator for Walk<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<&'a Path> {
        let dir = self.next_ancestor();
        let stop = match dir {
            None => true,
            Some(dir) if self.home.is_some_and(|home| paths_equal(dir, home)) => true,
            Some(_) if self.remaining == 0 => {
                self.hit_depth_cap = true;
                true
            }
            Some(_) => false,
        };

        if stop {
            self.ancestors = None;
            return None;
        }

        self.remaining -= 1;
        dir
    }
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
            || file_extension(name)
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

    #[cfg(unix)]
    #[test]
    fn preferred_home_uid_from_env_prefers_sudo_uid_for_root_sessions() {
        assert_eq!(
            preferred_home_uid_from_env(Some(OsStr::new("1000")), 0),
            Some(1000),
            "sudo sessions should prefer the invoking user's uid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn preferred_home_uid_from_env_ignores_sudo_uid_for_non_root_sessions() {
        assert_eq!(
            preferred_home_uid_from_env(Some(OsStr::new("1000")), 2000),
            Some(2000),
            "non-root sessions should keep the current effective uid"
        );
    }

    #[cfg(unix)]
    #[test]
    fn preferred_home_uid_from_env_falls_back_when_sudo_uid_is_invalid() {
        assert_eq!(
            preferred_home_uid_from_env(Some(OsStr::new("not-a-uid")), 0),
            Some(0),
            "invalid sudo metadata should not break home-directory lookup"
        );
    }

    #[cfg(unix)]
    #[test]
    fn select_home_dir_prefers_passwd_lookup_over_sudo_home() {
        let passwd_home = Some(PathBuf::from("/home/invoking-user"));
        let sudo_home = Some(PathBuf::from("/root"));
        let env_home = Some(PathBuf::from("/tmp/fallback"));

        assert_eq!(
            select_home_dir(passwd_home.clone(), sudo_home, env_home),
            passwd_home,
            "passwd-database resolution should win over environment-derived sudo home"
        );
    }

    #[cfg(unix)]
    #[test]
    fn select_home_dir_falls_back_to_sudo_home_before_home_env() {
        let sudo_home = Some(PathBuf::from("/home/invoking-user"));
        let env_home = Some(PathBuf::from("/root"));

        assert_eq!(
            select_home_dir(None, sudo_home.clone(), env_home),
            sudo_home,
            "sudo home should remain the fallback when passwd lookup is unavailable"
        );
    }

    fn walk(start: &str) -> Vec<&Path> {
        Walk::new(Path::new(start), None).collect()
    }

    #[test]
    fn relative_walks_end_at_the_current_directory() {
        assert_eq!(walk("src"), [Path::new("src"), Path::new(".")]);
        assert_eq!(
            walk("src/bin"),
            [Path::new("src/bin"), Path::new("src"), Path::new(".")]
        );
        assert_eq!(walk("."), [Path::new(".")]);
        assert_eq!(walk("./src"), [Path::new("./src"), Path::new(".")]);
        assert_eq!(walk("../x"), [Path::new("../x"), Path::new("..")]);
        assert_eq!(walk(""), Vec::<&Path>::new());
    }

    #[test]
    fn relative_start_finds_a_marker_in_the_current_directory() {
        // Tests run with the package root, which holds Cargo.toml, as the
        // working directory.
        assert_eq!(
            find_project_root(Path::new("src"), None).as_deref(),
            Some(Path::new("."))
        );
    }

    #[test]
    fn walk_reports_whether_the_depth_cap_ended_it() {
        let deep: PathBuf = (0..=MAX_WALK_DEPTH)
            .map(|index| format!("d{index}"))
            .collect();
        let mut capped = Walk::new(&deep, None);
        assert_eq!(capped.by_ref().count(), MAX_WALK_DEPTH);
        assert!(capped.hit_depth_cap());
        assert_eq!(capped.next(), None, "a finished walk stays finished");

        let exact: PathBuf = (1..MAX_WALK_DEPTH)
            .map(|index| format!("d{index}"))
            .collect();
        let mut complete = Walk::new(&exact, None);
        assert_eq!(complete.by_ref().count(), MAX_WALK_DEPTH, "the last is `.`");
        assert!(!complete.hit_depth_cap());

        let home = Path::new("/home/dev");
        let mut stopped = Walk::new(Path::new("/home/dev/app/src"), Some(home));
        assert_eq!(stopped.by_ref().count(), 2);
        assert!(!stopped.hit_depth_cap());
    }
}
