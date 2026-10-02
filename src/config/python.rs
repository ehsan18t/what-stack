use std::path::Path;

use super::files::ProjectFiles;
use crate::{StackLabel, labels};

const PYTHON_ENTRY_FILES: &[&str] = &["app.py", "main.py", "server.py", "wsgi.py", "asgi.py"];

const PYTHON_DEPENDENCY_FILES: &[&str] = &[
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "Pipfile",
    "poetry.lock",
    "uv.lock",
    "setup.py",
];

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
    .or_else(|| {
        detect_in_files(
            project_root,
            files,
            PYTHON_DEPENDENCY_FILES,
            detect_python_framework_from_dependency_text,
        )
    })
    .or_else(|| (!python_process).then_some(labels::PYTHON))
}

fn is_python_project(files: &ProjectFiles, python_process: bool) -> bool {
    if files.contains_exact("manage.py") || files.any_exact(PYTHON_DEPENDENCY_FILES) {
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

fn detect_python_framework_from_dependency_text(normalized: &str) -> Option<StackLabel> {
    PYTHON_DEPENDENCY_PATTERNS
        .iter()
        .find(|(package, _)| contains_dependency_token(normalized, package))
        .map(|(_, label)| label.clone())
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
