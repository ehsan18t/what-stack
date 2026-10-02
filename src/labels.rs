//! Built-in stack labels, one constant per label.
//!
//! Process, image, and config rules all refer to these constants, so the text
//! and [`StackKind`] of a label are defined in exactly one place and a label
//! cannot drift to a different kind in one rule table.

use crate::{StackKind, StackLabel};

macro_rules! labels {
    ($($name:ident = $kind:ident($text:literal);)+) => {
        $(
            #[doc = concat!("`", $text, "` (", stringify!($kind), ").")]
            pub const $name: StackLabel = StackLabel::from_static($text, StackKind::$kind);
        )+

        /// Every built-in label, for consistency tests.
        #[cfg(test)]
        pub const ALL: &[StackLabel] = &[$($name),+];
    };
}

labels! {
    // Language runtimes and application servers.
    NODE = Runtime("Node.js");
    DENO = Runtime("Deno");
    BUN = Runtime("Bun");
    PYTHON = Runtime("Python");
    GUNICORN = Runtime("Gunicorn");
    UVICORN = Runtime("Uvicorn");
    RUBY = Runtime("Ruby");
    PUMA = Runtime("Puma");
    PHP = Runtime("PHP");
    JAVA = Runtime("Java");
    DOTNET = Runtime(".NET");
    DOTNET_FSHARP = Runtime(".NET (F#)");
    GO = Runtime("Go");
    RUST = Runtime("Rust");
    ERLANG = Runtime("Erlang");
    ELIXIR = Runtime("Elixir");
    PERL = Runtime("Perl");
    DART = Runtime("Dart");
    SWIFT = Runtime("Swift");

    // Frameworks.
    NEXT_JS = Framework("Next.js");
    NUXT = Framework("Nuxt");
    ANGULAR = Framework("Angular");
    SVELTEKIT = Framework("SvelteKit");
    ASTRO = Framework("Astro");
    REMIX = Framework("Remix");
    GATSBY = Framework("Gatsby");
    REACT_ROUTER = Framework("React Router");
    NESTJS = Framework("NestJS");
    EXPRESS = Framework("Express");
    HUGO = Framework("Hugo");
    JEKYLL = Framework("Jekyll");
    DJANGO = Framework("Django");
    FLASK = Framework("Flask");
    FASTAPI = Framework("FastAPI");
    STARLETTE = Framework("Starlette");
    LITESTAR = Framework("Litestar");
    RAILS = Framework("Rails");
    RUBY_RACK = Framework("Ruby (Rack)");
    LARAVEL = Framework("Laravel");
    SYMFONY = Framework("Symfony");

    // Build tools, bundlers, and dev servers.
    VITE = Tool("Vite");
    WEBPACK = Tool("Webpack");
    VUE_CLI = Tool("Vue CLI");
    JAVA_MAVEN = Tool("Java (Maven)");
    JAVA_GRADLE = Tool("Java (Gradle)");
    KOTLIN_GRADLE = Tool("Kotlin (Gradle)");

    // Databases, caches, and search engines.
    POSTGRESQL = Database("PostgreSQL");
    MYSQL = Database("MySQL");
    MARIADB = Database("MariaDB");
    MONGODB = Database("MongoDB");
    REDIS = Database("Redis");
    VALKEY = Database("Valkey");
    MEMCACHED = Database("Memcached");
    CLICKHOUSE = Database("ClickHouse");
    COCKROACHDB = Database("CockroachDB");
    SQL_SERVER = Database("SQL Server");
    ELASTICSEARCH = Database("Elasticsearch");
    OPENSEARCH = Database("OpenSearch");

    // Web servers, proxies, brokers, and emulators.
    NGINX = Service("Nginx");
    APACHE = Service("Apache");
    CADDY = Service("Caddy");
    TRAEFIK = Service("Traefik");
    ENVOY = Service("Envoy");
    HAPROXY = Service("HAProxy");
    IIS = Service("IIS");
    RABBITMQ = Service("RabbitMQ");
    KAFKA = Service("Kafka");
    LOCALSTACK = Service("LocalStack");
}
