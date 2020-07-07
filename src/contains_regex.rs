use regex::{escape, Regex};

use crate::helpers::{read_file, Result};

/// Check that `path` contain the regular expression given by
/// `template`.
///
/// The placeholders `{name}` and `{version}` will be replaced with
/// `pkg_name` and `pkg_version`, if they are present in `template`.
/// It is okay if `template` do not contain these placeholders.
///
/// The matching is done in multi-line mode, which means that `^` in
/// the regular expression will match the beginning of any line in the
/// file, not just the very beginning of the file.
///
/// # Errors
///
/// If the regular expression cannot be found, an `Err` is returned
/// with a succinct error message. Status information has then already
/// been printed on `stdout`.
pub fn check_contains_regex(
    path: &str,
    template: &str,
    pkg_name: &str,
    pkg_version: &str,
) -> Result<()> {
    // Expand the optional {name} and {version} placeholders in the
    // template. This is almost like
    //
    //   format!(template, name = pkg_name, version = pkg_version)
    //
    // but allows the user to leave out unnecessary placeholders.
    let orig_regex = template
        .replace("{name}", &escape(pkg_name))
        .replace("{version}", &escape(pkg_version));

    // We start by constructing a Regex from the original string. This
    // ensurs that any errors refer to the string the user passed
    // instead of the string we use internally.
    let re = match Regex::new(&orig_regex) {
        Ok(_) => {
            // We now know that the regex is valid, so we can enable
            // multi-line mode by prepending "(?m)".
            let regex = String::from("(?m)") + &orig_regex;
            Regex::new(&regex).unwrap()
        }
        Err(err) => return Err(format!("could not parse template: {}", err)),
    };
    let text = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;

    println!("Searching for \"{}\" in {}...", orig_regex, path);
    match re.find(&text) {
        Some(m) => {
            let line_no = text[..m.start()].lines().count();
            println!("{} (line {}) ... ok", path, line_no + 1);
            Ok(())
        }
        None => Err(format!("could not find \"{}\" in {}", orig_regex, path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_regex() {
        // Check that the error from a bad pattern doesn't contain
        // the (?m) prefix.
        assert_eq!(
            check_contains_regex("README.md", "Version {version} [ups", "foobar", "1.2.3"),
            Err(String::from(
                [
                    r"could not parse template: regex parse error:",
                    r"    Version 1\.2\.3 [ups",
                    r"                    ^",
                    r"error: unclosed character class"
                ]
                .join("\n")
            ))
        )
    }

    #[test]
    fn not_found() {
        assert_eq!(
            check_contains_regex("README.md", "should not be found", "foobar", "1.2.3"),
            Err(String::from(
                "could not find \"should not be found\" in README.md"
            ))
        )
    }

    #[test]
    fn escaping() {
        assert_eq!(
            check_contains_regex(
                "README.md",
                "escaped: {name}-{version}, not escaped: foo*bar-1.2.3",
                "foo*bar",
                "1.2.3"
            ),
            Err(String::from(
                [
                    r#"could not find "escaped: foo\*bar-1\.2\.3,"#,
                    r#"not escaped: foo*bar-1.2.3" in README.md"#
                ]
                .join(" ")
            ))
        )
    }

    #[test]
    fn good_pattern() {
        assert_eq!(
            check_contains_regex("README.md", "{name}", "version-sync", "1.2.3"),
            Ok(())
        )
    }

    #[test]
    fn line_boundaries() {
        // The regex crate doesn't treat \r\n as a line boundary
        // (https://github.com/rust-lang/regex/issues/244), so
        // version-sync makes sure to normalize \r\n to \n when
        // reading files.
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();

        println!("Path: {}", file.path().to_str().unwrap());

        file.write_all(b"first line\r\nsecond line\r\nthird line\r\n")
            .unwrap();
        assert_eq!(
            check_contains_regex(file.path().to_str().unwrap(), "^second line$", "", ""),
            Ok(())
        )
    }
}
