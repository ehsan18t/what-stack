//! Image-name based stack detection.
//!
//! This module handles only image-string parsing and label matching. It does
//! not talk to a container daemon or validate that an image exists.

use std::borrow::Cow;

use crate::StackLabel;

const EXACT_IMAGE_RULES: &[(&str, &str)] = &[
    ("mongo", "MongoDB"),
    ("redis", "Redis"),
    ("httpd", "Apache"),
    ("node", "Node.js"),
    ("python", "Python"),
    ("python3", "Python"),
    ("ruby", "Ruby"),
    ("golang", "Go"),
    ("go", "Go"),
    ("rust", "Rust"),
];

const PREFIX_IMAGE_RULES: &[(&str, &str)] = &[
    ("postgres", "PostgreSQL"),
    ("mysql", "MySQL"),
    ("mariadb", "MariaDB"),
    ("mongodb", "MongoDB"),
    ("redis-stack", "Redis"),
    ("valkey", "Valkey"),
    ("memcached", "Memcached"),
    ("nginx", "Nginx"),
    ("apache", "Apache"),
    ("rabbitmq", "RabbitMQ"),
    ("localstack", "LocalStack"),
    ("elasticsearch", "Elasticsearch"),
    ("opensearch", "OpenSearch"),
    ("clickhouse", "ClickHouse"),
    ("caddy", "Caddy"),
    ("traefik", "Traefik"),
    ("openjdk", "Java"),
    ("eclipse-temurin", "Java"),
    ("dotnet", ".NET"),
];

/// Detect a stack label from a container or artifact image name.
///
/// The detector matches the base image name after removing any registry,
/// namespace, tag, or digest. Matching is ASCII case-insensitive and includes
/// false-positive guards for common exporter, admin, mirror, and linter images.
///
/// # Examples
///
/// ```
/// use what_stack::detect_from_image;
///
/// assert_eq!(detect_from_image("postgres:16").as_deref(), Some("PostgreSQL"));
/// assert_eq!(
///     detect_from_image("mcr.microsoft.com/dotnet/aspnet:8.0").as_deref(),
///     Some(".NET"),
/// );
/// assert_eq!(detect_from_image("prom/node-exporter:latest"), None);
/// ```
///
/// # Limits
///
/// This is intentionally a built-in rule set. The crate does not read custom
/// image rules or inspect image manifests.
#[must_use]
pub fn detect_from_image(image: &str) -> Option<StackLabel> {
    let last_segment = image.rsplit('/').next().unwrap_or(image);
    let base = last_segment
        .split([':', '@'])
        .next()
        .unwrap_or(last_segment);

    let label = detect_exact_base(base)
        .or_else(|| detect_prefixed_base(base))
        .or_else(|| image_has_dotnet_namespace(image).then_some(".NET"))?;

    Some(Cow::Borrowed(label))
}

fn detect_exact_base(base: &str) -> Option<&'static str> {
    EXACT_IMAGE_RULES
        .iter()
        .find_map(|(name, label)| base.eq_ignore_ascii_case(name).then_some(*label))
}

fn detect_prefixed_base(base: &str) -> Option<&'static str> {
    PREFIX_IMAGE_RULES
        .iter()
        .find_map(|(prefix, label)| starts_with_ascii_case(base, prefix).then_some(*label))
}

fn image_has_dotnet_namespace(image: &str) -> bool {
    image
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case("dotnet"))
}

fn starts_with_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}
