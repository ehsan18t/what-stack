//! Universal Rust library for detecting project roots and technology stacks

/// Returns a greeting for `name`.
#[must_use]
pub fn greeting(name: &str) -> String {
    format!("Hello, {name}!")
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn greeting_includes_name() {
        assert_eq!(greeting("Rust"), "Hello, Rust!");
    }
}
