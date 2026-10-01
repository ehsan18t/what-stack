use std::path::Path;

use super::files::ProjectFiles;
use crate::StackLabel;

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

/// Label for Python projects without a recognized framework.
pub const PYTHON_LABEL: StackLabel = StackLabel::runtime("Python");

/// Label for Django projects.
pub const DJANGO_LABEL: StackLabel = StackLabel::framework("Django");

type PythonSourcePattern = (StackLabel, &'static [&'static str], &'static str);

pub const PYTHON_SOURCE_PATTERNS: &[PythonSourcePattern] = &[
    (
        StackLabel::framework("FastAPI"),
        &["from fastapi import", "import fastapi"],
        "fastapi(",
    ),
    (
        StackLabel::framework("Starlette"),
        &["from starlette.applications import", "import starlette"],
        "starlette(",
    ),
    (
        StackLabel::framework("Litestar"),
        &["from litestar import", "import litestar"],
        "litestar(",
    ),
    (
        StackLabel::framework("Flask"),
        &["from flask import", "import flask"],
        "flask(",
    ),
];

pub const PYTHON_DEPENDENCY_PATTERNS: &[(&str, StackLabel)] = &[
    ("django", DJANGO_LABEL),
    ("flask", StackLabel::framework("Flask")),
    ("fastapi", StackLabel::framework("FastAPI")),
    ("starlette", StackLabel::framework("Starlette")),
    ("litestar", StackLabel::framework("Litestar")),
];

pub(super) fn detect_python_project(
    project_root: &Path,
    files: &ProjectFiles,
) -> Option<StackLabel> {
    if !is_python_project(files) {
        return None;
    }

    if files.contains_exact("manage.py") {
        return Some(DJANGO_LABEL);
    }

    detect_python_framework_from_entry_files(project_root, files)
        .or_else(|| detect_python_framework_from_dependencies(project_root, files))
        .or(Some(PYTHON_LABEL))
}

fn is_python_project(files: &ProjectFiles) -> bool {
    files.contains_exact("manage.py")
        || files.any_exact(PYTHON_ENTRY_FILES)
        || files.any_exact(PYTHON_DEPENDENCY_FILES)
}

fn detect_python_framework_from_entry_files(
    project_root: &Path,
    files: &ProjectFiles,
) -> Option<StackLabel> {
    for file_name in PYTHON_ENTRY_FILES {
        let Some(source) = files.read_text(project_root, file_name) else {
            continue;
        };

        if let Some(label) = detect_python_framework_from_source(&source) {
            return Some(label);
        }
    }

    None
}

fn detect_python_framework_from_dependencies(
    project_root: &Path,
    files: &ProjectFiles,
) -> Option<StackLabel> {
    for file_name in PYTHON_DEPENDENCY_FILES {
        let Some(contents) = files.read_text(project_root, file_name) else {
            continue;
        };

        let normalized = contents.to_ascii_lowercase();
        if let Some(label) = detect_python_framework_from_dependency_text(&normalized) {
            return Some(label);
        }
    }

    None
}

fn detect_python_framework_from_dependency_text(normalized: &str) -> Option<StackLabel> {
    PYTHON_DEPENDENCY_PATTERNS
        .iter()
        .find(|(package, _)| contains_dependency_token(normalized, package))
        .map(|(_, label)| label.clone())
}

fn detect_python_framework_from_source(source: &str) -> Option<StackLabel> {
    let normalized = source.to_ascii_lowercase();

    if contains_any(&normalized, DJANGO_SOURCE_PATTERNS) {
        return Some(DJANGO_LABEL);
    }

    PYTHON_SOURCE_PATTERNS
        .iter()
        .find(|(_, imports, constructor)| {
            source_mentions_framework(&normalized, imports, constructor)
        })
        .map(|(label, _, _)| label.clone())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn source_mentions_framework(haystack: &str, imports: &[&str], constructor: &str) -> bool {
    haystack.contains(constructor) && contains_any(haystack, imports)
}

fn contains_dependency_token(haystack: &str, token: &str) -> bool {
    let mut offset = 0;

    while let Some(index) = haystack[offset..].find(token) {
        let start = offset + index;
        let end = start + token.len();
        let bytes = haystack.as_bytes();
        let before = start
            .checked_sub(1)
            .and_then(|position| bytes.get(position))
            .copied();
        let after = bytes.get(end).copied();

        if is_dependency_boundary(before) && is_dependency_boundary(after) {
            return true;
        }

        offset = start + 1;
    }

    false
}

const fn is_dependency_boundary(byte: Option<u8>) -> bool {
    match byte {
        None => true,
        Some(value) => !matches!(value, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-'),
    }
}
