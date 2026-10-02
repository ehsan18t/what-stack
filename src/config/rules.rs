use std::path::Path;

use self::ConfigMatch::{AllOf, Exact, Extension, Prefix};
use super::files::ProjectFiles;
use super::python;
use crate::ecosystem::Ecosystem as E;
use crate::{StackLabel, labels};

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
    (Prefix("next.config"), labels::NEXT_JS, E::Node),
    (Prefix("nuxt.config"), labels::NUXT, E::Node),
    (Exact("angular.json"), labels::ANGULAR, E::Node),
    (Prefix("svelte.config"), labels::SVELTEKIT, E::Node),
    (Prefix("astro.config"), labels::ASTRO, E::Node),
    (Prefix("vite.config"), labels::VITE, E::Node),
    (Prefix("remix.config"), labels::REMIX, E::Node),
    (Prefix("gatsby-config"), labels::GATSBY, E::Node),
    (Prefix("vue.config"), labels::VUE_CLI, E::Node),
    (Prefix("webpack.config"), labels::WEBPACK, E::Node),
    (Exact("Cargo.toml"), labels::RUST, E::Rust),
    (Exact("go.mod"), labels::GO, E::Go),
    (Exact("go.work"), labels::GO, E::Go),
    (Exact("pom.xml"), labels::JAVA_MAVEN, E::Jvm),
    (Exact("build.gradle.kts"), labels::KOTLIN_GRADLE, E::Jvm),
    (Exact("build.gradle"), labels::JAVA_GRADLE, E::Jvm),
    (
        AllOf(&["artisan", "composer.json"]),
        labels::LARAVEL,
        E::Php,
    ),
    (Exact("composer.json"), labels::PHP, E::Php),
    (Exact("mix.exs"), labels::ELIXIR, E::Beam),
    (Exact("deno.json"), labels::DENO, E::Deno),
    (Exact("deno.jsonc"), labels::DENO, E::Deno),
];

/// Rules checked after Python detection, in priority order.
pub const LATE_CONFIG_RULES: &[ConfigRule] = &[
    (
        AllOf(&["Gemfile", "config.ru", "bin/rails"]),
        labels::RAILS,
        E::Ruby,
    ),
    (AllOf(&["Gemfile", "config.ru"]), labels::RUBY_RACK, E::Ruby),
    (Extension("csproj"), labels::DOTNET, E::DotNet),
    (Extension("fsproj"), labels::DOTNET_FSHARP, E::DotNet),
];

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
/// `None` means the process is unknown: every rule applies in the order of
/// [`detect_from_config`]. `Some(ecosystem)` limits detection to the rules
/// that ecosystem accepts (see [`E::accepts_config`]); a Python process gets
/// only framework labels (see [`python::detect_python_project`]).
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
