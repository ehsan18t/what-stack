#![allow(missing_docs, reason = "integration tests document behavior via names")]

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use what_stack::{
    MAX_WALK_DEPTH, ProjectInput, StackDetector, StackInput, StackKind, StackLabel,
    detect_from_config, detect_from_image, detect_from_process, detect_from_process_names,
    find_project_root, project_name, resolve_project_root,
};

/// Label text for assertions. Built-in labels are static, so this also checks
/// that detection did not allocate.
fn text(label: Option<StackLabel>) -> Option<&'static str> {
    match label?.into_cow() {
        Cow::Borrowed(text) => Some(text),
        Cow::Owned(text) => panic!("built-in label {text:?} should be static"),
    }
}

/// Fake home directory for tests that expect a walk to find nothing.
///
/// Fixtures are created inside it with [`temp_in`] and it is passed as the
/// walk ceiling, so marker files above the system temp directory cannot turn
/// an expected miss into a hit. On Windows `%TEMP%` lives under the user
/// profile, where a stray `package.json` is common.
fn fake_home() -> TempDir {
    TempDir::new().expect("fake home")
}

fn temp_in(home: &TempDir) -> TempDir {
    TempDir::new_in(home.path()).expect("temp dir")
}

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("test path has parent"))
        .expect("create parent directory");
    std::fs::write(path, contents).expect("write test file");
}

#[test]
fn image_detection_keeps_known_labels_and_false_positive_guards() {
    assert_eq!(text(detect_from_image("postgres:16")), Some("PostgreSQL"));
    assert_eq!(
        text(detect_from_image("ghcr.io/org/nginx:latest")),
        Some("Nginx")
    );
    assert_eq!(
        text(detect_from_image("mcr.microsoft.com/dotnet/aspnet:8.0")),
        Some(".NET")
    );
    assert_eq!(
        text(detect_from_image("redis/redis-stack:latest")),
        Some("Redis")
    );
    assert_eq!(detect_from_image("prom/node-exporter:latest"), None);
    assert_eq!(detect_from_image("mongo-express:latest"), None);
    assert_eq!(detect_from_image("redis-commander:latest"), None);
    assert_eq!(detect_from_image("python-linter:latest"), None);
    assert_eq!(detect_from_image("rubygems-mirror:latest"), None);
}

#[test]
fn image_detection_is_case_insensitive_and_ignores_tags_and_digests() {
    assert_eq!(text(detect_from_image("POSTGRES:16")), Some("PostgreSQL"));
    assert_eq!(
        text(detect_from_image("ghcr.io/org/NGINX@sha256:abcd")),
        Some("Nginx")
    );
    assert_eq!(
        text(detect_from_image("mcr.microsoft.com/DOTNET/aspnet:8.0")),
        Some(".NET")
    );
}

#[test]
fn image_detection_covers_existing_service_and_runtime_labels() {
    for (image, expected) in [
        ("mysql:8", "MySQL"),
        ("mariadb:11", "MariaDB"),
        ("mongodb/mongodb-community-server:7.0", "MongoDB"),
        ("valkey/valkey:8-alpine", "Valkey"),
        ("memcached:1.6", "Memcached"),
        ("httpd:2.4", "Apache"),
        ("rabbitmq:4", "RabbitMQ"),
        ("localstack/localstack:latest", "LocalStack"),
        ("opensearchproject/opensearch:2", "OpenSearch"),
        ("clickhouse/clickhouse-server:25", "ClickHouse"),
        ("caddy:2", "Caddy"),
        ("traefik:v3", "Traefik"),
        ("eclipse-temurin:21", "Java"),
        ("golang:1.24", "Go"),
        ("rust:1.90", "Rust"),
    ] {
        assert_eq!(
            text(detect_from_image(image)),
            Some(expected),
            "image {image} should detect {expected}"
        );
    }
}

#[test]
fn process_detection_is_exact_case_insensitive_and_strips_windows_exe_suffix() {
    assert_eq!(text(detect_from_process("postgres")), Some("PostgreSQL"));
    assert_eq!(text(detect_from_process("NGINX.EXE")), Some("Nginx"));
    assert_eq!(text(detect_from_process("Node")), Some("Node.js"));
    assert_eq!(detect_from_process("com.docker.backend"), None);
    assert_eq!(detect_from_process("node-exporter"), None);
}

#[test]
fn config_detection_preserves_framework_priority_and_python_specifics() {
    let next = TempDir::new().expect("temp dir");
    write_file(next.path(), "next.config.mjs", "");
    write_file(next.path(), "package.json", "{}");
    assert_eq!(text(detect_from_config(next.path())), Some("Next.js"));

    let fastapi = TempDir::new().expect("temp dir");
    write_file(
        fastapi.path(),
        "app.py",
        "from fastapi import FastAPI\napp = FastAPI()\n",
    );
    assert_eq!(text(detect_from_config(fastapi.path())), Some("FastAPI"));

    let django = TempDir::new().expect("temp dir");
    write_file(
        django.path(),
        "wsgi.py",
        "from django.core.wsgi import get_wsgi_application\napplication = get_wsgi_application()\n",
    );
    assert_eq!(text(detect_from_config(django.path())), Some("Django"));

    let rack = TempDir::new().expect("temp dir");
    write_file(rack.path(), "Gemfile", "");
    write_file(rack.path(), "config.ru", "");
    assert_eq!(text(detect_from_config(rack.path())), Some("Ruby (Rack)"));

    let dotnet = TempDir::new().expect("temp dir");
    write_file(dotnet.path(), "service/MyApp.csproj", "");
    assert_eq!(
        text(detect_from_config(dotnet.path().join("service").as_path())),
        Some(".NET")
    );
}

#[test]
fn config_detection_covers_existing_project_markers_and_no_match_cases() {
    for (marker, expected) in [
        ("Cargo.toml", "Rust"),
        ("go.mod", "Go"),
        ("pom.xml", "Java (Maven)"),
        ("build.gradle.kts", "Kotlin (Gradle)"),
        ("build.gradle", "Java (Gradle)"),
        ("composer.json", "PHP"),
        ("mix.exs", "Elixir"),
        ("deno.json", "Deno"),
        ("vite.config.ts", "Vite"),
        ("astro.config.mjs", "Astro"),
        ("svelte.config.js", "SvelteKit"),
    ] {
        let dir = TempDir::new().expect("temp dir");
        write_file(dir.path(), marker, "");
        assert_eq!(
            text(detect_from_config(dir.path())),
            Some(expected),
            "marker {marker} should detect {expected}"
        );
    }

    let rack_without_gemfile = TempDir::new().expect("temp dir");
    write_file(rack_without_gemfile.path(), "config.ru", "");
    assert_eq!(detect_from_config(rack_without_gemfile.path()), None);

    let renamed = TempDir::new().expect("temp dir");
    write_file(renamed.path(), "Cargo.toml.bak", "");
    write_file(renamed.path(), "manage.py.old", "");
    assert_eq!(detect_from_config(renamed.path()), None);
}

#[test]
fn config_prefix_detection_accepts_only_known_source_suffixes() {
    for suffix in ["", ".js", ".cjs", ".mjs", ".ts", ".cts", ".mts"] {
        let dir = TempDir::new().expect("temp dir");
        write_file(dir.path(), &format!("next.config{suffix}"), "");
        assert_eq!(
            text(detect_from_config(dir.path())),
            Some("Next.js"),
            "next.config{suffix} should be accepted"
        );
    }

    let backup = TempDir::new().expect("temp dir");
    write_file(backup.path(), "next.config.bak", "");
    assert_eq!(detect_from_config(backup.path()), None);
}

#[test]
fn config_detection_distinguishes_dotnet_csharp_and_fsharp_projects() {
    let csharp = TempDir::new().expect("temp dir");
    write_file(csharp.path(), "MyApp.csproj", "");
    assert_eq!(text(detect_from_config(csharp.path())), Some(".NET"));

    let fsharp = TempDir::new().expect("temp dir");
    write_file(fsharp.path(), "MyApp.fsproj", "");
    assert_eq!(text(detect_from_config(fsharp.path())), Some(".NET (F#)"));
}

#[test]
fn python_config_detection_uses_dependencies_and_generic_fallback() {
    let flask_dependency = TempDir::new().expect("temp dir");
    write_file(
        flask_dependency.path(),
        "pyproject.toml",
        "[project]\ndependencies = [\"flask>=3.0\"]\n",
    );
    assert_eq!(
        text(detect_from_config(flask_dependency.path())),
        Some("Flask")
    );

    let generic_python = TempDir::new().expect("temp dir");
    write_file(generic_python.path(), "app.py", "print('hello')\n");
    assert_eq!(
        text(detect_from_config(generic_python.path())),
        Some("Python")
    );
}

#[test]
fn python_dependency_detection_uses_package_boundaries() {
    let plugin_only = TempDir::new().expect("temp dir");
    write_file(
        plugin_only.path(),
        "requirements.txt",
        "flask-login==0.6.3\nstarlette-exporter==0.23.0\n",
    );
    assert_eq!(text(detect_from_config(plugin_only.path())), Some("Python"));

    let direct = TempDir::new().expect("temp dir");
    write_file(
        direct.path(),
        "requirements.txt",
        "fastapi[standard]>=0.115\n",
    );
    assert_eq!(text(detect_from_config(direct.path())), Some("FastAPI"));
}

#[test]
fn project_detection_walks_upward_and_respects_home_ceiling() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "Cargo.toml", "");
    let nested = project.path().join("src").join("deep");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    assert_eq!(
        find_project_root(&nested, None).as_deref(),
        Some(project.path())
    );
    assert_eq!(
        project_name(project.path()).as_deref(),
        project.path().file_name().and_then(std::ffi::OsStr::to_str)
    );

    let fake_home = TempDir::new().expect("temp dir");
    write_file(fake_home.path(), "package.json", "{}");
    let unrelated = fake_home.path().join("unrelated");
    std::fs::create_dir_all(&unrelated).expect("create unrelated dir");
    assert_eq!(find_project_root(&unrelated, Some(fake_home.path())), None);
}

#[test]
fn project_detection_returns_none_without_markers_and_respects_depth_limit() {
    let home = fake_home();
    let unmarked = temp_in(&home);
    assert_eq!(find_project_root(unmarked.path(), Some(home.path())), None);

    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "package.json", "{}");

    let mut deep = project.path().to_path_buf();
    for index in 0..MAX_WALK_DEPTH {
        deep = deep.join(format!("d{index}"));
    }
    std::fs::create_dir_all(&deep).expect("create deep dir");

    assert_eq!(
        find_project_root(&deep, None),
        None,
        "walk should stop after MAX_WALK_DEPTH directories"
    );
}

#[test]
fn walk_finds_marker_at_max_depth_boundary() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "package.json", "{}");

    let mut deep = project.path().to_path_buf();
    for index in 0..MAX_WALK_DEPTH - 1 {
        deep = deep.join(format!("d{index}"));
    }
    std::fs::create_dir_all(&deep).expect("create deep dir");

    assert_eq!(
        find_project_root(&deep, None).as_deref(),
        Some(project.path()),
        "walk should still find a marker at the depth boundary"
    );
}

#[test]
#[ignore = "pending fix: a walk stopped by the depth cap must not cache misses"]
fn depth_capped_walk_does_not_poison_the_cache() {
    let home = fake_home();
    let project = temp_in(&home);
    write_file(project.path(), "package.json", "{}");

    let mut deep = project.path().to_path_buf();
    let mut within_reach = None;
    for index in 0..MAX_WALK_DEPTH + 4 {
        deep = deep.join(format!("d{index}"));
        if index == 8 {
            within_reach = Some(deep.clone());
        }
    }
    std::fs::create_dir_all(&deep).expect("create deep dir");
    let within_reach = within_reach.expect("shallow directory");

    let mut detector = StackDetector::with_home(Some(home.path().to_path_buf()));
    assert_eq!(
        detector.detect_project_root(ProjectInput::new().cwd(deep.as_path())),
        None,
        "the deep walk stops at the depth cap"
    );
    assert_eq!(
        detector
            .detect_project_root(ProjectInput::new().cwd(within_reach.as_path()))
            .as_deref(),
        Some(project.path()),
        "a directory within reach of the marker must still find it"
    );
}

#[test]
fn project_detection_accepts_csproj_extension_marker() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "MyApp.csproj", "");
    let nested = project.path().join("Controllers");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    assert_eq!(
        find_project_root(project.path(), None).as_deref(),
        Some(project.path())
    );
    assert_eq!(
        find_project_root(&nested, None).as_deref(),
        Some(project.path())
    );
}

#[test]
fn project_input_uses_cwd_then_exe_then_absolute_command_arguments() {
    let workspace = TempDir::new().expect("temp dir");
    let exe_root = workspace.path().join("service");
    let cmd_root = workspace.path().join("tooling");
    let exe_path = exe_root.join("bin").join("service.exe");
    let cmd_path = cmd_root.join("scripts").join("launcher.py");

    write_file(&exe_root, "Cargo.toml", "");
    write_file(&cmd_root, "pyproject.toml", "");
    write_file(exe_path.parent().expect("exe parent"), "service.exe", "");
    write_file(cmd_path.parent().expect("cmd parent"), "launcher.py", "");

    let cmd = vec![OsString::from(&cmd_path)];
    let input = ProjectInput::new().exe(exe_path.as_path()).cmd(&cmd);

    assert_eq!(
        resolve_project_root(input, None).as_deref(),
        Some(exe_root.as_path())
    );

    let cmd_only = ProjectInput::new().cmd(&cmd);
    assert_eq!(
        resolve_project_root(cmd_only, None).as_deref(),
        Some(cmd_root.as_path())
    );
    assert_eq!(
        StackDetector::new()
            .detect_project_root(cmd_only)
            .as_deref(),
        Some(cmd_root.as_path())
    );
}

#[test]
fn project_input_uses_cwd_before_exe_and_absolute_command_arguments() {
    let workspace = TempDir::new().expect("temp dir");
    let web_root = workspace.path().join("web");
    let exe_root = workspace.path().join("service");
    let tooling_root = workspace.path().join("tooling");
    let cwd = web_root.join("src");
    let exe_path = exe_root.join("bin").join("service.exe");
    let cmd_path = tooling_root.join("scripts").join("launcher.py");

    write_file(&web_root, "package.json", "{}");
    write_file(&exe_root, "Cargo.toml", "");
    write_file(&tooling_root, "pyproject.toml", "");
    write_file(&cwd, "index.ts", "");
    write_file(exe_path.parent().expect("exe parent"), "service.exe", "");
    write_file(cmd_path.parent().expect("cmd parent"), "launcher.py", "");

    let cmd = vec![OsString::from(&cmd_path)];
    let input = ProjectInput::new()
        .cwd(cwd.as_path())
        .exe(exe_path.as_path())
        .cmd(&cmd);

    assert_eq!(
        resolve_project_root(input, None).as_deref(),
        Some(web_root.as_path())
    );
    assert_eq!(
        StackDetector::new().detect_project_root(input).as_deref(),
        Some(web_root.as_path())
    );
}

#[test]
fn detector_home_ceiling_comes_only_from_the_detector() {
    let fake_home = TempDir::new().expect("temp dir");
    write_file(fake_home.path(), "package.json", "{}");
    let unrelated = fake_home.path().join("unrelated");
    std::fs::create_dir_all(&unrelated).expect("create unrelated dir");
    let input = ProjectInput::new().cwd(unrelated.as_path());

    let mut ceiling = StackDetector::with_home(Some(fake_home.path().to_path_buf()));
    let mut no_ceiling = StackDetector::with_home(None);

    // Query order must not matter: each detector owns one ceiling and one cache.
    assert_eq!(
        no_ceiling.detect_project_root(input).as_deref(),
        Some(fake_home.path())
    );
    assert_eq!(ceiling.detect_project_root(input), None);
    assert_eq!(
        no_ceiling.detect_project_root(input).as_deref(),
        Some(fake_home.path())
    );
    assert_eq!(ceiling.detect_project_root(input), None);
    assert_eq!(resolve_project_root(input, Some(fake_home.path())), None);
}

#[test]
fn detector_home_is_the_configured_ceiling() {
    let home = PathBuf::from("/not/a/real/home");
    assert_eq!(
        StackDetector::with_home(Some(home.clone())).home(),
        Some(home.as_path())
    );
    assert_eq!(StackDetector::with_home(None).home(), None);
    assert_eq!(
        StackDetector::new().home(),
        StackDetector::default().home(),
        "Default is the same as new"
    );
}

#[test]
fn process_names_fall_back_to_executable_name() {
    assert_eq!(
        text(detect_from_process_names(
            "redis-serv",
            Some("redis-server")
        )),
        Some("Redis")
    );
    assert_eq!(
        text(detect_from_process_names("node", Some("redis-server"))),
        Some("Node.js"),
        "the process-table name wins when it is known"
    );
    assert_eq!(text(detect_from_process_names("helper", None)), None);
    assert_eq!(
        text(detect_from_process_names("", Some("helper.exe"))),
        None
    );
}

#[test]
fn labels_carry_kinds_from_their_rules() {
    let kind = |label: Option<StackLabel>| label.map(|label| label.kind());

    assert_eq!(kind(detect_from_process("node")), Some(StackKind::Runtime));
    assert_eq!(kind(detect_from_process("vite")), Some(StackKind::Tool));
    assert_eq!(
        kind(detect_from_process("rails")),
        Some(StackKind::Framework)
    );
    assert_eq!(
        kind(detect_from_process("postgres")),
        Some(StackKind::Database)
    );
    assert_eq!(kind(detect_from_process("nginx")), Some(StackKind::Service));
    assert_eq!(
        kind(detect_from_image("redis:7")),
        Some(StackKind::Database)
    );
    assert_eq!(
        kind(detect_from_image("traefik:v3")),
        Some(StackKind::Service)
    );

    let next = TempDir::new().expect("temp dir");
    write_file(next.path(), "next.config.mjs", "");
    assert_eq!(
        kind(detect_from_config(next.path())),
        Some(StackKind::Framework)
    );
}

#[test]
fn stack_detector_preserves_priority_and_config_guard() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "next.config.js", "");

    let mut detector = StackDetector::new();
    let image_wins = detector.detect_stack(
        StackInput::new("node")
            .image("postgres:16")
            .project_root(project.path()),
    );
    assert_eq!(text(image_wins), Some("PostgreSQL"));

    let config_for_runtime =
        detector.detect_stack(StackInput::new("node").project_root(project.path()));
    assert_eq!(text(config_for_runtime), Some("Next.js"));

    let external_shell = TempDir::new().expect("temp dir");
    let shell_path = external_shell.path().join("pwsh.exe");
    write_file(external_shell.path(), "pwsh.exe", "");
    let guarded = detector.detect_stack(
        StackInput::new("pwsh.exe")
            .exe_name("pwsh.exe")
            .exe_path(shell_path.as_path())
            .project_root(project.path()),
    );
    assert_eq!(guarded, None);

    let inside_exe = project.path().join("bin").join("my-app.exe");
    write_file(project.path(), "bin/my-app.exe", "");
    let unknown_inside_project = detector.detect_stack(
        StackInput::new("my-app.exe")
            .exe_path(inside_exe.as_path())
            .project_root(project.path()),
    );
    assert_eq!(text(unknown_inside_project), Some("Next.js"));
}

#[test]
fn service_and_database_labels_are_not_overridden_by_project_config() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "next.config.js", "");
    write_file(project.path(), "package.json", "{}");

    let mut detector = StackDetector::new();
    for (process, expected) in [
        ("redis-server", "Redis"),
        ("postgres", "PostgreSQL"),
        ("nginx", "Nginx"),
        ("mongod", "MongoDB"),
        ("rabbitmq-server", "RabbitMQ"),
    ] {
        let label = detector.detect_stack(StackInput::new(process).project_root(project.path()));
        assert_eq!(
            text(label),
            Some(expected),
            "{process} started in a Next.js folder"
        );
    }

    let truncated = detector.detect_stack(
        StackInput::new("redis-serv")
            .exe_name("redis-server")
            .project_root(project.path()),
    );
    assert_eq!(text(truncated), Some("Redis"));
}

#[test]
fn framework_labels_are_final_and_tool_labels_accept_config() {
    let rack = TempDir::new().expect("temp dir");
    write_file(rack.path(), "Gemfile", "");
    write_file(rack.path(), "config.ru", "");

    let svelte = TempDir::new().expect("temp dir");
    write_file(svelte.path(), "svelte.config.js", "");

    let mut detector = StackDetector::new();
    let rails = detector.detect_stack(StackInput::new("rails").project_root(rack.path()));
    assert_eq!(text(rails), Some("Rails"));

    let ruby = detector.detect_stack(StackInput::new("ruby").project_root(rack.path()));
    assert_eq!(text(ruby), Some("Ruby (Rack)"));

    let vite = detector.detect_stack(StackInput::new("vite").project_root(svelte.path()));
    assert_eq!(text(vite), Some("SvelteKit"));
}

#[test]
fn stack_detector_caches_project_and_config_detection_results() {
    let home = fake_home();
    let project = temp_in(&home);
    let nested = project.path().join("src");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    write_file(project.path(), "package.json", "{}");
    write_file(project.path(), "next.config.js", "");

    let mut detector = StackDetector::with_home(Some(home.path().to_path_buf()));
    let input = ProjectInput::new().cwd(nested.as_path());
    assert_eq!(
        detector.detect_project_root(input).as_deref(),
        Some(project.path())
    );

    // `node` is a runtime, so only the config file can make it Next.js.
    let stack_input = StackInput::new("node").project_root(project.path());
    assert_eq!(text(detector.detect_stack(stack_input)), Some("Next.js"));

    std::fs::remove_file(project.path().join("next.config.js")).expect("remove config");
    std::fs::remove_file(project.path().join("package.json")).expect("remove marker");

    assert_eq!(
        detector.detect_project_root(input).as_deref(),
        Some(project.path()),
        "project root should come from the cache"
    );
    assert_eq!(
        text(detector.detect_stack(stack_input)),
        Some("Next.js"),
        "config label should come from the cache"
    );

    detector.clear();

    assert_eq!(detector.detect_project_root(input), None);
    assert_eq!(text(detector.detect_stack(stack_input)), Some("Node.js"));
}

#[cfg(windows)]
fn upper_case_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_uppercase())
}

#[cfg(windows)]
#[test]
fn home_ceiling_ignores_case_on_windows() {
    let fake_home = TempDir::new().expect("temp dir");
    write_file(fake_home.path(), "package.json", "{}");
    let unrelated = upper_case_path(&fake_home.path().join("unrelated"));
    std::fs::create_dir_all(&unrelated).expect("create unrelated dir");

    assert_eq!(find_project_root(&unrelated, Some(fake_home.path())), None);

    let mut detector = StackDetector::with_home(Some(fake_home.path().to_path_buf()));
    assert_eq!(
        detector.detect_project_root(ProjectInput::new().cwd(unrelated.as_path())),
        None
    );
}

#[cfg(windows)]
#[test]
fn executable_inside_project_ignores_case_on_windows() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "next.config.js", "");
    write_file(project.path(), "bin/my-app.exe", "");
    let exe_path = upper_case_path(&project.path().join("bin").join("my-app.exe"));

    let mut detector = StackDetector::new();
    let label = detector.detect_stack(
        StackInput::new("my-app.exe")
            .exe_path(exe_path.as_path())
            .project_root(project.path()),
    );
    assert_eq!(text(label), Some("Next.js"));
}

fn detect_with(
    detector: &mut StackDetector,
    process: &str,
    project_root: &Path,
) -> Option<&'static str> {
    text(detector.detect_stack(StackInput::new(process).project_root(project_root)))
}

#[test]
fn image_prefix_rules_reject_companion_images_and_non_boundary_names() {
    for image in [
        "prometheuscommunity/postgres-exporter:v0.15.0",
        "bitnami/postgres_exporter",
        "prom/mysqld-exporter:v0.15.1",
        "nginx/nginx-prometheus-exporter:1.1",
        "quay.io/prometheuscommunity/elasticsearch-exporter",
        "kbudde/rabbitmq-exporter",
        "opensearchproject/opensearch-dashboards:2",
        "postgrest/postgrest:v12",
        "registry.opensource.zalan.do/acid/postgres-operator",
        "bitnami/mariadb-backup",
        "linuxserver/mysql-workbench",
        "mysql/mysql-shell:8.0",
        "bitnami/elasticsearch-curator",
        "thomseddon/traefik-forward-auth:2",
        "rediscommander/redis-commander",
        "redis/redisinsight:latest",
        "oliver006/redis_exporter",
        "provectuslabs/kafka-ui",
    ] {
        assert_eq!(detect_from_image(image), None, "image {image}");
    }
}

#[test]
fn image_prefix_rules_keep_service_images_and_variants() {
    for (image, expected) in [
        ("postgres:16", "PostgreSQL"),
        ("bitnami/postgresql:16", "PostgreSQL"),
        ("nginx:alpine", "Nginx"),
        ("nginxinc/nginx-unprivileged:stable", "Nginx"),
        ("jwilder/nginx-proxy", "Nginx"),
        ("library/redis", "Redis"),
        ("redis/redis-stack-server:7.2", "Redis"),
        ("bitnami/redis-sentinel:7.2", "Redis"),
        ("bitnami/redis-cluster", "Redis"),
        ("postgis/postgis:16-3.4", "PostgreSQL"),
        ("timescale/timescaledb:latest-pg16", "PostgreSQL"),
        ("timescale/timescaledb-ha:pg16", "PostgreSQL"),
        ("apache/kafka:3.8.0", "Kafka"),
        ("bitnami/kafka:3.8", "Kafka"),
        ("mysql/mysql-server:8.0", "MySQL"),
        ("elasticsearch:8.15.0", "Elasticsearch"),
        ("rabbitmq:3-management", "RabbitMQ"),
        ("oven/bun:1", "Bun"),
        ("denoland/deno:alpine", "Deno"),
        ("php:8.3-apache", "PHP"),
        ("php:8.3-fpm-alpine", "PHP"),
    ] {
        assert_eq!(
            text(detect_from_image(image)),
            Some(expected),
            "image {image}"
        );
    }
}

#[test]
fn process_detection_strips_runtime_versions_and_matches_truncated_titles() {
    for (process, expected) in [
        ("python3.12", "Python"),
        ("PYTHON3.12.EXE", "Python"),
        ("pythonw.exe", "Python"),
        ("php8.2", "PHP"),
        ("php-fpm", "PHP"),
        ("php-fpm8.2", "PHP"),
        ("ruby3.2", "Ruby"),
        ("node20", "Node.js"),
        ("javaw.exe", "Java"),
        ("puma", "Puma"),
        ("puma 6.4.2 (tc", "Puma"),
        ("puma: cluster w", "Puma"),
        ("gunicorn: maste", "Gunicorn"),
        ("gunicorn: worke", "Gunicorn"),
        ("next-server (v1", "Next.js"),
        ("w3wp.exe", "IIS"),
        ("sqlservr.exe", "SQL Server"),
    ] {
        assert_eq!(
            text(detect_from_process(process)),
            Some(expected),
            "process {process}"
        );
    }

    for process in [
        "postgres14",
        "node-exporter2",
        "python.",
        "3.12",
        "next-server-x",
        "gunicorn-x: a",
        "MySQL Workbench.exe",
    ] {
        assert_eq!(detect_from_process(process), None, "process {process}");
    }

    let kind = |name: &str| detect_from_process(name).map(|label| label.kind());
    assert_eq!(kind("puma"), Some(StackKind::Runtime));
    assert_eq!(kind("w3wp"), Some(StackKind::Service));
    assert_eq!(kind("sqlservr"), Some(StackKind::Database));

    let image_kind = |image: &str| detect_from_image(image).map(|label| label.kind());
    assert_eq!(image_kind("apache/kafka"), Some(StackKind::Service));
    assert_eq!(image_kind("postgis/postgis"), Some(StackKind::Database));
}

#[test]
fn config_precedence_follows_the_process_ecosystem() {
    let laravel = TempDir::new().expect("temp dir");
    for file in ["composer.json", "artisan", "package.json", "vite.config.js"] {
        write_file(laravel.path(), file, "");
    }

    let django_vite = TempDir::new().expect("temp dir");
    write_file(django_vite.path(), "manage.py", "");
    write_file(
        django_vite.path(),
        "requirements.txt",
        "django\ndjango-vite\n",
    );
    write_file(django_vite.path(), "package.json", "{}");
    write_file(django_vite.path(), "vite.config.ts", "");

    let vite_rails = TempDir::new().expect("temp dir");
    for file in ["Gemfile", "config.ru", "bin/rails", "package.json"] {
        write_file(vite_rails.path(), file, "");
    }
    write_file(vite_rails.path(), "vite.config.ts", "");

    let mut detector = StackDetector::new();
    for (process, root, expected) in [
        ("php", laravel.path(), "Laravel"),
        ("php-fpm8.2", laravel.path(), "Laravel"),
        ("node", laravel.path(), "Vite"),
        ("vite", laravel.path(), "Vite"),
        ("php", laravel.path(), "Laravel"),
        ("python3", django_vite.path(), "Django"),
        ("gunicorn", django_vite.path(), "Django"),
        ("node", django_vite.path(), "Vite"),
        ("ruby", vite_rails.path(), "Rails"),
        ("puma", vite_rails.path(), "Rails"),
        ("bun", vite_rails.path(), "Vite"),
    ] {
        assert_eq!(
            detect_with(&mut detector, process, root),
            Some(expected),
            "{process} in {}",
            root.display()
        );
    }

    // Unknown processes keep the documented fixed order.
    assert_eq!(text(detect_from_config(laravel.path())), Some("Vite"));
}

#[test]
fn config_runtime_labels_do_not_replace_a_different_process_runtime() {
    let node_deno = TempDir::new().expect("temp dir");
    write_file(node_deno.path(), "package.json", "{}");
    write_file(node_deno.path(), "deno.json", "{}");

    let plain_python = TempDir::new().expect("temp dir");
    write_file(
        plain_python.path(),
        "requirements.txt",
        "requests
",
    );

    let flask = TempDir::new().expect("temp dir");
    write_file(
        flask.path(),
        "requirements.txt",
        "flask
",
    );

    let mut detector = StackDetector::new();
    for (process, root, expected) in [
        ("node", node_deno.path(), "Node.js"),
        ("bun", node_deno.path(), "Bun"),
        ("deno", node_deno.path(), "Deno"),
        ("gunicorn", plain_python.path(), "Gunicorn"),
        ("uvicorn", plain_python.path(), "Uvicorn"),
        ("gunicorn: maste", plain_python.path(), "Gunicorn"),
        ("python3", plain_python.path(), "Python"),
        ("gunicorn", flask.path(), "Flask"),
    ] {
        assert_eq!(
            detect_with(&mut detector, process, root),
            Some(expected),
            "{process} in {}",
            root.display()
        );
    }

    // Unknown processes still get the config labels.
    assert_eq!(text(detect_from_config(node_deno.path())), Some("Deno"));
    assert_eq!(
        text(detect_from_config(plain_python.path())),
        Some("Python")
    );
}

#[test]
fn known_runtimes_ignore_config_labels_from_other_ecosystems() {
    let next = TempDir::new().expect("temp dir");
    write_file(next.path(), "next.config.js", "");
    write_file(next.path(), "package.json", "{}");

    let go = TempDir::new().expect("temp dir");
    write_file(go.path(), "go.mod", "");

    let mut detector = StackDetector::new();
    assert_eq!(
        detect_with(&mut detector, "python", next.path()),
        Some("Python")
    );
    assert_eq!(detect_with(&mut detector, "php", next.path()), Some("PHP"));
    assert_eq!(
        detect_with(&mut detector, "deno", next.path()),
        Some("Next.js")
    );
    assert_eq!(
        detect_with(&mut detector, "node", go.path()),
        Some("Node.js")
    );
    assert_eq!(detect_with(&mut detector, "go", go.path()), Some("Go"));
}

#[test]
fn laravel_and_rails_config_detection() {
    let laravel = TempDir::new().expect("temp dir");
    write_file(laravel.path(), "composer.json", "{}");
    write_file(laravel.path(), "artisan", "");
    let laravel_label = detect_from_config(laravel.path()).expect("laravel");
    assert_eq!(laravel_label, "Laravel");
    assert_eq!(laravel_label.kind(), StackKind::Framework);

    let plain_php = TempDir::new().expect("temp dir");
    write_file(plain_php.path(), "composer.json", "{}");
    assert_eq!(text(detect_from_config(plain_php.path())), Some("PHP"));

    let rails = TempDir::new().expect("temp dir");
    for file in ["Gemfile", "config.ru", "bin/rails"] {
        write_file(rails.path(), file, "");
    }
    let rails_label = detect_from_config(rails.path()).expect("rails");
    assert_eq!(rails_label, "Rails");
    assert_eq!(rails_label.kind(), StackKind::Framework);

    let gemfile_only = TempDir::new().expect("temp dir");
    write_file(gemfile_only.path(), "Gemfile", "");
    assert_eq!(detect_from_config(gemfile_only.path()), None);
}

#[test]
fn stray_python_entry_file_does_not_relabel_node_projects() {
    let project = TempDir::new().expect("temp dir");
    write_file(project.path(), "package.json", "{}");
    write_file(
        project.path(),
        "server.py",
        "from flask import Flask\napp = Flask(__name__)\n",
    );

    assert_eq!(detect_from_config(project.path()), None);

    let mut detector = StackDetector::new();
    assert_eq!(
        detect_with(&mut detector, "node", project.path()),
        Some("Node.js")
    );
    assert_eq!(
        detect_with(&mut detector, "python3", project.path()),
        Some("Flask"),
        "a Python process may still use the entry file"
    );

    let python_only = TempDir::new().expect("temp dir");
    write_file(python_only.path(), "app.py", "print('hello')\n");
    assert_eq!(
        detect_with(&mut detector, "node", python_only.path()),
        Some("Node.js")
    );

    write_file(project.path(), "requirements.txt", "requests\n");
    assert_eq!(
        text(detect_from_config(project.path())),
        Some("Flask"),
        "a Python dependency file makes it a mixed project again"
    );
}

#[test]
fn config_files_with_non_utf8_text_are_still_scanned() {
    let little_endian = TempDir::new().expect("temp dir");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend("Flask==3.0.3\r\n".encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(little_endian.path().join("requirements.txt"), bytes).expect("write");
    assert_eq!(
        text(detect_from_config(little_endian.path())),
        Some("Flask")
    );

    let big_endian = TempDir::new().expect("temp dir");
    let mut bytes = vec![0xFE, 0xFF];
    bytes.extend("fastapi\n".encode_utf16().flat_map(u16::to_be_bytes));
    std::fs::write(big_endian.path().join("requirements.txt"), bytes).expect("write");
    assert_eq!(text(detect_from_config(big_endian.path())), Some("FastAPI"));

    let latin1 = TempDir::new().expect("temp dir");
    std::fs::write(
        latin1.path().join("main.py"),
        b"# caf\xe9\nfrom flask import Flask\napp = Flask(__name__)\n",
    )
    .expect("write");
    assert_eq!(text(detect_from_config(latin1.path())), Some("Flask"));

    // A two-byte character straddles the 64 KiB read cap.
    let split = TempDir::new().expect("temp dir");
    let mut contents = String::from("django==5.1\n# ");
    while contents.len() < 64 * 1024 - 1 {
        contents.push('x');
    }
    contents.push_str("\u{e9}\n");
    write_file(split.path(), "requirements.txt", &contents);
    assert_eq!(text(detect_from_config(split.path())), Some("Django"));
}

#[test]
fn new_project_markers_resolve_roots_and_config_labels() {
    for marker in ["bun.lock", "deno.jsonc", "go.work", "setup.py", "Pipfile"] {
        let project = TempDir::new().expect("temp dir");
        write_file(project.path(), marker, "");
        let nested = project.path().join("src");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        assert_eq!(
            find_project_root(&nested, None).as_deref(),
            Some(project.path()),
            "marker {marker}"
        );
    }

    for (marker, expected) in [
        ("deno.jsonc", "Deno"),
        ("go.work", "Go"),
        ("setup.py", "Python"),
        ("Pipfile", "Python"),
    ] {
        let project = TempDir::new().expect("temp dir");
        write_file(project.path(), marker, "");
        assert_eq!(
            text(detect_from_config(project.path())),
            Some(expected),
            "marker {marker}"
        );
    }
}
