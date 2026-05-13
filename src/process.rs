//! Process-name based stack detection.
//!
//! Process detection is deliberately exact: `node` maps to `Node.js`, while
//! unrelated names such as `node-exporter` do not.

use std::borrow::Cow;

use crate::StackLabel;

/// Known process names mapped to stack labels.
///
/// Linear scan with `eq_ignore_ascii_case` avoids allocating a lowercase
/// `String` on every lookup.
const PROCESS_MAP: &[(&str, &str)] = &[
    ("node", "Node.js"),
    ("nodejs", "Node.js"),
    ("python", "Python"),
    ("python3", "Python"),
    ("ruby", "Ruby"),
    ("java", "Java"),
    ("go", "Go"),
    ("deno", "Deno"),
    ("bun", "Bun"),
    ("dotnet", ".NET"),
    ("php", "PHP"),
    ("perl", "Perl"),
    ("cargo", "Rust"),
    ("rustc", "Rust"),
    ("erlang", "Erlang"),
    ("beam.smp", "Erlang"),
    ("elixir", "Elixir"),
    ("dart", "Dart"),
    ("swift", "Swift"),
    ("postgres", "PostgreSQL"),
    ("postgresql", "PostgreSQL"),
    ("mysqld", "MySQL"),
    ("mysql", "MySQL"),
    ("mariadbd", "MariaDB"),
    ("mariadb", "MariaDB"),
    ("mongod", "MongoDB"),
    ("mongos", "MongoDB"),
    ("redis-server", "Redis"),
    ("redis", "Redis"),
    ("valkey-server", "Valkey"),
    ("valkey", "Valkey"),
    ("memcached", "Memcached"),
    ("clickhouse-server", "ClickHouse"),
    ("cockroach", "CockroachDB"),
    ("nginx", "Nginx"),
    ("apache2", "Apache"),
    ("httpd", "Apache"),
    ("caddy", "Caddy"),
    ("traefik", "Traefik"),
    ("envoy", "Envoy"),
    ("haproxy", "HAProxy"),
    ("gunicorn", "Gunicorn"),
    ("uvicorn", "Uvicorn"),
    ("elasticsearch", "Elasticsearch"),
    ("opensearch", "OpenSearch"),
    ("rabbitmq-server", "RabbitMQ"),
    ("kafka", "Kafka"),
    ("webpack", "Webpack"),
    ("vite", "Vite"),
    ("next-server", "Next.js"),
    ("nuxt", "Nuxt"),
    ("hugo", "Hugo"),
    ("jekyll", "Jekyll"),
    ("flask", "Flask"),
    ("rails", "Rails"),
    ("gradle", "Java (Gradle)"),
    ("mvn", "Java (Maven)"),
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
/// assert_eq!(detect_from_process("python3").as_deref(), Some("Python"));
/// assert_eq!(detect_from_process("NGINX.EXE").as_deref(), Some("Nginx"));
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
    let name = strip_windows_exe_suffix(process_name);

    PROCESS_MAP
        .iter()
        .find(|(key, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, label)| Cow::Borrowed(*label))
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
