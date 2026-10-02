//! Process-name based stack detection.
//!
//! Process detection is deliberately exact: `node` maps to `Node.js`, while
//! unrelated names such as `node-exporter` do not. Two narrow relaxations
//! cover real process tables: runtime version suffixes (`python3.12`,
//! `php-fpm8.2`) and titled processes truncated by Linux (`next-server (v1`,
//! `gunicorn: maste`).

use crate::ecosystem::Ecosystem as E;
use crate::text::strip_suffix_ignore_ascii_case;
use crate::{StackKind, StackLabel, labels};

/// One process rule: every executable name of one label, the label, and the
/// ecosystem whose project config may refine the label.
pub type ProcessRule = (&'static [&'static str], StackLabel, E);

/// Known process names mapped to stack labels, one row per label.
///
/// Linear scan with `eq_ignore_ascii_case` avoids allocating a lowercase
/// `String` on every lookup.
pub const PROCESS_RULES: &[ProcessRule] = &[
    (&["node", "nodejs"], labels::NODE, E::Node),
    (&["python", "python3", "pythonw"], labels::PYTHON, E::Python),
    (&["ruby"], labels::RUBY, E::Ruby),
    (&["java", "javaw"], labels::JAVA, E::Jvm),
    (&["go"], labels::GO, E::Go),
    (&["deno"], labels::DENO, E::Deno),
    (&["bun"], labels::BUN, E::Node),
    (&["dotnet"], labels::DOTNET, E::DotNet),
    (&["php", "php-fpm"], labels::PHP, E::Php),
    (&["perl"], labels::PERL, E::Other),
    (&["cargo", "rustc"], labels::RUST, E::Rust),
    (&["erlang", "beam.smp"], labels::ERLANG, E::Beam),
    (&["elixir"], labels::ELIXIR, E::Beam),
    (&["dart"], labels::DART, E::Other),
    (&["swift"], labels::SWIFT, E::Other),
    (&["postgres", "postgresql"], labels::POSTGRESQL, E::Other),
    (&["mysqld", "mysql"], labels::MYSQL, E::Other),
    (&["mariadbd", "mariadb"], labels::MARIADB, E::Other),
    (&["mongod", "mongos"], labels::MONGODB, E::Other),
    (&["redis-server", "redis"], labels::REDIS, E::Other),
    (&["valkey-server", "valkey"], labels::VALKEY, E::Other),
    (&["memcached"], labels::MEMCACHED, E::Other),
    (&["clickhouse-server"], labels::CLICKHOUSE, E::Other),
    (&["cockroach"], labels::COCKROACHDB, E::Other),
    (&["sqlservr"], labels::SQL_SERVER, E::Other),
    (&["nginx"], labels::NGINX, E::Other),
    (&["apache2", "httpd"], labels::APACHE, E::Other),
    (&["caddy"], labels::CADDY, E::Other),
    (&["traefik"], labels::TRAEFIK, E::Other),
    (&["envoy"], labels::ENVOY, E::Other),
    (&["haproxy"], labels::HAPROXY, E::Other),
    (&["w3wp"], labels::IIS, E::Other),
    (&["gunicorn"], labels::GUNICORN, E::Python),
    (&["uvicorn"], labels::UVICORN, E::Python),
    (&["puma"], labels::PUMA, E::Ruby),
    (&["elasticsearch"], labels::ELASTICSEARCH, E::Other),
    (&["opensearch"], labels::OPENSEARCH, E::Other),
    (&["rabbitmq-server"], labels::RABBITMQ, E::Other),
    (&["kafka"], labels::KAFKA, E::Other),
    (&["webpack"], labels::WEBPACK, E::Node),
    (&["vite"], labels::VITE, E::Node),
    (&["next-server"], labels::NEXT_JS, E::Node),
    (&["nuxt"], labels::NUXT, E::Node),
    (&["hugo"], labels::HUGO, E::Other),
    (&["jekyll"], labels::JEKYLL, E::Ruby),
    (&["flask"], labels::FLASK, E::Python),
    (&["rails"], labels::RAILS, E::Ruby),
    (&["gradle"], labels::JAVA_GRADLE, E::Jvm),
    (&["mvn"], labels::JAVA_MAVEN, E::Jvm),
];

/// Detect a stack label from a process executable name.
///
/// Matching is ASCII case-insensitive. A trailing Windows `.exe` suffix is
/// ignored before matching, so `NGINX.EXE` and `nginx` produce the same label.
///
/// Two relaxations apply when the exact name is unknown:
///
/// - A trailing version made of digits and dots is ignored for runtime names,
///   so `python3.12`, `php8.2`, `php-fpm8.2`, `ruby3.2`, and `node20` match.
/// - Processes that set a title which Linux truncates to 15 characters match
///   on the word before the first space or colon: `next-server (v1` is
///   `Next.js`, `puma 6.4.2 (tc` and `puma: cluster w` are `Puma`, and
///   `gunicorn: maste` is `Gunicorn`.
///
/// # Examples
///
/// ```
/// use what_stack::detect_from_process;
///
/// use what_stack::StackKind;
///
/// let python = detect_from_process("python3").expect("known runtime");
/// assert_eq!(python, "Python");
/// assert_eq!(python.kind(), StackKind::Runtime);
/// assert_eq!(detect_from_process("python3.12").expect("versioned runtime"), "Python");
/// assert_eq!(detect_from_process("next-server (v1").expect("titled"), "Next.js");
/// assert_eq!(detect_from_process("gunicorn: maste").expect("titled"), "Gunicorn");
/// assert_eq!(detect_from_process("NGINX.EXE").expect("known server"), "Nginx");
/// assert_eq!(detect_from_process("node-exporter"), None);
/// ```
///
/// # Limits
///
/// This function does not inspect command-line arguments or project files.
/// Use [`StackDetector`](crate::StackDetector) when process names should be
/// combined with project config and image metadata.
#[must_use]
pub fn detect_from_process(process_name: &str) -> Option<StackLabel> {
    find_process_rule(process_name).map(|(_, label, _)| label.clone())
}

/// Detect a stack label from a process name, falling back to an executable name.
///
/// The process-table name is tried first. When it is unknown (for example
/// because the operating system truncated it), `exe_name` is tried with the
/// same rules as [`detect_from_process`].
///
/// # Examples
///
/// ```
/// use what_stack::detect_from_process_names;
///
/// assert_eq!(
///     detect_from_process_names("redis-serv", Some("redis-server")).expect("known"),
///     "Redis",
/// );
/// assert_eq!(detect_from_process_names("node", None).expect("known"), "Node.js");
/// assert_eq!(detect_from_process_names("helper", Some("helper.exe")), None);
/// ```
#[must_use]
pub fn detect_from_process_names(process_name: &str, exe_name: Option<&str>) -> Option<StackLabel> {
    find_process_rule_by_names(process_name, exe_name).map(|(_, label, _)| label.clone())
}

/// Process names whose processes set a title that starts with the name and a
/// space or colon (`puma 6.4.2 (tcp://...)`, `gunicorn: master [app]` from
/// `setproctitle`). Linux truncates the title to 15 characters in the process
/// table.
const TITLED_PROCESSES: &[&str] = &["next-server", "puma", "gunicorn"];

/// Same lookup as [`detect_from_process_names`], returning the whole rule.
pub fn find_process_rule_by_names(
    process_name: &str,
    exe_name: Option<&str>,
) -> Option<&'static ProcessRule> {
    find_process_rule(process_name).or_else(|| exe_name.and_then(find_process_rule))
}

fn find_process_rule(process_name: &str) -> Option<&'static ProcessRule> {
    let name = strip_windows_exe_suffix(process_name);

    find_exact(name)
        .or_else(|| find_titled(name))
        .or_else(|| find_versioned_runtime(name))
}

fn find_exact(name: &str) -> Option<&'static ProcessRule> {
    PROCESS_RULES
        .iter()
        .find(|(names, _, _)| names.iter().any(|key| name.eq_ignore_ascii_case(key)))
}

fn find_titled(name: &str) -> Option<&'static ProcessRule> {
    let (head, _) = name.split_once([' ', ':'])?;
    TITLED_PROCESSES
        .iter()
        .any(|titled| head.eq_ignore_ascii_case(titled))
        .then(|| find_exact(head))
        .flatten()
}

/// Match `python3.12` as `python`. Only runtime rules accept a version suffix,
/// and the suffix must start with a digit.
fn find_versioned_runtime(name: &str) -> Option<&'static ProcessRule> {
    let base = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    let version = name.get(base.len()..)?;

    if base.is_empty() || !version.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    find_exact(base).filter(|(_, label, _)| label.kind() == StackKind::Runtime)
}

fn strip_windows_exe_suffix(process_name: &str) -> &str {
    strip_suffix_ignore_ascii_case(process_name, ".exe").unwrap_or(process_name)
}
