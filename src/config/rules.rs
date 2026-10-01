use std::path::Path;

use super::files::ProjectFiles;
use super::python;
use crate::StackLabel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigMatchKind {
    Exact,
    Prefix,
}

pub const CONFIG_PATTERNS: &[(&str, StackLabel, ConfigMatchKind)] = &[
    (
        "next.config",
        StackLabel::framework("Next.js"),
        ConfigMatchKind::Prefix,
    ),
    (
        "nuxt.config",
        StackLabel::framework("Nuxt"),
        ConfigMatchKind::Prefix,
    ),
    (
        "angular.json",
        StackLabel::framework("Angular"),
        ConfigMatchKind::Exact,
    ),
    (
        "svelte.config",
        StackLabel::framework("SvelteKit"),
        ConfigMatchKind::Prefix,
    ),
    (
        "astro.config",
        StackLabel::framework("Astro"),
        ConfigMatchKind::Prefix,
    ),
    (
        "vite.config",
        StackLabel::tool("Vite"),
        ConfigMatchKind::Prefix,
    ),
    (
        "remix.config",
        StackLabel::framework("Remix"),
        ConfigMatchKind::Prefix,
    ),
    (
        "gatsby-config",
        StackLabel::framework("Gatsby"),
        ConfigMatchKind::Prefix,
    ),
    (
        "vue.config",
        StackLabel::tool("Vue CLI"),
        ConfigMatchKind::Prefix,
    ),
    (
        "webpack.config",
        StackLabel::tool("Webpack"),
        ConfigMatchKind::Prefix,
    ),
    (
        "Cargo.toml",
        StackLabel::runtime("Rust"),
        ConfigMatchKind::Exact,
    ),
    ("go.mod", StackLabel::runtime("Go"), ConfigMatchKind::Exact),
    ("go.work", StackLabel::runtime("Go"), ConfigMatchKind::Exact),
    (
        "pom.xml",
        StackLabel::tool("Java (Maven)"),
        ConfigMatchKind::Exact,
    ),
    (
        "build.gradle.kts",
        StackLabel::tool("Kotlin (Gradle)"),
        ConfigMatchKind::Exact,
    ),
    (
        "build.gradle",
        StackLabel::tool("Java (Gradle)"),
        ConfigMatchKind::Exact,
    ),
    (
        "composer.json",
        StackLabel::runtime("PHP"),
        ConfigMatchKind::Exact,
    ),
    (
        "mix.exs",
        StackLabel::runtime("Elixir"),
        ConfigMatchKind::Exact,
    ),
    (
        "deno.json",
        StackLabel::runtime("Deno"),
        ConfigMatchKind::Exact,
    ),
    (
        "deno.jsonc",
        StackLabel::runtime("Deno"),
        ConfigMatchKind::Exact,
    ),
];

pub const CONFIG_EXTENSIONS: &[(&str, StackLabel)] = &[
    ("csproj", StackLabel::runtime(".NET")),
    ("fsproj", StackLabel::runtime(".NET (F#)")),
];

/// Label for Ruby projects with both `Gemfile` and `config.ru`.
pub const RACK_LABEL: StackLabel = StackLabel::framework("Ruby (Rack)");

const COMMON_CONFIG_SUFFIXES: &[&str] = &["", ".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];

/// Detect a stack label from configuration files in a project root.
///
/// The function scans only `project_root` and checks built-in rules in a fixed
/// priority order. More specific frontend framework config files are evaluated
/// before generic runtime markers such as `Cargo.toml` or `go.mod`. Python
/// projects get a second pass that can identify `Django`, `Flask`, `FastAPI`,
/// `Starlette`, and `Litestar` from entry files or dependency files.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use what_stack::detect_from_config;
///
/// let label = detect_from_config(Path::new("/workspace/api"));
/// println!("{label:?}");
/// ```
///
/// # Performance
///
/// Directory entries are read once into a small in-memory set. Source and
/// dependency files used for Python detection are capped to the first 64 KiB.
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
            return Some(label.clone());
        }
    }

    None
}

fn detect_rack_project(files: &ProjectFiles) -> Option<StackLabel> {
    (files.contains_exact("Gemfile") && files.contains_exact("config.ru")).then_some(RACK_LABEL)
}

fn detect_from_config_extensions(files: &ProjectFiles) -> Option<StackLabel> {
    CONFIG_EXTENSIONS
        .iter()
        .find(|(extension, _)| files.contains_extension(extension))
        .map(|(_, label)| label.clone())
}

pub(super) fn matches_config_name_prefix(name: &str, pattern: &str) -> bool {
    name.strip_prefix(pattern)
        .is_some_and(|suffix| COMMON_CONFIG_SUFFIXES.contains(&suffix))
}
