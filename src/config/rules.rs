use std::path::Path;

use self::ConfigMatch::{AllOf, Exact, Extension, Prefix};
use super::files::ProjectFiles;
use super::python;
use crate::StackLabel;
use crate::ecosystem::Ecosystem as E;

/// How a config rule recognizes its project files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigMatch {
    /// A root entry with exactly this name.
    Exact(&'static str),
    /// A root entry named like the prefix plus a common config suffix, such as
    /// `next.config.mjs`.
    Prefix(&'static str),
    /// Every listed path exists. Nested paths such as `bin/rails` use `/`.
    AllOf(&'static [&'static str]),
    /// A root entry with this file extension.
    Extension(&'static str),
}

/// One config rule: matcher, label, and the ecosystem of the label.
pub type ConfigRule = (ConfigMatch, StackLabel, E);

/// Rules checked before Python detection, in priority order.
pub const CONFIG_RULES: &[ConfigRule] = &[
    (
        Prefix("next.config"),
        StackLabel::framework("Next.js"),
        E::Node,
    ),
    (
        Prefix("nuxt.config"),
        StackLabel::framework("Nuxt"),
        E::Node,
    ),
    (
        Exact("angular.json"),
        StackLabel::framework("Angular"),
        E::Node,
    ),
    (
        Prefix("svelte.config"),
        StackLabel::framework("SvelteKit"),
        E::Node,
    ),
    (
        Prefix("astro.config"),
        StackLabel::framework("Astro"),
        E::Node,
    ),
    (Prefix("vite.config"), StackLabel::tool("Vite"), E::Node),
    (
        Prefix("remix.config"),
        StackLabel::framework("Remix"),
        E::Node,
    ),
    (
        Prefix("gatsby-config"),
        StackLabel::framework("Gatsby"),
        E::Node,
    ),
    (Prefix("vue.config"), StackLabel::tool("Vue CLI"), E::Node),
    (
        Prefix("webpack.config"),
        StackLabel::tool("Webpack"),
        E::Node,
    ),
    (Exact("Cargo.toml"), StackLabel::runtime("Rust"), E::Rust),
    (Exact("go.mod"), StackLabel::runtime("Go"), E::Go),
    (Exact("go.work"), StackLabel::runtime("Go"), E::Go),
    (Exact("pom.xml"), StackLabel::tool("Java (Maven)"), E::Jvm),
    (
        Exact("build.gradle.kts"),
        StackLabel::tool("Kotlin (Gradle)"),
        E::Jvm,
    ),
    (
        Exact("build.gradle"),
        StackLabel::tool("Java (Gradle)"),
        E::Jvm,
    ),
    (
        AllOf(&["artisan", "composer.json"]),
        StackLabel::framework("Laravel"),
        E::Php,
    ),
    (Exact("composer.json"), StackLabel::runtime("PHP"), E::Php),
    (Exact("mix.exs"), StackLabel::runtime("Elixir"), E::Beam),
    (Exact("deno.json"), StackLabel::runtime("Deno"), E::Deno),
    (Exact("deno.jsonc"), StackLabel::runtime("Deno"), E::Deno),
];

/// Rules checked after Python detection, in priority order.
pub const LATE_CONFIG_RULES: &[ConfigRule] = &[
    (
        AllOf(&["Gemfile", "config.ru", "bin/rails"]),
        StackLabel::framework("Rails"),
        E::Ruby,
    ),
    (AllOf(&["Gemfile", "config.ru"]), RACK_LABEL, E::Ruby),
    (Extension("csproj"), StackLabel::runtime(".NET"), E::DotNet),
    (
        Extension("fsproj"),
        StackLabel::runtime(".NET (F#)"),
        E::DotNet,
    ),
];

/// Label for Ruby projects with both `Gemfile` and `config.ru` but no
/// `bin/rails`.
pub const RACK_LABEL: StackLabel = StackLabel::framework("Ruby (Rack)");

const COMMON_CONFIG_SUFFIXES: &[&str] = &["", ".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"];

/// Detect a stack label from configuration files in a project root.
///
/// The function scans only `project_root` and checks built-in rules in a fixed
/// priority order. More specific frontend framework config files are evaluated
/// before generic runtime markers such as `Cargo.toml` or `go.mod`. Python
/// projects get a second pass that can identify `Django`, `Flask`, `FastAPI`,
/// `Starlette`, and `Litestar` from entry files or dependency files. Ruby
/// projects need `Gemfile` and `config.ru` (`Ruby (Rack)`), plus `bin/rails`
/// for `Rails`. PHP projects with `artisan` next to `composer.json` are
/// `Laravel`.
///
/// This function knows nothing about the process, so it uses the same rules
/// [`StackDetector`](crate::StackDetector) applies to an unknown process.
/// Python entry files such as `server.py` alone do not make a Python project
/// when `package.json` is present without a Python dependency file. The
/// detector narrows the rules to the process's ecosystem: a `php` process in a
/// Laravel project with `vite.config.js` is `Laravel`, while a `node` process
/// there is `Vite`.
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
    detect_for_ecosystem(project_root, None)
}

/// Config detection for a process of a known ecosystem.
///
/// `None` means the process is unknown: every rule applies in priority order,
/// and Python entry files alone do not make a Python project when
/// `package.json` is present. `Some(ecosystem)` limits detection to that
/// ecosystem's rules (a `deno` process also accepts Node rules), so a `php` process in a Laravel project with
/// `vite.config.js` is `Laravel` and a `node` process next to a stray
/// `server.py` is not `Python`. A Python process gets only framework labels:
/// the generic `Python` fallback adds nothing to it, and would replace a more
/// specific `Gunicorn` or `Uvicorn` label.
pub fn detect_for_ecosystem(project_root: &Path, scope: Option<E>) -> Option<StackLabel> {
    let files = ProjectFiles::read(project_root)?;
    let in_scope = |ecosystem: E| scope.is_none_or(|scope| scope.accepts_config(ecosystem));
    let python_process = scope == Some(E::Python);

    detect_from_rules(project_root, &files, CONFIG_RULES, in_scope)
        .or_else(|| {
            in_scope(E::Python)
                .then(|| python::detect_python_project(project_root, &files, python_process))
                .flatten()
        })
        .or_else(|| detect_from_rules(project_root, &files, LATE_CONFIG_RULES, in_scope))
}

fn detect_from_rules(
    project_root: &Path,
    files: &ProjectFiles,
    rules: &[ConfigRule],
    in_scope: impl Fn(E) -> bool,
) -> Option<StackLabel> {
    rules
        .iter()
        .filter(|(_, _, ecosystem)| in_scope(*ecosystem))
        .find(|(matcher, _, _)| rule_matches(project_root, files, *matcher))
        .map(|(_, label, _)| label.clone())
}

fn rule_matches(project_root: &Path, files: &ProjectFiles, matcher: ConfigMatch) -> bool {
    match matcher {
        Exact(name) => files.contains_exact(name),
        Prefix(prefix) => files.contains_prefix(prefix),
        AllOf(paths) => paths
            .iter()
            .all(|path| files.contains_path(project_root, path)),
        Extension(extension) => files.contains_extension(extension),
    }
}

pub(super) fn matches_config_name_prefix(name: &str, pattern: &str) -> bool {
    name.strip_prefix(pattern)
        .is_some_and(|suffix| COMMON_CONFIG_SUFFIXES.contains(&suffix))
}
