use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug)]
pub(super) struct ProjectFiles {
    names: HashSet<String>,
}

impl ProjectFiles {
    pub(super) fn read(project_root: &Path) -> Option<Self> {
        let entries = std::fs::read_dir(project_root).ok()?;
        let names = entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();

        Some(Self { names })
    }

    pub(super) fn contains_exact(&self, target: &str) -> bool {
        self.names.contains(target)
    }

    pub(super) fn contains_prefix(&self, prefix: &str) -> bool {
        self.names
            .iter()
            .any(|name| super::rules::matches_config_name_prefix(name, prefix))
    }

    pub(super) fn any_exact(&self, targets: &[&str]) -> bool {
        targets.iter().any(|target| self.contains_exact(target))
    }

    pub(super) fn contains_extension(&self, target_extension: &str) -> bool {
        self.names.iter().any(|name| {
            Path::new(name)
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension == target_extension)
        })
    }

    pub(super) fn read_text(&self, project_root: &Path, file_name: &str) -> Option<String> {
        use std::io::Read;

        const MAX_SCAN_BYTES: u64 = 64 * 1024;

        if !self.contains_exact(file_name) {
            return None;
        }

        let file = std::fs::File::open(project_root.join(file_name)).ok()?;
        let mut buffer = String::new();
        file.take(MAX_SCAN_BYTES).read_to_string(&mut buffer).ok()?;
        Some(buffer)
    }
}
