//! Cached high-level stack and project detection.
//!
//! [`StackDetector`] is the API to use when enriching many process entries from
//! the same scan. It caches project-root walks and config-file results so
//! repeated processes in the same project do not repeat filesystem work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config;
use crate::image::detect_from_image;
use crate::process::detect_from_process;
use crate::project::{has_marker, walk_ancestors};
use crate::{ProjectInput, StackInput, StackLabel};

/// Cache-owning detector for repeated stack and project lookups.
///
/// A detector is intentionally stateful. Cache entries are retained until the
/// detector is dropped, so it is best used for one coherent scan of process or
/// project metadata. Create a fresh detector for a later scan when filesystem
/// changes should be observed.
///
/// # Stack Priority
///
/// [`detect_stack`](Self::detect_stack) returns the first label found in this
/// order:
///
/// 1. Image name via [`crate::detect_from_image`].
/// 2. Project config via [`crate::detect_from_config`].
/// 3. Process or executable name via [`crate::detect_from_process`].
///
/// Config detection is guarded. If a process name is unknown, config labels are
/// accepted only when the executable path belongs to the project root.
///
/// # Examples
///
/// ```
/// use what_stack::{StackDetector, StackInput};
///
/// let mut detector = StackDetector::new(None);
/// let label = detector.detect_stack(StackInput {
///     image: Some("postgres:16"),
///     project_root: None,
///     process_name: "postgres",
///     exe_name: None,
///     exe_path: None,
/// });
///
/// assert_eq!(label.as_deref(), Some("PostgreSQL"));
/// ```
#[derive(Debug, Default)]
pub struct StackDetector {
    home: Option<PathBuf>,
    project_cache: HashMap<PathBuf, Option<PathBuf>>,
    config_cache: HashMap<PathBuf, Option<StackLabel>>,
}

impl StackDetector {
    /// Create a detector with an optional home-directory ceiling.
    ///
    /// The configured home is used by [`detect_project_root`](Self::detect_project_root)
    /// when [`ProjectInput::home`](crate::ProjectInput::home) is `None`.
    /// Passing a home ceiling prevents upward walks from labeling unrelated
    /// processes with markers found directly in a user's home directory.
    #[must_use]
    pub fn new(home: Option<PathBuf>) -> Self {
        Self {
            home,
            project_cache: HashMap::new(),
            config_cache: HashMap::new(),
        }
    }

    /// Detect a project root from process-like path inputs.
    ///
    /// Results are cached by visited directory. Positive hits cache the visited
    /// descendants below the discovered root; negative walks cache the visited
    /// directories as misses. This mirrors the expected process-enrichment hot
    /// path where many entries share a working directory or project ancestor.
    #[must_use]
    pub fn detect_project_root(&mut self, input: ProjectInput<'_>) -> Option<PathBuf> {
        if input.home.is_some() {
            return self.detect_project_root_with_home(input, input.home);
        }

        let configured_home = self.home.clone();
        self.detect_project_root_with_home(input, configured_home.as_deref())
    }

    fn detect_project_root_with_home(
        &mut self,
        input: ProjectInput<'_>,
        home: Option<&Path>,
    ) -> Option<PathBuf> {
        if let Some(cwd) = input.cwd
            && let Some(root) = self.cached_project_root(cwd, home)
        {
            return Some(root);
        }

        if let Some(exe_parent) = input.exe.and_then(Path::parent)
            && let Some(root) = self.cached_project_root(exe_parent, home)
        {
            return Some(root);
        }

        crate::project::absolute_cmd_parents(input.cmd)
            .find_map(|parent| self.cached_project_root(parent, home))
    }

    /// Detect a stack label from image, project, and process metadata.
    ///
    /// Image and process matching are pure string operations. Config matching
    /// reads the given project-root directory on the first lookup and caches
    /// the result for future calls with the same path.
    #[must_use]
    pub fn detect_stack(&mut self, input: StackInput<'_>) -> Option<StackLabel> {
        if let Some(image) = input.image
            && let Some(label) = detect_from_image(image)
        {
            return Some(label);
        }

        let process_stack = detect_process_stack(input.process_name, input.exe_name);

        if let Some(project_root) = input.project_root
            && config_detection_allowed(process_stack.as_deref(), input.exe_path, project_root)
            && let Some(label) = self.cached_config_stack(project_root)
        {
            return Some(label);
        }

        process_stack
    }

    fn cached_project_root(&mut self, start: &Path, home: Option<&Path>) -> Option<PathBuf> {
        let mut visited = Vec::new();

        for dir in walk_ancestors(start, home) {
            if let Some(cached) = self.project_cache.get(&dir).cloned() {
                for path in visited {
                    self.project_cache.insert(path, cached.clone());
                }
                return cached;
            }

            visited.push(dir.clone());

            if has_marker(&dir) {
                let result = Some(dir);
                for path in visited {
                    self.project_cache.insert(path, result.clone());
                }
                return result;
            }
        }

        for path in visited {
            self.project_cache.insert(path, None);
        }

        None
    }

    fn cached_config_stack(&mut self, project_root: &Path) -> Option<StackLabel> {
        if let Some(cached) = self.config_cache.get(project_root) {
            return cached.clone();
        }

        let result = config::detect_from_config(project_root);
        self.config_cache
            .insert(project_root.to_path_buf(), result.clone());
        result
    }
}

fn detect_process_stack(process_name: &str, exe_name: Option<&str>) -> Option<StackLabel> {
    detect_from_process(process_name).or_else(|| exe_name.and_then(detect_from_process))
}

fn config_detection_allowed(
    process_stack: Option<&str>,
    exe_path: Option<&Path>,
    project_root: &Path,
) -> bool {
    process_stack.is_some() || exe_path.is_some_and(|path| path.starts_with(project_root))
}
