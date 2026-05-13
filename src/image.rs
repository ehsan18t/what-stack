//! Image-name based stack detection.

use std::borrow::Cow;

use crate::StackLabel;

/// Detect a stack label from a container or artifact image name.
#[must_use]
pub fn detect_from_image(image: &str) -> Option<StackLabel> {
    let normalized = image.to_ascii_lowercase();
    let last_segment = normalized
        .split('/')
        .next_back()
        .unwrap_or(normalized.as_str());
    let base = last_segment
        .split([':', '@'])
        .next()
        .unwrap_or(last_segment);

    let label = match base {
        name if name.starts_with("postgres") => Some("PostgreSQL"),
        name if name.starts_with("mysql") => Some("MySQL"),
        name if name.starts_with("mariadb") => Some("MariaDB"),
        name if name == "mongo" || name.starts_with("mongodb") => Some("MongoDB"),
        name if name == "redis" || name.starts_with("redis-stack") => Some("Redis"),
        name if name.starts_with("valkey") => Some("Valkey"),
        name if name.starts_with("memcached") => Some("Memcached"),
        name if name.starts_with("nginx") => Some("Nginx"),
        name if name == "httpd" || name.starts_with("apache") => Some("Apache"),
        name if name.starts_with("rabbitmq") => Some("RabbitMQ"),
        name if name.starts_with("localstack") => Some("LocalStack"),
        name if name.starts_with("elasticsearch") => Some("Elasticsearch"),
        name if name.starts_with("opensearch") => Some("OpenSearch"),
        name if name.starts_with("clickhouse") => Some("ClickHouse"),
        name if name.starts_with("caddy") => Some("Caddy"),
        name if name.starts_with("traefik") => Some("Traefik"),
        "node" => Some("Node.js"),
        "python" | "python3" => Some("Python"),
        "ruby" => Some("Ruby"),
        "golang" | "go" => Some("Go"),
        "rust" => Some("Rust"),
        name if name.starts_with("openjdk") || name.starts_with("eclipse-temurin") => Some("Java"),
        name if name.starts_with("dotnet")
            || normalized.split('/').any(|segment| segment == "dotnet") =>
        {
            Some(".NET")
        }
        _ => None,
    }?;

    Some(Cow::Borrowed(label))
}
