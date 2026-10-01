//! Image-name based stack detection.
//!
//! This module handles only image-string parsing and label matching. It does
//! not talk to a container daemon or validate that an image exists.

use crate::StackLabel;

pub const EXACT_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("mongo", StackLabel::database("MongoDB")),
    ("httpd", StackLabel::service("Apache")),
    ("node", StackLabel::runtime("Node.js")),
    ("python", StackLabel::runtime("Python")),
    ("python3", StackLabel::runtime("Python")),
    ("ruby", StackLabel::runtime("Ruby")),
    ("golang", StackLabel::runtime("Go")),
    ("go", StackLabel::runtime("Go")),
    ("rust", StackLabel::runtime("Rust")),
    ("bun", StackLabel::runtime("Bun")),
    ("deno", StackLabel::runtime("Deno")),
    ("php", StackLabel::runtime("PHP")),
];

/// Prefix rules match when the base name equals the prefix or continues with a
/// separator (`-`, `_`, `.`), so `postgrest` is not `postgres`. The image tag
/// (`:16`) is removed before matching.
pub const PREFIX_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("postgres", StackLabel::database("PostgreSQL")),
    ("postgresql", StackLabel::database("PostgreSQL")),
    ("postgis", StackLabel::database("PostgreSQL")),
    ("timescaledb", StackLabel::database("PostgreSQL")),
    ("mysql", StackLabel::database("MySQL")),
    ("mariadb", StackLabel::database("MariaDB")),
    ("mongodb", StackLabel::database("MongoDB")),
    ("redis", StackLabel::database("Redis")),
    ("valkey", StackLabel::database("Valkey")),
    ("memcached", StackLabel::database("Memcached")),
    ("nginx", StackLabel::service("Nginx")),
    ("apache", StackLabel::service("Apache")),
    ("rabbitmq", StackLabel::service("RabbitMQ")),
    ("kafka", StackLabel::service("Kafka")),
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

/// Name segments that mark a companion image (a metrics exporter, admin UI,
/// Kubernetes operator, backup job, client, or auth sidecar) rather than the
/// service itself: `postgres-exporter`, `nginx-prometheus-exporter`,
/// `opensearch-dashboards`, `mysql-workbench`, `traefik-forward-auth`.
///
/// `proxy` is deliberately absent: `nginx-proxy` and `nginx-proxy-manager` run
/// Nginx.
const COMPANION_SEGMENTS: &[&str] = &[
    "exporter",
    "dashboard",
    "dashboards",
    "operator",
    "admin",
    "ui",
    "gui",
    "backup",
    "client",
    "cli",
    "commander",
    "workbench",
    "shell",
    "curator",
    "auth",
];

/// Label for images under a `dotnet` registry namespace, such as
/// `mcr.microsoft.com/dotnet/aspnet`.
pub const DOTNET_NAMESPACE_LABEL: StackLabel = StackLabel::runtime(".NET");

/// Detect a stack label from a container or artifact image name.
///
/// The detector matches the base image name after removing any registry,
/// namespace, tag, or digest. Matching is ASCII case-insensitive.
///
/// Most language runtime images, such as `node`, `python`, and `php`, match
/// only their exact name, so `python-linter` or `rubygems-mirror` are not
/// runtimes. Database and service images, plus the `openjdk`,
/// `eclipse-temurin`, and `dotnet` runtime images, match a prefix followed by
/// the end of the name or a separator (`-`, `_`, `.`): `mysql-server` is
/// `MySQL` and `redis-sentinel` is `Redis`, while `postgrest` is not
/// `PostgreSQL` and `redisinsight` is not `Redis`.
/// Prefix matches are rejected when a later name segment marks a companion
/// image such as `exporter`, `dashboards`, `operator`, `admin`, or `ui`, so
/// `postgres-exporter` and `opensearch-dashboards` produce no label.
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
/// assert_eq!(detect_from_image("prometheuscommunity/postgres-exporter"), None);
/// assert_eq!(detect_from_image("postgrest/postgrest"), None);
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
        .find(|(prefix, _)| matches_service_prefix(base, prefix))
        .map(|(_, label)| label.clone())
}

fn image_has_dotnet_namespace(image: &str) -> bool {
    image
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case("dotnet"))
}

const NAME_SEPARATORS: [char; 3] = ['-', '_', '.'];

fn matches_service_prefix(base: &str, prefix: &str) -> bool {
    let Some(rest) = strip_prefix_ascii_case(base, prefix) else {
        return false;
    };

    if rest.is_empty() {
        return true;
    }

    rest.strip_prefix(NAME_SEPARATORS).is_some_and(|rest| {
        !rest.split(NAME_SEPARATORS).any(|segment| {
            COMPANION_SEGMENTS
                .iter()
                .any(|companion| segment.eq_ignore_ascii_case(companion))
        })
    })
}

fn strip_prefix_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .and_then(|_| value.get(prefix.len()..))
}
