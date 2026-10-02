//! Cached high-level stack and project detection.
//!
//! [`StackDetector`] is the API to use when enriching many process entries from
//! the same scan. It caches project-root walks and config-file results so
//! repeated processes in the same project do not repeat filesystem work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config;
use crate::ecosystem::Ecosystem;
use crate::image::detect_from_image;
use crate::process::find_process_rule_by_names;
use crate::project::{Walk, has_marker, path_starts_with, project_root_candidates};
use crate::{ProjectInput, StackInput, StackKind, StackLabel};

/// Cache-owning detector for repeated stack and project lookups.
///
/// A detector is intentionally stateful. Cache entries are retained until the
/// detector is dropped or [`clear`](Self::clear) is called, so it is best used
/// for one coherent scan of process or project metadata. Call `clear` between
/// scans when filesystem changes should be observed.
///
/// # Home Ceiling
///
/// Upward project walks stop before testing the detector's home directory, so
/// stray marker files directly in a user's home do not claim unrelated
/// processes. [`new`](Self::new) and [`Default`] use [`crate::home_dir`];
/// [`with_home`](Self::with_home) overrides it.
///
/// # Stack Priority
///
/// [`detect_stack`](Self::detect_stack) resolves a label in this order:
///
/// 1. Image name via [`crate::detect_from_image`].
/// 2. Process or executable name via [`crate::detect_from_process_names`],
///    when that label is final: [`StackKind::Framework`],
///    [`StackKind::Database`], [`StackKind::Service`], or any future kind.
/// 3. Project config, when the process label is a [`StackKind::Runtime`] or
///    [`StackKind::Tool`], or when the process is unknown but its executable
///    path lies inside the project root.
/// 4. The process label, if any.
///
/// Config detection is ecosystem-aware. A known runtime or tool accepts only
/// config labels from its own ecosystem: a `php` or `php-fpm` process in a
/// Laravel project that also has `vite.config.js` is `Laravel`, a `node` or
/// `vite` process there is `Vite`, and a `python` process in a Next.js project
/// stays `Python`. Deno config has its own ecosystem, so a `node` or `bun`
/// process next to `deno.json` keeps its label, while a `deno` process also
/// accepts Node config. A Python process takes only framework labels from
/// config, so `gunicorn` in a Python project with no recognized framework
/// stays `Gunicorn`. An unknown process uses every rule, in the order of
/// [`crate::detect_from_config`].
///
/// # Examples
///
/// ```
/// use what_stack::{StackDetector, StackInput};
///
/// let mut detector = StackDetector::new();
/// let label = detector.detect_stack(StackInput::new("postgres").image("postgres:16"));
///
/// assert_eq!(label.expect("known image"), "PostgreSQL");
/// ```
#[derive(Debug)]
pub struct StackDetector {
    home: Option<PathBuf>,
    project_cache: HashMap<PathBuf, Option<PathBuf>>,
    /// Config results per project root, one entry per process ecosystem seen
    /// (`None` for unknown processes).
    config_cache: HashMap<PathBuf, Vec<ConfigCacheEntry>>,
}

type ConfigCacheEntry = (Option<Ecosystem>, Option<StackLabel>);

impl Default for StackDetector {
    /// Same as [`StackDetector::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl StackDetector {
    /// Create a detector whose home ceiling is the current user's home
    /// directory, as returned by [`crate::home_dir`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_home(crate::home_dir())
    }

    /// Create a detector with an explicit home ceiling.
    ///
    /// `None` disables the ceiling, so upward walks may reach the file system
    /// root (bounded by [`crate::MAX_WALK_DEPTH`]).
    #[must_use]
    pub fn with_home(home: Option<PathBuf>) -> Self {
        Self {
            home,
            project_cache: HashMap::new(),
            config_cache: HashMap::new(),
        }
    }

    /// Return the configured home ceiling.
    #[must_use]
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    /// Drop all cached project-root and config results.
    ///
    /// The home ceiling is kept. Call this between scans when one detector is
    /// reused and filesystem changes should be observed.
    pub fn clear(&mut self) {
        self.project_cache.clear();
        self.config_cache.clear();
    }

    /// Detect a project root from process-like path inputs.
    ///
    /// Uses the same fallback order as [`crate::resolve_project_root`] with the
    /// detector's home ceiling. Results are cached by visited directory.
    /// Positive hits cache the visited directories from the start up to the
    /// discovered root; negative walks cache the visited directories as misses,
    /// except when the walk stopped at [`crate::MAX_WALK_DEPTH`], because a
    /// walk from a shallower visited directory can reach further up.
    /// This mirrors the process-enrichment hot path where many entries share a
    /// working directory or project ancestor.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use what_stack::{ProjectInput, StackDetector};
    ///
    /// let mut detector = StackDetector::new();
    /// let root = detector.detect_project_root(ProjectInput::new().cwd(Path::new(".")));
    /// println!("{root:?}");
    /// ```
    #[must_use]
    pub fn detect_project_root(&mut self, input: ProjectInput<'_>) -> Option<PathBuf> {
        project_root_candidates(input).find_map(|start| self.cached_project_root(start))
    }

    /// Detect a stack label from image, process, and project metadata.
    ///
    /// Image and process matching are pure string operations. Config matching
    /// reads the given project-root directory on the first lookup and caches
    /// the result for future calls with the same path. See the type-level
    /// documentation for the priority and config guard.
    #[must_use]
    pub fn detect_stack(&mut self, input: StackInput<'_>) -> Option<StackLabel> {
        if let Some(image) = input.image
            && let Some(label) = detect_from_image(image)
        {
            return Some(label);
        }

        let process_rule = find_process_rule_by_names(input.process_name, input.exe_name);
        let process_stack = process_rule.map(|(_, label, _)| label);

        if let Some(project_root) = input.project_root
            && config_detection_allowed(process_stack, input.exe_path, project_root)
            && let Some(label) =
                self.cached_config_stack(project_root, process_rule.map(|(_, _, eco)| *eco))
        {
            return Some(label);
        }

        process_stack.cloned()
    }

    fn cached_project_root(&mut self, start: &Path) -> Option<PathBuf> {
        let mut visited = Vec::new();
        let mut walk = Walk::new(start, self.home.as_deref());
        let result = walk
            .by_ref()
            .find_map(|dir| {
                if let Some(cached) = self.project_cache.get(dir) {
                    return Some(cached.clone());
                }
                visited.push(dir);
                has_marker(dir).then(|| Some(dir.to_path_buf()))
            })
            .flatten();

        // A miss caused by the depth cap is only a miss for the deepest start:
        // a walk from a shallower visited directory can reach further up.
        if result.is_none() && walk.hit_depth_cap() {
            return None;
        }

        for path in visited {
            self.project_cache
                .insert(path.to_path_buf(), result.clone());
        }

        result
    }

    fn cached_config_stack(
        &mut self,
        project_root: &Path,
        ecosystem: Option<Ecosystem>,
    ) -> Option<StackLabel> {
        if let Some((_, cached)) = self
            .config_cache
            .get(project_root)
            .and_then(|entries| entries.iter().find(|(seen, _)| *seen == ecosystem))
        {
            return cached.clone();
        }

        let result = config::detect_for_ecosystem(project_root, ecosystem);
        self.config_cache
            .entry(project_root.to_path_buf())
            .or_default()
            .push((ecosystem, result.clone()));
        result
    }
}

/// Whether a project config label may replace (or supply) the process label.
fn config_detection_allowed(
    process_stack: Option<&StackLabel>,
    exe_path: Option<&Path>,
    project_root: &Path,
) -> bool {
    process_stack.map_or_else(
        || exe_path.is_some_and(|path| path_starts_with(path, project_root)),
        |label| accepts_config_override(label.kind()),
    )
}

/// Runtime and tool labels are generic hosts for project code; every other
/// kind, including kinds added later, is final.
const fn accepts_config_override(kind: StackKind) -> bool {
    matches!(kind, StackKind::Runtime | StackKind::Tool)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn write_marker(dir: &Path, name: &str) {
        fs::create_dir_all(dir).expect("create marker directory");
        fs::write(dir.join(name), "").expect("write marker");
    }

    fn assert_cached_root(detector: &StackDetector, path: &Path, expected: &Path, message: &str) {
        assert_eq!(
            detector.project_cache.get(path).and_then(Option::as_deref),
            Some(expected),
            "{message}"
        );
    }

    #[test]
    fn project_root_cache_learns_visited_ancestors() {
        let root = TempDir::new().expect("temp dir");
        write_marker(root.path(), "Cargo.toml");

        let first = root.path().join("src").join("db");
        let second = root.path().join("src").join("utils");
        fs::create_dir_all(&first).expect("create first dir");
        fs::create_dir_all(&second).expect("create second dir");

        let mut detector = StackDetector::new();

        let first_result = detector.detect_project_root(ProjectInput::new().cwd(first.as_path()));
        assert_eq!(first_result.as_deref(), Some(root.path()));
        assert_cached_root(
            &detector,
            &first,
            root.path(),
            "the original cwd should be cached",
        );
        assert_cached_root(
            &detector,
            first.parent().expect("first has parent"),
            root.path(),
            "visited ancestors should also be cached",
        );

        let second_result = detector.detect_project_root(ProjectInput::new().cwd(second.as_path()));
        assert_eq!(second_result.as_deref(), Some(root.path()));
        assert_cached_root(
            &detector,
            &second,
            root.path(),
            "sibling directories should learn from the cached ancestor",
        );
    }

    #[test]
    fn project_root_cache_does_not_poison_unrelated_ancestors() {
        let workspace = TempDir::new().expect("temp dir");
        let outer = workspace.path().join("workspace");
        let project_root = outer.join("app");
        let inside = project_root.join("src").join("db");
        let unrelated = outer.join("services").join("worker");

        fs::create_dir_all(&inside).expect("create inside dir");
        fs::create_dir_all(&unrelated).expect("create unrelated dir");
        write_marker(&project_root, "Cargo.toml");

        let mut detector = StackDetector::new();

        let first_result = detector.detect_project_root(ProjectInput::new().cwd(inside.as_path()));
        assert_eq!(first_result.as_deref(), Some(project_root.as_path()));
        assert!(
            !detector.project_cache.contains_key(outer.as_path()),
            "ancestors above the discovered project root must not be cached as project hits"
        );

        let unrelated_result =
            detector.detect_project_root(ProjectInput::new().cwd(unrelated.as_path()));
        assert!(
            unrelated_result.is_none(),
            "an unrelated path under the same ancestor must not inherit another project's root"
        );
    }

    #[test]
    fn clear_drops_cached_results_but_keeps_home() {
        let project = TempDir::new().expect("temp dir");
        write_marker(project.path(), "Cargo.toml");
        let home = PathBuf::from("/not/a/real/home");

        let mut detector = StackDetector::with_home(Some(home.clone()));
        let root = detector.detect_project_root(ProjectInput::new().cwd(project.path()));
        assert_eq!(root.as_deref(), Some(project.path()));
        let stack = detector.detect_stack(StackInput::new("cargo").project_root(project.path()));
        assert_eq!(stack.expect("rust project"), "Rust");
        assert!(!detector.project_cache.is_empty());
        assert!(!detector.config_cache.is_empty());

        detector.clear();

        assert!(detector.project_cache.is_empty());
        assert!(detector.config_cache.is_empty());
        assert_eq!(detector.home(), Some(home.as_path()));
    }

    #[test]
    fn only_runtime_and_tool_kinds_accept_config_override() {
        assert!(accepts_config_override(StackKind::Runtime));
        assert!(accepts_config_override(StackKind::Tool));
        assert!(!accepts_config_override(StackKind::Framework));
        assert!(!accepts_config_override(StackKind::Database));
        assert!(!accepts_config_override(StackKind::Service));
    }
}
