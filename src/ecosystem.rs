//! Internal language-ecosystem tags.
//!
//! Process rules and config rules both carry an [`Ecosystem`]. When a known
//! runtime or tool process is enriched with project config, only config labels
//! from the process's own ecosystem may replace its label, so a `php` process
//! in a Laravel project that also has `vite.config.js` is `Laravel`, not
//! `Vite`.

/// Language ecosystem a process or config rule belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    /// JavaScript and TypeScript: Node.js, Bun, and their tools.
    Node,
    /// The Deno runtime and its `deno.json` config. Deno also runs Node
    /// projects, so a Deno process accepts [`Ecosystem::Node`] config too.
    Deno,
    /// Python runtimes, application servers, and frameworks.
    Python,
    /// Ruby runtimes, application servers, and frameworks.
    Ruby,
    /// PHP runtimes and frameworks.
    Php,
    /// Java and Kotlin on the JVM, including Maven and Gradle.
    Jvm,
    /// .NET runtimes and project files.
    DotNet,
    /// Rust toolchain.
    Rust,
    /// Go toolchain.
    Go,
    /// Erlang and Elixir on the BEAM.
    Beam,
    /// No config ecosystem: services, databases, and runtimes without config
    /// rules (`Perl`, `Dart`, `Swift`).
    Other,
}

impl Ecosystem {
    /// Whether a config rule tagged `rule` may refine the label of a process
    /// from this ecosystem.
    ///
    /// A `node` or `bun` process next to `deno.json` stays `Node.js` or `Bun`,
    /// while a `deno` process in a Next.js project is `Next.js`.
    pub fn accepts_config(self, rule: Self) -> bool {
        self == rule || (self == Self::Deno && rule == Self::Node)
    }

    /// Whether projects of this ecosystem are compiled to a native or
    /// bytecode artifact that runs as its own process (Rust, Go, .NET, and the
    /// JVM), so an unknown executable inside the project is most likely one.
    pub const fn is_compiled(self) -> bool {
        matches!(self, Self::Rust | Self::Go | Self::DotNet | Self::Jvm)
    }
}
