use std::borrow::Cow;
use std::path::Path;

use super::files::ProjectFiles;
use super::python;
use crate::StackLabel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConfigMatchKind {
    Exact,
    Prefix,
}

const CONFIG_PATTERNS: &[(&str, &str, ConfigMatchKind)] = &[
    ("next.config", "Next.js", ConfigMatchKind::Prefix),
    ("nuxt.config", "Nuxt", ConfigMatchKind::Prefix),
    ("angular.json", "Angular", ConfigMatchKind::Exact),
    ("svelte.config", "SvelteKit", ConfigMatchKind::Prefix),
    ("astro.config", "Astro", ConfigMatchKind::Prefix),
    ("vite.config", "Vite", ConfigMatchKind::Prefix),
    ("remix.config", "Remix", ConfigMatchKind::Prefix),
    ("gatsby-config", "Gatsby", ConfigMatchKind::Prefix),
    ("vue.config", "Vue CLI", ConfigMatchKind::Prefix),
    ("webpack.config", "Webpack", ConfigMatchKind::Prefix),
    ("Cargo.toml", "Rust", ConfigMatchKind::Exact),
    ("go.mod", "Go", ConfigMatchKind::Exact),
    ("pom.xml", "Java (Maven)", ConfigMatchKind::Exact),
    (
        "build.gradle.kts",
        "Kotlin (Gradle)",
        ConfigMatchKind::Exact,
    ),
    ("build.gradle", "Java (Gradle)", ConfigMatchKind::Exact),
    ("composer.json", "PHP", ConfigMatchKind::Exact),
    ("mix.exs", "Elixir", ConfigMatchKind::Exact),
    ("deno.json", "Deno", ConfigMatchKind::Exact),
];

const CONFIG_EXTENSIONS: &[(&str, &str)] = &[("csproj", ".NET"), ("fsproj", ".NET (F#)")];

const COMMON_CONFIG_SUFFIXES: &[&str] = &["", ".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];

/// Detect a stack label from configuration files in a project root.
#[must_use]
pub fn detect_from_config(project_root: &Path) -> Option<StackLabel> {
    let files = ProjectFiles::read(project_root)?;

    detect_from_config_patterns(&files)
        .or_else(|| python::detect_python_project(project_root, &files))
        .or_else(|| detect_rack_project(&files))
        .or_else(|| detect_from_config_extensions(&files))
}

fn detect_from_config_patterns(files: &ProjectFiles) -> Option<StackLabel> {
    for (pattern, label, match_kind) in CONFIG_PATTERNS {
        let matches = match match_kind {
            ConfigMatchKind::Exact => files.contains_exact(pattern),
            ConfigMatchKind::Prefix => files.contains_prefix(pattern),
        };

        if matches {
            return Some(Cow::Borrowed(label));
        }
    }

    None
}

fn detect_rack_project(files: &ProjectFiles) -> Option<StackLabel> {
    (files.contains_exact("Gemfile") && files.contains_exact("config.ru"))
        .then_some(Cow::Borrowed("Ruby (Rack)"))
}

fn detect_from_config_extensions(files: &ProjectFiles) -> Option<StackLabel> {
    CONFIG_EXTENSIONS.iter().find_map(|(extension, label)| {
        files
            .contains_extension(extension)
            .then_some(Cow::Borrowed(*label))
    })
}

pub(super) fn matches_config_name_prefix(name: &str, pattern: &str) -> bool {
    name.strip_prefix(pattern)
        .is_some_and(|suffix| COMMON_CONFIG_SUFFIXES.contains(&suffix))
}
