//! Fixture corpus: realistic project trees run through the full detection
//! pipeline a process collector uses.
//!
//! Each case builds its files inside a fresh fake home directory, which is
//! also the detector's home ceiling, so marker files on the host (above the
//! system temp directory) cannot change a result. The case then resolves the
//! project root from the process working directory, executable path, and
//! command line, detects the stack label for that root, and compares both with
//! the expectation. Paths in a case are relative to the fake home and use `/`.
//!
//! Every case is its own test, named after its fixture, so a failure names the
//! fixture. Cases that describe accepted but not yet implemented behavior are
//! marked `#[ignore = "pending rule: ..."]`; remove the attribute when the rule
//! lands. Run them with `cargo test --test corpus -- --ignored`.

#![allow(missing_docs, reason = "integration tests document behavior via names")]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use what_stack::{ProjectInput, StackDetector, StackInput, StackLabel};

/// Files of one fixture: home-relative path and contents. A path ending in
/// `/` creates an empty directory instead of a file.
type Files = &'static [(&'static str, &'static str)];

/// A fake home directory holding one fixture's project tree.
struct Fixture {
    home: TempDir,
}

impl Fixture {
    fn build(files: Files) -> Self {
        let fixture = Self {
            home: TempDir::new().expect("create fake home"),
        };
        for (relative, contents) in files {
            if relative.ends_with('/') {
                fixture.dir(relative);
            } else {
                fixture.write(relative, contents);
            }
        }
        fixture
    }

    fn home(&self) -> &Path {
        self.home.path()
    }

    /// Absolute path of a home-relative `/`-separated path. `""` is the home.
    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .filter(|part| !part.is_empty())
            .fold(self.home().to_path_buf(), |path, part| path.join(part))
    }

    fn dir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(&path).expect("create fixture directory");
        path
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("create fixture parent directory");
        std::fs::write(&path, contents).expect("write fixture file");
        path
    }

    /// Home-relative `/`-separated form of `path`, for readable failures.
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(self.home()).map_or_else(
            |_| path.display().to_string(),
            |relative| {
                relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            },
        )
    }
}

/// One corpus case: a fixture, the process inputs, and the expected result.
#[derive(Clone, Copy)]
struct Case {
    files: Files,
    process: &'static str,
    /// Home-relative executable path; an empty file is created there.
    exe: Option<&'static str>,
    /// Home-relative working directory; created when missing.
    cwd: Option<&'static str>,
    /// Command-line arguments. `~/` arguments become absolute home paths.
    cmd: &'static [&'static str],
    label: Option<&'static str>,
    /// Home-relative expected project root.
    root: Option<&'static str>,
}

impl Case {
    const fn new(files: Files) -> Self {
        Self {
            files,
            process: "",
            exe: None,
            cwd: None,
            cmd: &[],
            label: None,
            root: None,
        }
    }

    const fn process(mut self, process: &'static str) -> Self {
        self.process = process;
        self
    }

    const fn exe(mut self, exe: &'static str) -> Self {
        self.exe = Some(exe);
        self
    }

    const fn cwd(mut self, cwd: &'static str) -> Self {
        self.cwd = Some(cwd);
        self
    }

    const fn cmd(mut self, cmd: &'static [&'static str]) -> Self {
        self.cmd = cmd;
        self
    }

    const fn expect(mut self, label: Option<&'static str>, root: Option<&'static str>) -> Self {
        self.label = label;
        self.root = root;
        self
    }

    fn check(self, name: &str) {
        let fixture = Fixture::build(self.files);
        let exe = self.exe.map(|relative| fixture.write(relative, ""));
        let cwd = self.cwd.map(|relative| fixture.dir(relative));
        let cmd: Vec<OsString> = self
            .cmd
            .iter()
            .map(|arg| {
                arg.strip_prefix("~/").map_or_else(
                    || OsString::from(arg),
                    |relative| fixture.path(relative).into_os_string(),
                )
            })
            .collect();
        let exe_name = exe
            .as_deref()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str);

        let mut detector = StackDetector::with_home(Some(fixture.home().to_path_buf()));
        let root = detector.detect_project_root(
            ProjectInput::new()
                .cwd(cwd.as_deref())
                .exe(exe.as_deref())
                .cmd(&cmd),
        );
        let label = detector.detect_stack(
            StackInput::new(self.process)
                .exe_name(exe_name)
                .exe_path(exe.as_deref())
                .project_root(root.as_deref()),
        );

        let actual = (
            label.as_ref().map(StackLabel::as_str),
            root.as_deref().map(|root| fixture.relative(root)),
        );
        let expected = (self.label, self.root.map(String::from));
        assert_eq!(
            actual, expected,
            "fixture {name}: (label, root) for process {:?}, exe {:?}, cwd {:?}, cmd {:?}",
            self.process, self.exe, self.cwd, self.cmd
        );
    }
}

/// Generate one test per case. Attributes such as `#[ignore = "..."]` go on
/// the generated test.
macro_rules! corpus {
    ($($(#[$attr:meta])* $name:ident: $case:expr;)+) => {
        $(
            #[test]
            $(#[$attr])*
            fn $name() {
                $case.check(stringify!($name));
            }
        )+
    };
}

// Runtime executables live under the fake home, outside every project, the
// way a version manager or system install keeps them.
const NODE: &str = ".runtimes/node/bin/node";
const NODE_WINDOWS: &str = ".runtimes/nodejs/node.exe";
const BUN: &str = ".bun/bin/bun";
const DENO: &str = ".deno/bin/deno";
const PYTHON: &str = ".runtimes/python/bin/python3";
const RUBY: &str = ".runtimes/ruby/bin/ruby";
const PHP: &str = ".runtimes/php/bin/php";
const PHP_FPM: &str = ".runtimes/php/sbin/php-fpm8.2";
const DOTNET: &str = ".dotnet/dotnet";
const GO: &str = ".runtimes/go/bin/go";
const CARGO: &str = ".cargo/bin/cargo";
const JAVA: &str = ".jdks/temurin-21/bin/java";
const BEAM: &str = ".asdf/installs/erlang/27.0/erts-15.0/bin/beam.smp";
const ERL_WINDOWS: &str = "scoop/apps/erlang/current/erts-15.0/bin/erl.exe";
const POSTGRES: &str = ".runtimes/postgres/bin/postgres";

// ---------------------------------------------------------------------------
// Fixtures: JavaScript and TypeScript
// ---------------------------------------------------------------------------

const NEXT_MJS: Files = &[
    (
        "web/package.json",
        r#"{
  "name": "web",
  "private": true,
  "scripts": { "dev": "next dev", "build": "next build", "start": "next start" },
  "dependencies": { "next": "14.2.3", "react": "18.3.1", "react-dom": "18.3.1" }
}
"#,
    ),
    (
        "web/next.config.mjs",
        "/** @type {import('next').NextConfig} */\nconst nextConfig = {};\n\nexport default nextConfig;\n",
    ),
    (
        "web/app/page.tsx",
        "export default function Home() {\n  return <main>Hello</main>;\n}\n",
    ),
    ("web/public/", ""),
];

const NEXT_TS: Files = &[
    (
        "web/package.json",
        r#"{ "name": "web", "dependencies": { "next": "15.0.3", "react": "19.0.0" } }
"#,
    ),
    (
        "web/next.config.ts",
        "import type { NextConfig } from \"next\";\n\nconst config: NextConfig = {};\nexport default config;\n",
    ),
    (
        "web/tsconfig.json",
        "{ \"compilerOptions\": { \"strict\": true } }\n",
    ),
    (
        "web/src/app/layout.tsx",
        "export default function Layout() {}\n",
    ),
];

const VITE_REACT: Files = &[
    (
        "dashboard/package.json",
        r#"{
  "name": "dashboard",
  "type": "module",
  "scripts": { "dev": "vite", "build": "tsc -b && vite build" },
  "dependencies": { "react": "^18.3.1", "react-dom": "^18.3.1" },
  "devDependencies": { "@vitejs/plugin-react": "^4.3.1", "vite": "^5.4.0" }
}
"#,
    ),
    (
        "dashboard/vite.config.ts",
        "import { defineConfig } from 'vite'\nimport react from '@vitejs/plugin-react'\n\nexport default defineConfig({ plugins: [react()] })\n",
    ),
    ("dashboard/index.html", "<div id=\"root\"></div>\n"),
    ("dashboard/src/main.tsx", "import React from 'react'\n"),
];

const SVELTEKIT: Files = &[
    (
        "site/package.json",
        r#"{ "name": "site", "devDependencies": { "@sveltejs/kit": "^2.5.0", "svelte": "^4.2.0", "vite": "^5.0.0" } }
"#,
    ),
    (
        "site/svelte.config.js",
        "import adapter from '@sveltejs/adapter-auto';\n\nexport default { kit: { adapter: adapter() } };\n",
    ),
    (
        "site/vite.config.ts",
        "import { sveltekit } from '@sveltejs/kit/vite';\nimport { defineConfig } from 'vite';\n\nexport default defineConfig({ plugins: [sveltekit()] });\n",
    ),
    ("site/src/routes/+page.svelte", "<h1>Hello</h1>\n"),
];

const NUXT: Files = &[
    (
        "shop/package.json",
        r#"{ "name": "shop", "scripts": { "dev": "nuxt dev" }, "dependencies": { "nuxt": "^3.12.0", "vue": "^3.4.0" } }
"#,
    ),
    (
        "shop/nuxt.config.ts",
        "export default defineNuxtConfig({ devtools: { enabled: true } })\n",
    ),
    ("shop/app.vue", "<template><NuxtPage /></template>\n"),
];

const ASTRO: Files = &[
    (
        "blog/package.json",
        r#"{ "name": "blog", "type": "module", "dependencies": { "astro": "^4.11.0" } }
"#,
    ),
    (
        "blog/astro.config.mjs",
        "import { defineConfig } from 'astro/config';\n\nexport default defineConfig({});\n",
    ),
    ("blog/src/pages/index.astro", "---\n---\n<h1>Blog</h1>\n"),
];

const ANGULAR: Files = &[
    (
        "admin/package.json",
        r#"{ "name": "admin", "scripts": { "start": "ng serve" }, "dependencies": { "@angular/core": "^18.0.0" }, "devDependencies": { "@angular/cli": "^18.0.0" } }
"#,
    ),
    (
        "admin/angular.json",
        r#"{ "version": 1, "projects": { "admin": { "projectType": "application" } } }
"#,
    ),
    ("admin/tsconfig.json", "{}\n"),
    ("admin/src/main.ts", "bootstrapApplication(AppComponent);\n"),
];

const PLAIN_NODE: Files = &[
    (
        "worker/package.json",
        r#"{ "name": "worker", "main": "index.js", "dependencies": { "dotenv": "^16.4.5", "pg": "^8.12.0" } }
"#,
    ),
    (
        "worker/index.js",
        "require('dotenv').config();\nconsole.log('working');\n",
    ),
    ("worker/package-lock.json", "{ \"lockfileVersion\": 3 }\n"),
];

const BUN_APP: Files = &[
    (
        "edge/package.json",
        r#"{ "name": "edge", "module": "index.ts", "type": "module", "devDependencies": { "@types/bun": "latest" } }
"#,
    ),
    (
        "edge/bun.lock",
        "{\n  \"lockfileVersion\": 1,\n  \"workspaces\": {}\n}\n",
    ),
    (
        "edge/index.ts",
        "Bun.serve({ port: 3000, fetch: () => new Response('ok') });\n",
    ),
];

const DENO_JSON: Files = &[
    (
        "fresh/deno.json",
        r#"{ "tasks": { "dev": "deno run --watch main.ts" }, "imports": { "@std/http": "jsr:@std/http@^1.0.0" } }
"#,
    ),
    ("fresh/main.ts", "Deno.serve(() => new Response('ok'));\n"),
];

const DENO_JSONC: Files = &[
    (
        "tools/deno.jsonc",
        "{\n  // Tasks for local development.\n  \"tasks\": { \"dev\": \"deno run -A main.ts\" }\n}\n",
    ),
    ("tools/main.ts", "console.log('tool');\n"),
];

const PNPM_TURBO: Files = &[
    (
        "mono/package.json",
        r#"{ "name": "mono", "private": true, "packageManager": "pnpm@9.4.0", "devDependencies": { "turbo": "^2.0.0" } }
"#,
    ),
    (
        "mono/pnpm-workspace.yaml",
        "packages:\n  - \"apps/*\"\n  - \"packages/*\"\n",
    ),
    (
        "mono/turbo.json",
        "{ \"tasks\": { \"dev\": { \"cache\": false } } }\n",
    ),
    (
        "mono/apps/web/package.json",
        r#"{ "name": "@mono/web", "dependencies": { "next": "14.2.3", "@mono/ui": "workspace:*" } }
"#,
    ),
    ("mono/apps/web/next.config.js", "module.exports = {};\n"),
    (
        "mono/packages/ui/package.json",
        r#"{ "name": "@mono/ui" }
"#,
    ),
];

const HOME_NESTED: Files = &[
    // A stray manifest directly in the home directory, as left by a global
    // `npm install` without `-g`.
    (
        "package.json",
        r#"{ "dependencies": { "left-pad": "^1.3.0" } }
"#,
    ),
    (
        "projects/app/package.json",
        r#"{ "name": "app", "dependencies": { "next": "14.2.3" } }
"#,
    ),
    ("projects/app/next.config.mjs", "export default {};\n"),
    ("projects/app/src/", ""),
    ("scratch/notes.txt", "todo\n"),
];

// ---------------------------------------------------------------------------
// Fixtures: Python
// ---------------------------------------------------------------------------

const DJANGO: Files = &[
    (
        "mysite/manage.py",
        "#!/usr/bin/env python\nimport os\nimport sys\n\nif __name__ == \"__main__\":\n    os.environ.setdefault(\"DJANGO_SETTINGS_MODULE\", \"mysite.settings\")\n    from django.core.management import execute_from_command_line\n    execute_from_command_line(sys.argv)\n",
    ),
    (
        "mysite/requirements.txt",
        "Django==5.0.6\npsycopg[binary]==3.1.19\ngunicorn==22.0.0\n",
    ),
    ("mysite/mysite/__init__.py", ""),
    (
        "mysite/mysite/settings.py",
        "DEBUG = True\nINSTALLED_APPS = []\n",
    ),
    (
        "mysite/mysite/wsgi.py",
        "import os\nfrom django.core.wsgi import get_wsgi_application\n\nos.environ.setdefault(\"DJANGO_SETTINGS_MODULE\", \"mysite.settings\")\napplication = get_wsgi_application()\n",
    ),
];

/// Django project with its virtual environment inside the project, as
/// `python -m venv .venv` or `uv venv` creates it. Both the Unix (`bin`) and
/// the Windows (`Scripts`) layouts are present.
const DJANGO_VENV: Files = &[
    (
        "shop/pyproject.toml",
        "[project]
name = \"shop\"
dependencies = [\"django>=5.0\"]
",
    ),
    (
        "shop/manage.py",
        "import sys
from django.core.management import execute_from_command_line

execute_from_command_line(sys.argv)
",
    ),
    (
        "shop/.venv/pyvenv.cfg",
        "home = /usr/bin
include-system-site-packages = false
version = 3.12.3
",
    ),
];

/// Flask project whose virtual environment was created in place (`python -m
/// venv .` inside the project), below a parent directory that has its own
/// `package.json`. The walk must stop at the project, not the parent.
const FLASK_IN_PLACE_VENV: Files = &[
    ("workspace/package.json", "{\"name\": \"workspace\"}\n"),
    (
        "workspace/notes/pyvenv.cfg",
        "home = /usr/bin
version = 3.12.3
",
    ),
    (
        "workspace/notes/requirements.txt",
        "flask==3.0.0
",
    ),
    (
        "workspace/notes/app.py",
        "from flask import Flask

app = Flask(__name__)
",
    ),
];

/// The same project with a conda environment created by `conda create -p
/// ./env`, which writes no `pyvenv.cfg`.
const DJANGO_CONDA_ENV: Files = &[
    (
        "shop/pyproject.toml",
        "[project]
name = \"shop\"
dependencies = [\"django>=5.0\"]
",
    ),
    (
        "shop/manage.py",
        "import sys
from django.core.management import execute_from_command_line

execute_from_command_line(sys.argv)
",
    ),
    ("shop/env/conda-meta/history", ""),
];

/// virtualenvwrapper keeps environments under `~/.virtualenvs`; a stray
/// requirements file there must not become the project of every environment.
const VIRTUALENVWRAPPER: Files = &[
    (
        ".virtualenvs/requirements.txt",
        "flask
",
    ),
    (
        ".virtualenvs/shop/pyvenv.cfg",
        "home = /usr/bin
version = 3.12.3
",
    ),
];

/// Django project nested below the repository root, with the dependency file
/// only at the root.
const DJANGO_NESTED_SRC: Files = &[
    (
        "portal/requirements.txt",
        "django>=5.0,<5.1\ndjango-environ\n",
    ),
    ("portal/README.md", "# Portal\n"),
    (
        "portal/src/mysite/manage.py",
        "import os\nimport sys\n\nos.environ.setdefault(\"DJANGO_SETTINGS_MODULE\", \"mysite.settings\")\n",
    ),
    ("portal/src/mysite/mysite/settings.py", "DEBUG = False\n"),
];

const FLASK: Files = &[
    (
        "notes/app.py",
        "from flask import Flask\n\napp = Flask(__name__)\n\n\n@app.get(\"/\")\ndef index():\n    return \"ok\"\n",
    ),
    ("notes/requirements.txt", "Flask==3.0.3\n"),
];

/// Flask application factory: the entry file never calls `Flask(` itself.
const FLASK_FACTORY: Files = &[
    (
        "ledger/requirements.txt",
        "Flask==3.0.3\npython-dotenv==1.0.1\n",
    ),
    (
        "ledger/wsgi.py",
        "from app import create_app\n\napp = create_app()\n",
    ),
    (
        "ledger/app/__init__.py",
        "from flask import Flask\n\n\ndef create_app():\n    app = Flask(__name__)\n    return app\n",
    ),
];

const FASTAPI_ROOT: Files = &[
    (
        "api/main.py",
        "from fastapi import FastAPI\n\napp = FastAPI()\n\n\n@app.get(\"/health\")\ndef health():\n    return {\"ok\": True}\n",
    ),
    (
        "api/requirements.txt",
        "fastapi==0.111.0\nuvicorn[standard]==0.30.1\n",
    ),
];

const FASTAPI_APP_LAYOUT: Files = &[
    (
        "orders/pyproject.toml",
        "[project]\nname = \"orders\"\nversion = \"0.1.0\"\nrequires-python = \">=3.11\"\ndependencies = [\n    \"fastapi>=0.110\",\n    \"uvicorn[standard]>=0.29\",\n]\n",
    ),
    ("orders/app/__init__.py", ""),
    (
        "orders/app/main.py",
        "from fastapi import FastAPI\n\napp = FastAPI()\n",
    ),
];

const POETRY: Files = &[
    (
        "billing/pyproject.toml",
        "[tool.poetry]\nname = \"billing\"\nversion = \"0.1.0\"\ndescription = \"\"\nauthors = [\"Dev <dev@example.com>\"]\n\n[tool.poetry.dependencies]\npython = \"^3.12\"\nfastapi = \"^0.111.0\"\n\n[build-system]\nrequires = [\"poetry-core\"]\nbuild-backend = \"poetry.core.masonry.api\"\n",
    ),
    (
        "billing/poetry.lock",
        "# This file is automatically @generated by Poetry 1.8.3 and should not be changed by hand.\n\n[[package]]\nname = \"fastapi\"\nversion = \"0.111.0\"\n",
    ),
    ("billing/billing/__init__.py", ""),
];

const UV: Files = &[
    (
        "events/pyproject.toml",
        "[project]\nname = \"events\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\ndependencies = [\"litestar>=2.9\"]\n",
    ),
    (
        "events/uv.lock",
        "version = 1\nrequires-python = \">=3.12\"\n\n[[package]]\nname = \"litestar\"\nversion = \"2.9.1\"\n\n[[package]]\nname = \"anyio\"\nversion = \"4.4.0\"\n",
    ),
    ("events/.python-version", "3.12\n"),
    ("events/src/events/__init__.py", ""),
];

const PLAIN_PYTHON: Files = &[
    (
        "scraper/requirements.txt",
        "requests==2.32.3\nrich==13.7.1\n",
    ),
    (
        "scraper/main.py",
        "import requests\n\nprint(requests.get(\"https://example.com\").status_code)\n",
    ),
];

// ---------------------------------------------------------------------------
// Fixtures: Ruby and PHP
// ---------------------------------------------------------------------------

const RAILS: Files = &[
    (
        "store/Gemfile",
        "source \"https://rubygems.org\"\n\ngem \"rails\", \"~> 7.1.3\"\ngem \"puma\", \">= 5.0\"\n",
    ),
    ("store/Gemfile.lock", "GEM\n  specs:\n    rails (7.1.3)\n"),
    (
        "store/config.ru",
        "require_relative \"config/environment\"\n\nrun Rails.application\n",
    ),
    (
        "store/bin/rails",
        "#!/usr/bin/env ruby\nrequire_relative \"../config/boot\"\n",
    ),
    (
        "store/config/application.rb",
        "module Store\n  class Application < Rails::Application\n  end\nend\n",
    ),
];

const SINATRA: Files = &[
    (
        "hooks/Gemfile",
        "source \"https://rubygems.org\"\n\ngem \"sinatra\"\ngem \"puma\"\n",
    ),
    (
        "hooks/config.ru",
        "require \"./app\"\nrun Sinatra::Application\n",
    ),
    (
        "hooks/app.rb",
        "require \"sinatra\"\n\nget \"/\" do\n  \"ok\"\nend\n",
    ),
];

const LARAVEL: Files = &[
    (
        "crm/composer.json",
        r#"{ "name": "acme/crm", "require": { "php": "^8.2", "laravel/framework": "^11.0" } }
"#,
    ),
    ("crm/artisan", "#!/usr/bin/env php\n<?php\n"),
    (
        "crm/package.json",
        r#"{ "private": true, "type": "module", "scripts": { "dev": "vite" }, "devDependencies": { "laravel-vite-plugin": "^1.0", "vite": "^5.0" } }
"#,
    ),
    (
        "crm/vite.config.js",
        "import { defineConfig } from 'vite';\nimport laravel from 'laravel-vite-plugin';\n\nexport default defineConfig({ plugins: [laravel({ input: ['resources/js/app.js'] })] });\n",
    ),
    ("crm/public/index.php", "<?php\n"),
];

// ---------------------------------------------------------------------------
// Fixtures: compiled ecosystems
// ---------------------------------------------------------------------------

const DOTNET_CSHARP: Files = &[
    (
        "payments/Payments.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk.Web\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    ),
    (
        "payments/Program.cs",
        "var app = WebApplication.Create(args);\napp.Run();\n",
    ),
];

const DOTNET_FSHARP: Files = &[
    (
        "pricing/Pricing.fsproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <ItemGroup>\n    <Compile Include=\"Program.fs\" />\n  </ItemGroup>\n</Project>\n",
    ),
    ("pricing/Program.fs", "printfn \"pricing\"\n"),
];

const GO_SERVICE: Files = &[
    (
        "inventory/go.mod",
        "module example.com/inventory\n\ngo 1.22\n",
    ),
    ("inventory/go.sum", ""),
    ("inventory/main.go", "package main\n\nfunc main() {}\n"),
];

const GO_WORK: Files = &[
    (
        "platform/go.work",
        "go 1.22\n\nuse (\n\t./api\n\t./worker\n)\n",
    ),
    (
        "platform/api/go.mod",
        "module example.com/platform/api\n\ngo 1.22\n",
    ),
    ("platform/api/main.go", "package main\n\nfunc main() {}\n"),
    (
        "platform/worker/go.mod",
        "module example.com/platform/worker\n\ngo 1.22\n",
    ),
];

const CARGO_WORKSPACE: Files = &[
    (
        "engine/Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n",
    ),
    ("engine/Cargo.lock", "version = 3\n"),
    (
        "engine/crates/server/Cargo.toml",
        "[package]\nname = \"server\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    ),
    ("engine/crates/server/src/main.rs", "fn main() {}\n"),
    (
        "engine/crates/core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    ),
];

// ---------------------------------------------------------------------------
// Fixtures for pending rules
// ---------------------------------------------------------------------------

const NEXT_WITHOUT_CONFIG: Files = &[
    (
        "landing/package.json",
        r#"{ "name": "landing", "scripts": { "dev": "next dev" }, "dependencies": { "next": "15.0.3", "react": "19.0.0", "react-dom": "19.0.0" } }
"#,
    ),
    (
        "landing/app/page.tsx",
        "export default function Page() {}\n",
    ),
];

const REMIX: Files = &[
    (
        "remix-app/package.json",
        r#"{ "name": "remix-app", "type": "module", "dependencies": { "@remix-run/node": "^2.9.2", "@remix-run/react": "^2.9.2", "@remix-run/serve": "^2.9.2" }, "devDependencies": { "@remix-run/dev": "^2.9.2", "vite": "^5.1.0" } }
"#,
    ),
    (
        "remix-app/vite.config.ts",
        "import { vitePlugin as remix } from \"@remix-run/dev\";\nimport { defineConfig } from \"vite\";\n\nexport default defineConfig({ plugins: [remix()] });\n",
    ),
    (
        "remix-app/app/root.tsx",
        "export default function App() {}\n",
    ),
];

const REACT_ROUTER: Files = &[
    (
        "rr-app/package.json",
        r#"{ "name": "rr-app", "type": "module", "dependencies": { "react-router": "^7.0.0", "@react-router/node": "^7.0.0" }, "devDependencies": { "@react-router/dev": "^7.0.0", "vite": "^5.4.0" } }
"#,
    ),
    (
        "rr-app/react-router.config.ts",
        "import type { Config } from \"@react-router/dev/config\";\n\nexport default { ssr: true } satisfies Config;\n",
    ),
    (
        "rr-app/vite.config.ts",
        "import { reactRouter } from \"@react-router/dev/vite\";\nimport { defineConfig } from \"vite\";\n\nexport default defineConfig({ plugins: [reactRouter()] });\n",
    ),
    ("rr-app/app/root.tsx", "export default function App() {}\n"),
];

const NESTJS: Files = &[
    (
        "gateway/package.json",
        r#"{ "name": "gateway", "scripts": { "start": "nest start" }, "dependencies": { "@nestjs/common": "^10.3.0", "@nestjs/core": "^10.3.0", "@nestjs/platform-express": "^10.3.0", "reflect-metadata": "^0.2.0", "rxjs": "^7.8.1" }, "devDependencies": { "@nestjs/cli": "^10.3.0" } }
"#,
    ),
    (
        "gateway/nest-cli.json",
        r#"{ "collection": "@nestjs/schematics", "sourceRoot": "src" }
"#,
    ),
    ("gateway/tsconfig.json", "{}\n"),
    (
        "gateway/src/main.ts",
        "import { NestFactory } from '@nestjs/core';\n",
    ),
    ("gateway/dist/main.js", "\"use strict\";\n"),
];

const EXPRESS: Files = &[
    (
        "legacy-api/package.json",
        r#"{ "name": "legacy-api", "main": "server.js", "dependencies": { "express": "^4.19.2", "cors": "^2.8.5" } }
"#,
    ),
    (
        "legacy-api/server.js",
        "const express = require('express');\nconst app = express();\napp.listen(3000);\n",
    ),
];

/// `uv.lock` lists every transitive package; `starlette` here comes from
/// `mcp`, not from a direct dependency.
const UV_TRANSITIVE_STARLETTE: Files = &[
    (
        "agent/pyproject.toml",
        "[project]\nname = \"agent\"\nversion = \"0.1.0\"\nrequires-python = \">=3.11\"\ndependencies = [\"mcp>=1.2.0\", \"httpx>=0.27\"]\n",
    ),
    (
        "agent/uv.lock",
        "version = 1\n\n[[package]]\nname = \"mcp\"\nversion = \"1.2.0\"\ndependencies = [\n    { name = \"httpx\" },\n    { name = \"starlette\" },\n]\n\n[[package]]\nname = \"starlette\"\nversion = \"0.41.3\"\n",
    ),
    ("agent/main.py", "import httpx\n"),
];

const PYPROJECT_COMMENT: Files = &[
    (
        "crawler/pyproject.toml",
        "[project]\nname = \"crawler\"\nversion = \"0.1.0\"\n# We tried flask for a status page but dropped it.\ndependencies = [\"requests>=2.32\", \"beautifulsoup4\"]\n",
    ),
    ("crawler/crawler/__init__.py", ""),
];

const SYMFONY: Files = &[
    (
        "tickets/composer.json",
        r#"{ "type": "project", "require": { "php": ">=8.2", "symfony/console": "7.1.*", "symfony/framework-bundle": "7.1.*", "symfony/runtime": "7.1.*" } }
"#,
    ),
    ("tickets/symfony.lock", "{}\n"),
    ("tickets/bin/console", "#!/usr/bin/env php\n<?php\n"),
    ("tickets/config/bundles.php", "<?php\nreturn [];\n"),
    ("tickets/public/index.php", "<?php\n"),
];

const SPRING_MAVEN: Files = &[
    (
        "accounts/pom.xml",
        "<project>\n  <modelVersion>4.0.0</modelVersion>\n  <parent>\n    <groupId>org.springframework.boot</groupId>\n    <artifactId>spring-boot-starter-parent</artifactId>\n    <version>3.3.0</version>\n  </parent>\n  <artifactId>accounts</artifactId>\n  <dependencies>\n    <dependency>\n      <groupId>org.springframework.boot</groupId>\n      <artifactId>spring-boot-starter-web</artifactId>\n    </dependency>\n  </dependencies>\n</project>\n",
    ),
    (
        "accounts/src/main/java/com/example/accounts/AccountsApplication.java",
        "@SpringBootApplication\npublic class AccountsApplication {}\n",
    ),
    ("accounts/target/accounts-0.0.1-SNAPSHOT.jar", ""),
];

const SPRING_GRADLE_KTS: Files = &[
    (
        "catalog/build.gradle.kts",
        "plugins {\n    id(\"org.springframework.boot\") version \"3.3.0\"\n    id(\"io.spring.dependency-management\") version \"1.1.5\"\n    kotlin(\"jvm\") version \"1.9.24\"\n}\n\ndependencies {\n    implementation(\"org.springframework.boot:spring-boot-starter-web\")\n}\n",
    ),
    (
        "catalog/settings.gradle.kts",
        "rootProject.name = \"catalog\"\n",
    ),
    (
        "catalog/src/main/kotlin/com/example/catalog/CatalogApplication.kt",
        "@SpringBootApplication\nclass CatalogApplication\n",
    ),
];

const GRADLE_MULTI_MODULE: Files = &[
    (
        "suite/settings.gradle.kts",
        "rootProject.name = \"suite\"\ninclude(\":app\", \":lib\")\n",
    ),
    ("suite/build.gradle.kts", "plugins {\n    base\n}\n"),
    (
        "suite/app/build.gradle.kts",
        "plugins {\n    application\n}\n\ndependencies {\n    implementation(project(\":lib\"))\n}\n",
    ),
    ("suite/app/src/main/java/App.java", "public class App {}\n"),
    (
        "suite/lib/build.gradle.kts",
        "plugins {\n    `java-library`\n}\n",
    ),
];

const DOTNET_SOLUTION: Files = &[
    (
        "erp/Erp.sln",
        "Microsoft Visual Studio Solution File, Format Version 12.00\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Api\", \"src\\Api\\Api.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\nEndProject\n",
    ),
    (
        "erp/src/Api/Api.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk.Web\" />\n",
    ),
    (
        "erp/src/Domain/Domain.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
    ),
];

/// A Go service with an embedded Vite frontend: `go build -o app` at the root.
const GO_VITE: Files = &[
    ("console/go.mod", "module example.com/console\n\ngo 1.22\n"),
    (
        "console/main.go",
        "package main\n\nimport \"embed\"\n\n//go:embed web/dist\nvar dist embed.FS\n\nfunc main() {}\n",
    ),
    (
        "console/package.json",
        r#"{ "private": true, "scripts": { "build": "vite build" }, "devDependencies": { "vite": "^5.4.0" } }
"#,
    ),
    ("console/vite.config.ts", "export default {};\n"),
    ("console/web/src/main.ts", "console.log('ui');\n"),
];

const PHOENIX: Files = &[
    (
        "chat/mix.exs",
        "defmodule Chat.MixProject do\n  use Mix.Project\n\n  def project do\n    [app: :chat, version: \"0.1.0\", elixir: \"~> 1.14\", deps: deps()]\n  end\n\n  defp deps do\n    [\n      {:phoenix, \"~> 1.7.14\"},\n      {:phoenix_live_view, \"~> 0.20.17\"},\n      {:bandit, \"~> 1.5\"}\n    ]\n  end\nend\n",
    ),
    ("chat/mix.lock", "%{}\n"),
    ("chat/config/config.exs", "import Config\n"),
    (
        "chat/lib/chat_web/endpoint.ex",
        "defmodule ChatWeb.Endpoint do\nend\n",
    ),
];

/// nvm is a git checkout with its own `package.json` around every Node.js it
/// installs.
const NVM: Files = &[
    (
        ".nvm/package.json",
        r#"{ "name": "nvm", "version": "0.39.7", "private": true }
"#,
    ),
    (".nvm/nvm.sh", "# nvm\n"),
    (".nvm/versions/node/v20.11.1/bin/node", ""),
];

// ---------------------------------------------------------------------------
// Cases that pass today
// ---------------------------------------------------------------------------

corpus! {
    next_mjs_config_node: Case::new(NEXT_MJS)
        .process("node").exe(NODE).cwd("web").cmd(&["~/web/node_modules/.bin/next", "dev"])
        .expect(Some("Next.js"), Some("web"));
    next_ts_config_nested_cwd: Case::new(NEXT_TS)
        .process("node").exe(NODE).cwd("web/src/app")
        .expect(Some("Next.js"), Some("web"));
    next_server_title: Case::new(NEXT_MJS)
        .process("next-server (v1").exe(NODE).cwd("web")
        .expect(Some("Next.js"), Some("web"));
    next_windows_node_exe: Case::new(NEXT_MJS)
        .process("node.exe").exe(NODE_WINDOWS).cwd("web")
        .expect(Some("Next.js"), Some("web"));
    vite_react: Case::new(VITE_REACT)
        .process("node").exe(NODE).cwd("dashboard")
        .expect(Some("Vite"), Some("dashboard"));
    sveltekit_over_vite: Case::new(SVELTEKIT)
        .process("node").exe(NODE).cwd("site")
        .expect(Some("SvelteKit"), Some("site"));
    nuxt: Case::new(NUXT)
        .process("node").exe(NODE).cwd("shop")
        .expect(Some("Nuxt"), Some("shop"));
    astro: Case::new(ASTRO)
        .process("node").exe(NODE).cwd("blog")
        .expect(Some("Astro"), Some("blog"));
    angular_ng_serve_via_exe_name: Case::new(ANGULAR)
        .process("ng serve").exe(NODE).cwd("admin")
        .expect(Some("Angular"), Some("admin"));
    plain_node: Case::new(PLAIN_NODE)
        .process("node").exe(NODE).cwd("worker").cmd(&["index.js"])
        .expect(Some("Node.js"), Some("worker"));
    plain_node_versioned_name: Case::new(PLAIN_NODE)
        .process("node18").exe(NODE).cwd("worker")
        .expect(Some("Node.js"), Some("worker"));
    bun: Case::new(BUN_APP)
        .process("bun").exe(BUN).cwd("edge").cmd(&["run", "index.ts"])
        .expect(Some("Bun"), Some("edge"));
    deno_json: Case::new(DENO_JSON)
        .process("deno").exe(DENO).cwd("fresh").cmd(&["run", "--watch", "main.ts"])
        .expect(Some("Deno"), Some("fresh"));
    deno_jsonc: Case::new(DENO_JSONC)
        .process("deno").exe(DENO).cwd("tools")
        .expect(Some("Deno"), Some("tools"));
    pnpm_turbo_web_app: Case::new(PNPM_TURBO)
        .process("node").exe(NODE).cwd("mono/apps/web")
        .expect(Some("Next.js"), Some("mono/apps/web"));
    home_nested_project: Case::new(HOME_NESTED)
        .process("node").exe(NODE).cwd("projects/app/src")
        .expect(Some("Next.js"), Some("projects/app"));
    home_stray_marker_does_not_claim_unrelated_dir: Case::new(HOME_NESTED)
        .process("node").exe(NODE).cwd("scratch")
        .expect(Some("Node.js"), None);

    django_python: Case::new(DJANGO)
        .process("python").exe(PYTHON).cwd("mysite").cmd(&["manage.py", "runserver"])
        .expect(Some("Django"), Some("mysite"));
    django_python_minor_version: Case::new(DJANGO)
        .process("python3.12").exe(PYTHON).cwd("mysite")
        .expect(Some("Django"), Some("mysite"));
    django_gunicorn_title: Case::new(DJANGO)
        .process("gunicorn: maste").exe(PYTHON).cwd("mysite")
        .cmd(&["~/mysite/.venv/bin/gunicorn", "mysite.wsgi"])
        .expect(Some("Django"), Some("mysite"));
    django_venv_python_without_cwd: Case::new(DJANGO_VENV)
        .process("python").exe("shop/.venv/bin/python")
        .expect(Some("Django"), Some("shop"));
    django_venv_windows_python_without_cwd: Case::new(DJANGO_VENV)
        .process("python.exe").exe("shop/.venv/Scripts/python.exe")
        .expect(Some("Django"), Some("shop"));
    flask_in_place_venv_python_without_cwd: Case::new(FLASK_IN_PLACE_VENV)
        .process("python").exe("workspace/notes/bin/python")
        .expect(Some("Flask"), Some("workspace/notes"));
    flask_in_place_venv_windows_python_without_cwd: Case::new(FLASK_IN_PLACE_VENV)
        .process("python.exe").exe("workspace/notes/Scripts/python.exe")
        .expect(Some("Flask"), Some("workspace/notes"));
    django_conda_env_python_without_cwd: Case::new(DJANGO_CONDA_ENV)
        .process("python").exe("shop/env/bin/python")
        .expect(Some("Python"), None);
    virtualenvwrapper_python_has_no_project: Case::new(VIRTUALENVWRAPPER)
        .process("python").exe(".virtualenvs/shop/bin/python")
        .expect(Some("Python"), None);
    django_nested_src_mysite: Case::new(DJANGO_NESTED_SRC)
        .process("python3").exe(PYTHON).cwd("portal/src/mysite")
        .expect(Some("Django"), Some("portal"));
    flask: Case::new(FLASK)
        .process("python").exe(PYTHON).cwd("notes").cmd(&["app.py"])
        .expect(Some("Flask"), Some("notes"));
    flask_factory_patch_version: Case::new(FLASK_FACTORY)
        .process("python3.12.1").exe(PYTHON).cwd("ledger")
        .expect(Some("Flask"), Some("ledger"));
    fastapi_root_layout: Case::new(FASTAPI_ROOT)
        .process("python3").exe(PYTHON).cwd("api").cmd(&["main.py"])
        .expect(Some("FastAPI"), Some("api"));
    fastapi_app_layout_uvicorn: Case::new(FASTAPI_APP_LAYOUT)
        .process("uvicorn").exe(PYTHON).cwd("orders").cmd(&["app.main:app", "--reload"])
        .expect(Some("FastAPI"), Some("orders"));
    fastapi_app_layout_python_m_uvicorn: Case::new(FASTAPI_APP_LAYOUT)
        .process("python3").exe(PYTHON).cwd("orders").cmd(&["-m", "uvicorn", "app.main:app"])
        .expect(Some("FastAPI"), Some("orders"));
    poetry: Case::new(POETRY)
        .process("python").exe(PYTHON).cwd("billing")
        .expect(Some("FastAPI"), Some("billing"));
    uv: Case::new(UV)
        .process("python").exe(PYTHON).cwd("events/src")
        .expect(Some("Litestar"), Some("events"));
    plain_python: Case::new(PLAIN_PYTHON)
        .process("python3").exe(PYTHON).cwd("scraper").cmd(&["main.py"])
        .expect(Some("Python"), Some("scraper"));
    gunicorn_without_framework: Case::new(PLAIN_PYTHON)
        .process("gunicorn").exe(PYTHON).cwd("scraper")
        .expect(Some("Gunicorn"), Some("scraper"));

    rails_ruby: Case::new(RAILS)
        .process("ruby").exe(RUBY).cwd("store").cmd(&["bin/rails", "server"])
        .expect(Some("Rails"), Some("store"));
    rails_ruby_versioned_name: Case::new(RAILS)
        .process("ruby3.2").exe(RUBY).cwd("store")
        .expect(Some("Rails"), Some("store"));
    rails_puma_title: Case::new(RAILS)
        .process("puma 6.4.2 (tc").exe(RUBY).cwd("store")
        .expect(Some("Rails"), Some("store"));
    rails_puma_cluster_worker: Case::new(RAILS)
        .process("puma: cluster w").exe(RUBY).cwd("store")
        .expect(Some("Rails"), Some("store"));
    sinatra_rack: Case::new(SINATRA)
        .process("ruby").exe(RUBY).cwd("hooks").cmd(&["app.rb"])
        .expect(Some("Ruby (Rack)"), Some("hooks"));
    laravel_php: Case::new(LARAVEL)
        .process("php").exe(PHP).cwd("crm").cmd(&["artisan", "serve"])
        .expect(Some("Laravel"), Some("crm"));
    laravel_php_fpm: Case::new(LARAVEL)
        .process("php-fpm8.2").exe(PHP_FPM).cwd("crm/public")
        .expect(Some("Laravel"), Some("crm"));
    laravel_node_is_vite: Case::new(LARAVEL)
        .process("node").exe(NODE).cwd("crm")
        .expect(Some("Vite"), Some("crm"));

    dotnet_csharp: Case::new(DOTNET_CSHARP)
        .process("dotnet").exe(DOTNET).cwd("payments").cmd(&["run"])
        .expect(Some(".NET"), Some("payments"));
    dotnet_fsharp: Case::new(DOTNET_FSHARP)
        .process("dotnet").exe(DOTNET).cwd("pricing")
        .expect(Some(".NET (F#)"), Some("pricing"));
    go_built_binary: Case::new(GO_SERVICE)
        .process("inventory").exe("inventory/bin/inventory")
        .expect(Some("Go"), Some("inventory"));
    go_run_from_module: Case::new(GO_SERVICE)
        .process("go").exe(GO).cwd("inventory").cmd(&["run", "."])
        .expect(Some("Go"), Some("inventory"));
    go_work_root: Case::new(GO_WORK)
        .process("go").exe(GO).cwd("platform")
        .expect(Some("Go"), Some("platform"));
    cargo_workspace_root: Case::new(CARGO_WORKSPACE)
        .process("cargo").exe(CARGO).cwd("engine").cmd(&["run", "-p", "server"])
        .expect(Some("Rust"), Some("engine"));
    database_in_project_keeps_its_label: Case::new(NEXT_MJS)
        .process("postgres").exe(POSTGRES).cwd("web")
        .expect(Some("PostgreSQL"), Some("web"));
}

// ---------------------------------------------------------------------------
// Cases pending a detection rule or fix
// ---------------------------------------------------------------------------

corpus! {
    next_package_json_without_config: Case::new(NEXT_WITHOUT_CONFIG)
        .process("node").exe(NODE).cwd("landing")
        .expect(Some("Next.js"), Some("landing"));
    remix_vite: Case::new(REMIX)
        .process("node").exe(NODE).cwd("remix-app")
        .expect(Some("Remix"), Some("remix-app"));
    react_router_framework: Case::new(REACT_ROUTER)
        .process("node").exe(NODE).cwd("rr-app")
        .expect(Some("React Router"), Some("rr-app"));
    nestjs: Case::new(NESTJS)
        .process("node").exe(NODE).cwd("gateway").cmd(&["~/gateway/dist/main.js"])
        .expect(Some("NestJS"), Some("gateway"));
    express: Case::new(EXPRESS)
        .process("node").exe(NODE).cwd("legacy-api").cmd(&["server.js"])
        .expect(Some("Express"), Some("legacy-api"));
    uv_lock_transitive_starlette: Case::new(UV_TRANSITIVE_STARLETTE)
        .process("python3").exe(PYTHON).cwd("agent")
        .expect(Some("Python"), Some("agent"));
    pyproject_comment_mentions_framework: Case::new(PYPROJECT_COMMENT)
        .process("python3").exe(PYTHON).cwd("crawler")
        .expect(Some("Python"), Some("crawler"));
    symfony: Case::new(SYMFONY)
        .process("php").exe(PHP).cwd("tickets").cmd(&["bin/console", "server:run"])
        .expect(Some("Symfony"), Some("tickets"));
    spring_boot_maven: Case::new(SPRING_MAVEN)
        .process("java").exe(JAVA).cwd("accounts")
        .cmd(&["-jar", "~/accounts/target/accounts-0.0.1-SNAPSHOT.jar"])
        .expect(Some("Spring Boot"), Some("accounts"));
    spring_boot_gradle_kts: Case::new(SPRING_GRADLE_KTS)
        .process("java").exe(JAVA).cwd("catalog")
        .expect(Some("Spring Boot"), Some("catalog"));
    // The nearest build file wins, as for Cargo workspace members and .NET
    // projects in a solution: a process run inside a module belongs to that
    // module. settings.gradle only marks a root that has no build script.
    gradle_module_in_multi_module_build: Case::new(GRADLE_MULTI_MODULE)
        .process("java").exe(JAVA).cwd("suite/app/src/main")
        .expect(Some("Kotlin (Gradle)"), Some("suite/app"));
    dotnet_solution_root: Case::new(DOTNET_SOLUTION)
        .process("dotnet").exe(DOTNET).cwd("erp").cmd(&["run", "--project", "src/Api"])
        .expect(Some(".NET"), Some("erp"));
    go_run_temp_executable: Case::new(GO_SERVICE)
        .process("main").exe("tmp/go-build2734481/b001/exe/main").cwd("inventory")
        .expect(Some("Go"), Some("inventory"));
    go_vite_repository_binary: Case::new(GO_VITE)
        .process("app").exe("console/app").cwd("console")
        .expect(Some("Go"), Some("console"));
    cargo_member_run_from_crate_dir: Case::new(CARGO_WORKSPACE)
        .process("server").exe("engine/target/debug/server").cwd("engine/crates/server")
        .expect(Some("Rust"), Some("engine/crates/server"));
    phoenix_beam: Case::new(PHOENIX)
        .process("beam.smp").exe(BEAM).cwd("chat")
        .expect(Some("Phoenix"), Some("chat"));
    phoenix_erl_windows: Case::new(PHOENIX)
        .process("erl.exe").exe(ERL_WINDOWS).cwd("chat")
        .expect(Some("Phoenix"), Some("chat"));
    nvm_node_has_no_project: Case::new(NVM)
        .process("node").exe(".nvm/versions/node/v20.11.1/bin/node").cwd("")
        .expect(Some("Node.js"), None);
    nvm_node_without_cwd_has_no_project: Case::new(NVM)
        .process("node").exe(".nvm/versions/node/v20.11.1/bin/node")
        .expect(Some("Node.js"), None);
}
