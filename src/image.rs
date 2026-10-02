//! Image-name based stack detection.
//!
//! This module handles only image-string parsing and label matching. It does
//! not talk to a container daemon or validate that an image exists.

use crate::text::strip_prefix_ignore_ascii_case;
use crate::{StackLabel, labels};

pub const EXACT_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("mongo", labels::MONGODB),
    ("httpd", labels::APACHE),
    ("node", labels::NODE),
    ("python", labels::PYTHON),
    ("python3", labels::PYTHON),
    ("ruby", labels::RUBY),
    ("golang", labels::GO),
    ("go", labels::GO),
    ("rust", labels::RUST),
    ("bun", labels::BUN),
    ("deno", labels::DENO),
    ("php", labels::PHP),
    ("elixir", labels::ELIXIR),
];

/// Prefix rules match when the base name equals the prefix or continues with a
/// separator (`-`, `_`, `.`), so `postgrest` is not `postgres`. The image tag
/// (`:16`) is removed before matching.
pub const PREFIX_IMAGE_RULES: &[(&str, StackLabel)] = &[
    ("postgres", labels::POSTGRESQL),
    ("postgresql", labels::POSTGRESQL),
    ("postgis", labels::POSTGRESQL),
    ("timescaledb", labels::POSTGRESQL),
    ("pgvector", labels::POSTGRESQL),
    ("mysql", labels::MYSQL),
    ("mariadb", labels::MARIADB),
    ("mongodb", labels::MONGODB),
    ("redis", labels::REDIS),
    ("valkey", labels::VALKEY),
    ("memcached", labels::MEMCACHED),
    ("nginx", labels::NGINX),
    ("apache", labels::APACHE),
    ("rabbitmq", labels::RABBITMQ),
    ("kafka", labels::KAFKA),
    ("cp-kafka", labels::KAFKA),
    ("localstack", labels::LOCALSTACK),
    ("elasticsearch", labels::ELASTICSEARCH),
    ("opensearch", labels::OPENSEARCH),
    ("clickhouse", labels::CLICKHOUSE),
    ("caddy", labels::CADDY),
    ("traefik", labels::TRAEFIK),
    ("openjdk", labels::JAVA),
    ("eclipse-temurin", labels::JAVA),
    ("amazoncorretto", labels::JAVA),
    ("dotnet", labels::DOTNET),
];

/// Name segments that mark a companion image (a metrics exporter, admin UI,
/// Kubernetes operator, backup job, client, auth sidecar, or connector worker)
/// rather than the service itself: `postgres-exporter`,
/// `nginx-prometheus-exporter`, `opensearch-dashboards`, `mysql-workbench`,
/// `traefik-forward-auth`, `cp-kafka-connect`.
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
    "connect",
];

/// Registry namespaces whose images all run one stack, whatever the base
/// name: `mcr.microsoft.com/dotnet/aspnet` and `mcr.microsoft.com/mssql/server`.
const NAMESPACE_IMAGE_RULES: &[(&str, StackLabel)] =
    &[("dotnet", labels::DOTNET), ("mssql", labels::SQL_SERVER)];

/// Detect a stack label from a container or artifact image name.
///
/// The detector matches the base image name after removing any registry,
/// namespace, tag, or digest. Matching is ASCII case-insensitive.
///
/// Most language runtime images, such as `node`, `python`, and `php`, match
/// only their exact name, so `python-linter` or `rubygems-mirror` are not
/// runtimes. Database and service images, plus the `openjdk`,
/// `eclipse-temurin`, `amazoncorretto`, and `dotnet` runtime images, match a
/// prefix followed by
/// the end of the name or a separator (`-`, `_`, `.`): `mysql-server` is
/// `MySQL` and `redis-sentinel` is `Redis`, while `postgrest` is not
/// `PostgreSQL` and `redisinsight` is not `Redis`.
/// Prefix matches are rejected when a later name segment marks a companion
/// image such as `exporter`, `dashboards`, `operator`, `admin`, or `ui`, so
/// `postgres-exporter` and `opensearch-dashboards` produce no label.
/// Images under the `dotnet` or `mssql` registry namespaces are `.NET` and
/// `SQL Server` whatever their base name.
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
        .or_else(|| detect_namespace(image))
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

/// Match the namespace segments of `image` (every `/` segment but the last,
/// which holds the base name) against [`NAMESPACE_IMAGE_RULES`].
fn detect_namespace(image: &str) -> Option<StackLabel> {
    image.rsplit('/').skip(1).find_map(|segment| {
        NAMESPACE_IMAGE_RULES
            .iter()
            .find(|(namespace, _)| segment.eq_ignore_ascii_case(namespace))
            .map(|(_, label)| label.clone())
    })
}

const NAME_SEPARATORS: [char; 3] = ['-', '_', '.'];

fn matches_service_prefix(base: &str, prefix: &str) -> bool {
    let Some(rest) = strip_prefix_ignore_ascii_case(base, prefix) else {
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
