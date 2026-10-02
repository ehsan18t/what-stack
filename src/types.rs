//! Public input and output types shared by the detection APIs.

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::path::Path;

/// Broad category of a detected [`StackLabel`].
///
/// The kind is assigned where each detection rule is defined, so every label
/// produced by this crate carries one. [`StackDetector`](crate::StackDetector)
/// uses it to decide whether project config may refine a process label:
///
/// - [`Runtime`](Self::Runtime) and [`Tool`](Self::Tool) labels are generic
///   hosts for project code, so a project config label (for example `Next.js`
///   for a `node` process) may replace them.
/// - [`Framework`](Self::Framework), [`Database`](Self::Database), and
///   [`Service`](Self::Service) labels are final: a `postgres` or `rails`
///   process keeps its own label even when started from a project directory.
///
/// The enum is `#[non_exhaustive]`; match it with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StackKind {
    /// Language runtime or process that executes project code, such as
    /// `Node.js`, `Python`, `.NET`, `Go`, or the `Gunicorn` and `Uvicorn`
    /// application servers.
    Runtime,
    /// Application or site framework, such as `Next.js`, `Django`, `Rails`,
    /// or `Hugo`.
    Framework,
    /// Build tool, bundler, or dev server, such as `Vite`, `Webpack`, or
    /// `Java (Maven)`.
    Tool,
    /// Database, cache, or search engine, such as `PostgreSQL`, `Redis`, or
    /// `Elasticsearch`.
    Database,
    /// Network service that is not a data store, such as a web server, reverse
    /// proxy, message broker, or cloud emulator (`Nginx`, `Traefik`,
    /// `RabbitMQ`, `LocalStack`).
    Service,
}

/// Human-readable application, framework, runtime, or service label.
///
/// A label pairs display text with a [`StackKind`]. Built-in labels borrow
/// static strings, so producing them never allocates. Use
/// [`as_str`](Self::as_str), [`AsRef<str>`], or [`Display`](fmt::Display) to
/// read the text, and [`kind`](Self::kind) for the category.
///
/// Equality and hashing consider both text and kind. Comparisons with `str`
/// and `&str` consider only the text, which keeps assertions short.
///
/// # Examples
///
/// ```
/// use what_stack::{StackKind, StackLabel, detect_from_process};
///
/// let label: StackLabel = detect_from_process("postgres").expect("known process");
/// assert_eq!(label, "PostgreSQL");
/// assert_eq!(label.as_str(), "PostgreSQL");
/// assert_eq!(label.kind(), StackKind::Database);
/// assert_eq!(label.to_string(), "PostgreSQL");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StackLabel {
    text: Cow<'static, str>,
    kind: StackKind,
}

impl StackLabel {
    /// Create a label from owned or static text.
    ///
    /// Prefer [`from_static`](Self::from_static) for string literals; it is a
    /// `const fn` and never allocates.
    #[must_use]
    pub fn new(text: impl Into<Cow<'static, str>>, kind: StackKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }

    /// Create a label from static text without allocating.
    #[must_use]
    pub const fn from_static(text: &'static str, kind: StackKind) -> Self {
        Self {
            text: Cow::Borrowed(text),
            kind,
        }
    }

    /// Return the label text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Return the label category.
    #[must_use]
    pub const fn kind(&self) -> StackKind {
        self.kind
    }

    /// Convert the label into its text, dropping the kind.
    ///
    /// Static labels stay borrowed, so this does not allocate for built-in
    /// detections.
    #[must_use]
    pub fn into_cow(self) -> Cow<'static, str> {
        self.text
    }
}

impl fmt::Display for StackLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl AsRef<str> for StackLabel {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl PartialEq<str> for StackLabel {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<&str> for StackLabel {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<StackLabel> for str {
    fn eq(&self, other: &StackLabel) -> bool {
        self == other.text
    }
}

impl PartialEq<StackLabel> for &str {
    fn eq(&self, other: &StackLabel) -> bool {
        *self == other.text
    }
}

impl From<StackLabel> for Cow<'static, str> {
    fn from(label: StackLabel) -> Self {
        label.text
    }
}

impl From<StackLabel> for String {
    fn from(label: StackLabel) -> Self {
        label.text.into_owned()
    }
}

/// Process-like path inputs used to resolve a project root.
///
/// Build one with [`ProjectInput::new`] and the chainable setters. Project root
/// resolution is best-effort and uses a stable fallback order:
///
/// 1. [`cwd`](Self::cwd)
/// 2. parent of [`exe`](Self::exe)
/// 3. parents of absolute paths in [`cmd`](Self::cmd)
///
/// Relative command-line arguments are ignored because they cannot be resolved
/// safely without knowing the process working directory at the time the command
/// started.
///
/// The home-directory ceiling is not part of the input. It is configured on
/// [`StackDetector`](crate::StackDetector) or passed to
/// [`resolve_project_root`](crate::resolve_project_root) directly.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use what_stack::ProjectInput;
///
/// let input = ProjectInput::new()
///     .cwd(Path::new("/workspace/api/src"))
///     .exe(None);
/// # let _ = input;
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ProjectInput<'a> {
    pub(crate) cwd: Option<&'a Path>,
    pub(crate) exe: Option<&'a Path>,
    pub(crate) cmd: &'a [OsString],
}

impl<'a> ProjectInput<'a> {
    /// Create an empty input with no path hints.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cwd: None,
            exe: None,
            cmd: &[],
        }
    }

    /// Set the process working directory.
    ///
    /// This is the most trustworthy project-root hint and is checked first.
    /// Accepts a `&Path` or an `Option<&Path>`.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<Option<&'a Path>>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Set the process executable path.
    ///
    /// The parent directory is searched when the working directory is missing
    /// or does not resolve to a project. Accepts a `&Path` or an
    /// `Option<&Path>`.
    #[must_use]
    pub fn exe(mut self, exe: impl Into<Option<&'a Path>>) -> Self {
        self.exe = exe.into();
        self
    }

    /// Set the process command-line arguments.
    ///
    /// Only absolute path arguments are inspected, in their original order.
    #[must_use]
    pub const fn cmd(mut self, cmd: &'a [OsString]) -> Self {
        self.cmd = cmd;
        self
    }
}

/// Inputs used by [`StackDetector`](crate::StackDetector) to resolve a stack.
///
/// Build one with [`StackInput::new`] and the chainable setters. The detector
/// uses image, process, and project config metadata. Config detection is
/// guarded to avoid false positives: a config label can win only when the
/// process label is a [`StackKind::Runtime`] or [`StackKind::Tool`], or when the
/// process is unknown but its executable path lies inside the project root. A
/// known runtime or tool accepts only config labels from its own ecosystem.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use what_stack::StackInput;
///
/// let input = StackInput::new("node")
///     .exe_name("node.exe")
///     .project_root(Path::new("/workspace/web"))
///     .image(None);
/// # let _ = input;
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct StackInput<'a> {
    pub(crate) image: Option<&'a str>,
    pub(crate) project_root: Option<&'a Path>,
    pub(crate) process_name: &'a str,
    pub(crate) exe_name: Option<&'a str>,
    pub(crate) exe_path: Option<&'a Path>,
}

impl<'a> StackInput<'a> {
    /// Create an input from a process name taken from the process table.
    ///
    /// Pass an empty string when no process name is available.
    #[must_use]
    pub const fn new(process_name: &'a str) -> Self {
        Self {
            image: None,
            project_root: None,
            process_name,
            exe_name: None,
            exe_path: None,
        }
    }

    /// Set the container or artifact image name.
    ///
    /// Images are checked first because they usually identify services such as
    /// databases more precisely than process names. Accepts a `&str` or an
    /// `Option<&str>`.
    #[must_use]
    pub fn image(mut self, image: impl Into<Option<&'a str>>) -> Self {
        self.image = image.into();
        self
    }

    /// Set the project root directory.
    ///
    /// The directory itself is scanned; parent and child directories are not
    /// searched by stack detection. Use
    /// [`StackDetector::detect_project_root`](crate::StackDetector::detect_project_root)
    /// first when starting from process paths. Accepts a `&Path` or an
    /// `Option<&Path>`.
    #[must_use]
    pub fn project_root(mut self, project_root: impl Into<Option<&'a Path>>) -> Self {
        self.project_root = project_root.into();
        self
    }

    /// Set the executable file name taken from the executable path.
    ///
    /// This is used as a fallback when the process name is truncated or less
    /// specific. Accepts a `&str` or an `Option<&str>`.
    #[must_use]
    pub fn exe_name(mut self, exe_name: impl Into<Option<&'a str>>) -> Self {
        self.exe_name = exe_name.into();
        self
    }

    /// Set the full executable path.
    ///
    /// The path is not read. It is used only to decide whether config detection
    /// is allowed for an unknown process name. Accepts a `&Path` or an
    /// `Option<&Path>`.
    #[must_use]
    pub fn exe_path(mut self, exe_path: impl Into<Option<&'a Path>>) -> Self {
        self.exe_path = exe_path.into();
        self
    }
}
