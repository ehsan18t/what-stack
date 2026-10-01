//! Process-name based stack detection.
//!
//! Process detection is deliberately exact: `node` maps to `Node.js`, while
//! unrelated names such as `node-exporter` do not.

use crate::StackLabel;

/// One process rule: executable name and label.
pub type ProcessRule = (&'static str, StackLabel);

/// Known process names mapped to stack labels.
///
/// Each label carries its [`StackKind`](crate::StackKind) at the rule site.
/// Linear scan with `eq_ignore_ascii_case` avoids allocating a lowercase
/// `String` on every lookup.
pub const PROCESS_MAP: &[ProcessRule] = &[
    ("node", StackLabel::runtime("Node.js")),
    ("nodejs", StackLabel::runtime("Node.js")),
    ("python", StackLabel::runtime("Python")),
    ("python3", StackLabel::runtime("Python")),
    ("ruby", StackLabel::runtime("Ruby")),
    ("java", StackLabel::runtime("Java")),
    ("go", StackLabel::runtime("Go")),
    ("deno", StackLabel::runtime("Deno")),
    ("bun", StackLabel::runtime("Bun")),
    ("dotnet", StackLabel::runtime(".NET")),
    ("php", StackLabel::runtime("PHP")),
    ("perl", StackLabel::runtime("Perl")),
    ("cargo", StackLabel::runtime("Rust")),
    ("rustc", StackLabel::runtime("Rust")),
    ("erlang", StackLabel::runtime("Erlang")),
    ("beam.smp", StackLabel::runtime("Erlang")),
    ("elixir", StackLabel::runtime("Elixir")),
    ("dart", StackLabel::runtime("Dart")),
    ("swift", StackLabel::runtime("Swift")),
    ("postgres", StackLabel::database("PostgreSQL")),
    ("postgresql", StackLabel::database("PostgreSQL")),
    ("mysqld", StackLabel::database("MySQL")),
    ("mysql", StackLabel::database("MySQL")),
    ("mariadbd", StackLabel::database("MariaDB")),
    ("mariadb", StackLabel::database("MariaDB")),
    ("mongod", StackLabel::database("MongoDB")),
    ("mongos", StackLabel::database("MongoDB")),
    ("redis-server", StackLabel::database("Redis")),
    ("redis", StackLabel::database("Redis")),
    ("valkey-server", StackLabel::database("Valkey")),
    ("valkey", StackLabel::database("Valkey")),
    ("memcached", StackLabel::database("Memcached")),
    ("clickhouse-server", StackLabel::database("ClickHouse")),
    ("cockroach", StackLabel::database("CockroachDB")),
    ("nginx", StackLabel::service("Nginx")),
    ("apache2", StackLabel::service("Apache")),
    ("httpd", StackLabel::service("Apache")),
    ("caddy", StackLabel::service("Caddy")),
    ("traefik", StackLabel::service("Traefik")),
    ("envoy", StackLabel::service("Envoy")),
    ("haproxy", StackLabel::service("HAProxy")),
    ("gunicorn", StackLabel::runtime("Gunicorn")),
    ("uvicorn", StackLabel::runtime("Uvicorn")),
    ("elasticsearch", StackLabel::database("Elasticsearch")),
    ("opensearch", StackLabel::database("OpenSearch")),
    ("rabbitmq-server", StackLabel::service("RabbitMQ")),
    ("kafka", StackLabel::service("Kafka")),
    ("webpack", StackLabel::tool("Webpack")),
    ("vite", StackLabel::tool("Vite")),
    ("next-server", StackLabel::framework("Next.js")),
    ("nuxt", StackLabel::framework("Nuxt")),
    ("hugo", StackLabel::framework("Hugo")),
    ("jekyll", StackLabel::framework("Jekyll")),
    ("flask", StackLabel::framework("Flask")),
    ("rails", StackLabel::framework("Rails")),
    ("gradle", StackLabel::tool("Java (Gradle)")),
    ("mvn", StackLabel::tool("Java (Maven)")),
];

/// Detect a stack label from a process executable name.
///
/// Matching is ASCII case-insensitive. A trailing Windows `.exe` suffix is
/// ignored before matching, so `NGINX.EXE` and `nginx` produce the same label.
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
    find_process_rule(process_name).map(|(_, label)| label.clone())
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
    find_process_rule_by_names(process_name, exe_name).map(|(_, label)| label.clone())
}

/// Same lookup as [`detect_from_process_names`], returning the whole rule.
pub fn find_process_rule_by_names(
    process_name: &str,
    exe_name: Option<&str>,
) -> Option<&'static ProcessRule> {
    find_process_rule(process_name).or_else(|| exe_name.and_then(find_process_rule))
}

fn find_process_rule(process_name: &str) -> Option<&'static ProcessRule> {
    let name = strip_windows_exe_suffix(process_name);

    PROCESS_MAP
        .iter()
        .find(|(key, _)| name.eq_ignore_ascii_case(key))
}

fn strip_windows_exe_suffix(process_name: &str) -> &str {
    let Some(prefix_len) = process_name.len().checked_sub(4) else {
        return process_name;
    };

    match process_name.get(prefix_len..) {
        Some(suffix) if suffix.eq_ignore_ascii_case(".exe") => {
            process_name.get(..prefix_len).unwrap_or(process_name)
        }
        _ => process_name,
    }
}
