//! Public input and output types.

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::Path;

/// Human-readable application, framework, runtime, or service label.
///
/// Built-in detections return borrowed static labels. The `Cow` return type
/// leaves room for future runtime-generated labels without changing the API.
pub type StackLabel = Cow<'static, str>;

/// Inputs used to resolve a project root.
#[derive(Debug, Clone, Copy)]
pub struct ProjectInput<'a> {
    /// Process working directory, when available.
    pub cwd: Option<&'a Path>,
    /// Process executable path, when available.
    pub exe: Option<&'a Path>,
    /// Process command-line arguments.
    pub cmd: &'a [OsString],
    /// Optional home directory ceiling for upward project walks.
    pub home: Option<&'a Path>,
}

/// Inputs used to resolve a technology stack label.
#[derive(Debug, Clone, Copy)]
pub struct StackInput<'a> {
    /// Container or artifact image name, when available.
    pub image: Option<&'a str>,
    /// Project root directory, when available.
    pub project_root: Option<&'a Path>,
    /// Process executable name.
    pub process_name: &'a str,
    /// Full executable file name from the executable path, when available.
    pub exe_name: Option<&'a str>,
    /// Full executable path, when available.
    pub exe_path: Option<&'a Path>,
}
