//! Small name helpers shared by the process, image, project, and config rules.

use std::ffi::OsStr;
use std::path::Path;

/// Return `value` without `prefix`, comparing the prefix ignoring ASCII case.
pub fn strip_prefix_ignore_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .and_then(|_| value.get(prefix.len()..))
}

/// Return `value` without `suffix`, comparing the suffix ignoring ASCII case.
pub fn strip_suffix_ignore_ascii_case<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    let split = value.len().checked_sub(suffix.len())?;
    value
        .get(split..)
        .filter(|tail| tail.eq_ignore_ascii_case(suffix))
        .and_then(|_| value.get(..split))
}

/// Return the extension of a file name with [`Path::extension`] rules, so
/// `.bashrc` has none and `app.csproj` has `csproj`.
pub fn file_extension(name: &str) -> Option<&str> {
    Path::new(name).extension().and_then(OsStr::to_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affix_helpers_ignore_ascii_case_and_respect_char_boundaries() {
        assert_eq!(
            strip_prefix_ignore_ascii_case("Postgres-16", "postgres"),
            Some("-16")
        );
        assert_eq!(strip_prefix_ignore_ascii_case("pg", "postgres"), None);
        assert_eq!(strip_prefix_ignore_ascii_case("\u{e9}x", "e"), None);
        assert_eq!(
            strip_suffix_ignore_ascii_case("NGINX.EXE", ".exe"),
            Some("NGINX")
        );
        assert_eq!(strip_suffix_ignore_ascii_case("exe", ".exe"), None);
        assert_eq!(strip_suffix_ignore_ascii_case("x\u{e9}", "e"), None);
    }

    #[test]
    fn file_extension_follows_path_rules() {
        assert_eq!(file_extension("App.csproj"), Some("csproj"));
        assert_eq!(file_extension(".bashrc"), None);
        assert_eq!(file_extension("Makefile"), None);
    }
}
