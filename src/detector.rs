//! Cached high-level stack and project detection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config;
use crate::image::detect_from_image;
use crate::process::detect_from_process;
use crate::project::{has_marker, walk_ancestors};
use crate::{ProjectInput, StackInput, StackLabel};

/// Cache-owning detector for repeated stack and project lookups.
#[derive(Debug, Default)]
pub struct StackDetector {
    home: Option<PathBuf>,
    project_cache: HashMap<PathBuf, Option<PathBuf>>,
    config_cache: HashMap<PathBuf, Option<StackLabel>>,
}

impl StackDetector {
    /// Create a detector with an optional home-directory ceiling.
    #[must_use]
    pub fn new(home: Option<PathBuf>) -> Self {
        Self {
            home,
            project_cache: HashMap::new(),
            config_cache: HashMap::new(),
        }
    }

    /// Detect a project root from process-like path inputs.
    pub fn detect_project_root(&mut self, input: ProjectInput<'_>) -> Option<PathBuf> {
        let configured_home = self.home.clone();
        let home = input.home.or(configured_home.as_deref());

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
