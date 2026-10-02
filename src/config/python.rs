use std::cell::OnceCell;
use std::path::Path;

use super::files::ProjectFiles;
use crate::{StackLabel, labels};

const PYTHON_ENTRY_FILES: &[&str] = &["app.py", "main.py", "server.py", "wsgi.py", "asgi.py"];

/// Files that declare a project's direct dependencies.
const PYTHON_MANIFEST_FILES: &[&str] = &[
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "Pipfile",
    "setup.py",
];

/// Lock files. They mark a Python project, but they also list transitive
/// dependencies (`starlette` through `mcp` or `fastapi`), so they only confirm
/// frameworks a manifest names and never add one.
const PYTHON_LOCK_FILES: &[&str] = &["poetry.lock", "uv.lock"];

const DJANGO_SOURCE_PATTERNS: &[&str] = &[
    "django.core.wsgi",
    "django.core.asgi",
    "django_settings_module",
    "get_wsgi_application",
    "get_asgi_application",
];

type PythonSourcePattern = (StackLabel, &'static [&'static str], &'static str);

pub const PYTHON_SOURCE_PATTERNS: &[PythonSourcePattern] = &[
    (
        labels::FASTAPI,
        &["from fastapi import", "import fastapi"],
        "fastapi(",
    ),
    (
        labels::STARLETTE,
        &["from starlette.applications import", "import starlette"],
        "starlette(",
    ),
    (
        labels::LITESTAR,
        &["from litestar import", "import litestar"],
        "litestar(",
    ),
    (
        labels::FLASK,
        &["from flask import", "import flask"],
        "flask(",
    ),
];

pub const PYTHON_DEPENDENCY_PATTERNS: &[(&str, StackLabel)] = &[
    ("django", labels::DJANGO),
    ("flask", labels::FLASK),
    ("fastapi", labels::FASTAPI),
    ("starlette", labels::STARLETTE),
    ("litestar", labels::LITESTAR),
];

/// Detect a Python project and its framework.
///
/// `python_process` is true when the process is a known Python runtime or
/// application server. Such a process gets only framework labels: the generic
/// `Python` fallback adds nothing to it and would replace a more specific
/// `Gunicorn` or `Uvicorn` label. For other callers, Python entry files alone
/// (`server.py` with no dependency file) do not count when `package.json` is
/// present: a stray script in a Node project should not label the project
/// `Python`.
pub(super) fn detect_python_project(
    project_root: &Path,
    files: &ProjectFiles,
    python_process: bool,
) -> Option<StackLabel> {
    if !is_python_project(files, python_process) {
        return None;
    }

    if files.contains_exact("manage.py") {
        return Some(labels::DJANGO);
    }

    detect_in_files(
        project_root,
        files,
        PYTHON_ENTRY_FILES,
        detect_python_framework_from_source,
    )
    .or_else(|| detect_python_framework_from_dependencies(project_root, files))
    .or_else(|| (!python_process).then_some(labels::PYTHON))
}

fn is_python_project(files: &ProjectFiles, python_process: bool) -> bool {
    if files.contains_exact("manage.py")
        || files.any_exact(PYTHON_MANIFEST_FILES)
        || files.any_exact(PYTHON_LOCK_FILES)
    {
        return true;
    }

    files.any_exact(PYTHON_ENTRY_FILES) && (python_process || !files.contains_exact("package.json"))
}

/// Run `detect` on the lowercased text of each listed file that can be read,
/// in order, and return the first label found.
fn detect_in_files(
    project_root: &Path,
    files: &ProjectFiles,
    file_names: &[&str],
    detect: fn(&str) -> Option<StackLabel>,
) -> Option<StackLabel> {
    file_names
        .iter()
        .filter_map(|file_name| files.read_text(project_root, file_name))
        .find_map(|mut text| {
            text.make_ascii_lowercase();
            detect(&text)
        })
}

/// The first framework a manifest declares that every lock file confirms.
///
/// Manifests are checked in order, and within one manifest the frameworks in
/// [`PYTHON_DEPENDENCY_PATTERNS`] order. A lock file that was read completely
/// must list the framework as a package; a lock file longer than the read cap
/// cannot rule it out. Lock files are read only once a manifest names a
/// framework.
fn detect_python_framework_from_dependencies(
    project_root: &Path,
    files: &ProjectFiles,
) -> Option<StackLabel> {
    let locks = OnceCell::new();
    let confirmed_by_locks = |package: &str| {
        locks
            .get_or_init(|| read_lock_files(project_root, files))
            .iter()
            .all(|lock| lock_lists_package(lock, package))
    };

    PYTHON_MANIFEST_FILES
        .iter()
        .filter_map(|file_name| files.read_text(project_root, file_name))
        .find_map(|mut manifest| {
            manifest.make_ascii_lowercase();
            PYTHON_DEPENDENCY_PATTERNS
                .iter()
                .find(|(package, _)| {
                    declares_dependency(&manifest, package) && confirmed_by_locks(package)
                })
                .map(|(_, label)| label.clone())
        })
}

fn read_lock_files(project_root: &Path, files: &ProjectFiles) -> Vec<String> {
    PYTHON_LOCK_FILES
        .iter()
        .filter_map(|file_name| files.read_complete_text(project_root, file_name))
        .map(|mut lock| {
            lock.make_ascii_lowercase();
            lock
        })
        .collect()
}

/// Whether a lowercased `uv.lock` or `poetry.lock` has a package entry named
/// `package`. Both write `name = "package"` in each `[[package]]` table.
fn lock_lists_package(lock: &str, package: &str) -> bool {
    lock.lines().any(|line| {
        line.trim()
            .strip_prefix("name = \"")
            .and_then(|rest| rest.strip_suffix('"'))
            == Some(package)
    })
}

/// Whether a lowercased manifest names `package` outside `#` comment lines.
fn declares_dependency(manifest: &str, package: &str) -> bool {
    manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .any(|line| contains_dependency_token(line, package))
}

fn detect_python_framework_from_source(normalized: &str) -> Option<StackLabel> {
    if contains_any(normalized, DJANGO_SOURCE_PATTERNS) {
        return Some(labels::DJANGO);
    }

    PYTHON_SOURCE_PATTERNS
        .iter()
        .find(|(_, imports, constructor)| {
            source_mentions_framework(normalized, imports, constructor)
        })
        .map(|(label, _, _)| label.clone())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn source_mentions_framework(haystack: &str, imports: &[&str], constructor: &str) -> bool {
    haystack.contains(constructor) && contains_any(haystack, imports)
}

/// Whether `token` occurs in `haystack` as a whole package name, not as part
/// of a longer name such as `flask-login` or `pytest-django`.
fn contains_dependency_token(haystack: &str, token: &str) -> bool {
    let bytes = haystack.as_bytes();

    haystack.match_indices(token).any(|(start, _)| {
        let before = start.checked_sub(1).and_then(|index| bytes.get(index));
        let after = bytes.get(start + token.len());
        is_dependency_boundary(before.copied()) && is_dependency_boundary(after.copied())
    })
}

const fn is_dependency_boundary(byte: Option<u8>) -> bool {
    match byte {
        None => true,
        Some(value) => !matches!(value, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'),
    }
}
