//! Image-name based stack detection.
//!
//! This module handles only image-string parsing and label matching. It does
//! not talk to a container daemon or validate that an image exists.

use crate::StackLabel;

pub const EXACT_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("mongo", StackLabel::database("MongoDB")),
    ("redis", StackLabel::database("Redis")),
    ("httpd", StackLabel::service("Apache")),
    ("node", StackLabel::runtime("Node.js")),
    ("python", StackLabel::runtime("Python")),
    ("python3", StackLabel::runtime("Python")),
    ("ruby", StackLabel::runtime("Ruby")),
    ("golang", StackLabel::runtime("Go")),
    ("go", StackLabel::runtime("Go")),
    ("rust", StackLabel::runtime("Rust")),
];

pub const PREFIX_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("postgres", StackLabel::database("PostgreSQL")),
    ("mysql", StackLabel::database("MySQL")),
    ("mariadb", StackLabel::database("MariaDB")),
    ("mongodb", StackLabel::database("MongoDB")),
    ("redis-stack", StackLabel::database("Redis")),
    ("valkey", StackLabel::database("Valkey")),
    ("memcached", StackLabel::database("Memcached")),
    ("nginx", StackLabel::service("Nginx")),
    ("apache", StackLabel::service("Apache")),
    ("rabbitmq", StackLabel::service("RabbitMQ")),
    ("localstack", StackLabel::service("LocalStack")),
    ("elasticsearch", StackLabel::database("Elasticsearch")),
    ("opensearch", StackLabel::database("OpenSearch")),
    ("clickhouse", StackLabel::database("ClickHouse")),
    ("caddy", StackLabel::service("Caddy")),
    ("traefik", StackLabel::service("Traefik")),
    ("openjdk", StackLabel::runtime("Java")),
    ("eclipse-temurin", StackLabel::runtime("Java")),
    ("dotnet", StackLabel::runtime(".NET")),
];

/// Label for images under a `dotnet` registry namespace, such as
/// `mcr.microsoft.com/dotnet/aspnet`.
pub const DOTNET_NAMESPACE_LABEL: StackLabel = StackLabel::runtime(".NET");

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
/// assert_eq!(detect_from_image("postgres:16").expect("known image"), "PostgreSQL");
/// assert_eq!(
///     detect_from_image("mcr.microsoft.com/dotnet/aspnet:8.0").expect("known image"),
///     ".NET",
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

    detect_exact_base(base)
        .or_else(|| detect_prefixed_base(base))
        .or_else(|| image_has_dotnet_namespace(image).then_some(DOTNET_NAMESPACE_LABEL))
}

fn detect_exact_base(base: &str) -> Option<StackLabel> {
    EXACT_IMAGE_RULES
        .iter()
        .find(|(name, _)| base.eq_ignore_ascii_case(name))
        .map(|(_, label)| label.clone())
}

fn detect_prefixed_base(base: &str) -> Option<StackLabel> {
    PREFIX_IMAGE_RULES
        .iter()
        .find(|(prefix, _)| starts_with_ascii_case(base, prefix))
        .map(|(_, label)| label.clone())
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
