//! Public input and output types shared by the detection APIs.

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::Path;

/// Human-readable application, framework, runtime, or service label.
///
/// Built-in detections return borrowed static labels. The `Cow` return type
/// leaves room for future runtime-generated labels without changing the API.
///
/// # Examples
///
/// ```
/// use what_stack::{StackLabel, detect_from_process};
///
/// let label: Option<StackLabel> = detect_from_process("postgres");
/// assert_eq!(label.as_deref(), Some("PostgreSQL"));
/// ```
pub type StackLabel = Cow<'static, str>;

/// Process-like path inputs used to resolve a project root.
///
/// Project root resolution is intentionally best-effort. The caller decides how
/// much process metadata is available, and the resolver applies a stable
/// fallback order:
///
/// 1. [`cwd`](Self::cwd)
/// 2. parent of [`exe`](Self::exe)
/// 3. parents of absolute paths in [`cmd`](Self::cmd)
///
/// Relative command-line arguments are ignored because they cannot be resolved
/// safely without knowing the process working directory at the time the command
/// started.
#[derive(Debug, Clone, Copy)]
pub struct ProjectInput<'a> {
    /// Process working directory, when available.
    ///
    /// This is the most trustworthy project-root hint and is checked first.
    pub cwd: Option<&'a Path>,

    /// Process executable path, when available.
    ///
    /// The parent directory is searched when [`cwd`](Self::cwd) is missing or
    /// does not resolve to a project.
    pub exe: Option<&'a Path>,

    /// Process command-line arguments.
    ///
    /// Only absolute path arguments are inspected, in their original order.
    pub cmd: &'a [OsString],

    /// Optional home directory ceiling for upward project walks.
    ///
    /// When provided, the upward walk stops before testing `home` itself. This
    /// avoids treating stray project markers in a user's home directory as the
    /// root for unrelated processes.
    pub home: Option<&'a Path>,
}

/// Inputs used by [`StackDetector`](crate::StackDetector) to resolve a stack.
///
/// The detector uses image, project config, and process metadata in that order.
/// Config detection is guarded to avoid false positives from helper processes:
/// a config label can win only when the process is a known runtime/tool or the
/// executable path starts with the project root.
#[derive(Debug, Clone, Copy)]
pub struct StackInput<'a> {
    /// Container or artifact image name, when available.
    ///
    /// Images are checked first because they usually identify services such as
    /// databases more precisely than process names.
    pub image: Option<&'a str>,

    /// Project root directory, when available.
    ///
    /// The directory itself is scanned; parent and child directories are not
    /// searched by stack detection. Use [`resolve_project_root`](crate::resolve_project_root)
    /// first when starting from process paths.
    pub project_root: Option<&'a Path>,

    /// Process executable name from the process table.
    ///
    /// Pass an empty string when no process name is available.
    pub process_name: &'a str,

    /// Full executable file name from the executable path, when available.
    ///
    /// This is used as a fallback when [`process_name`](Self::process_name) is
    /// truncated or less specific.
    pub exe_name: Option<&'a str>,

    /// Full executable path, when available.
    ///
    /// The path is not read. It is used only to decide whether config detection
    /// is allowed for an unknown process name.
    pub exe_path: Option<&'a Path>,
}
