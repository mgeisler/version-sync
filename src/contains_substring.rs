use crate::helpers::{read_file, Result};

/// Check that `path` contain the substring given by `template`.
///
/// The placeholders `{name}` and `{version}` will be replaced with
/// `pkg_name` and `pkg_version`, if they are present in `template`.
/// It is okay if `template` do not contain these placeholders.
///
/// See [`check_contains_regex`](crate::check_contains_regex) if you
/// want to match with a regular expression instead.
///
/// # Errors
///
/// If the template cannot be found, an `Err` is returned with a
/// succinct error message. Status information has then already been
/// printed on `stdout`.
pub fn check_contains_substring(
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
    let pattern = template
        .replace("{name}", &pkg_name)
        .replace("{version}", &pkg_version);

    let text = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;

    println!("Searching for \"{}\" in {}...", template, path);
    match text.find(&pattern) {
        Some(idx) => {
            let line_no = text[..idx].lines().count();
            println!("{} (line {}) ... ok", path, line_no + 1);
            Ok(())
        }
        None => Err(format!("could not find \"{}\" in {}", pattern, path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_not_found() {
        assert_eq!(
            check_contains_substring("README.md", "should not be found", "foobar", "1.2.3"),
            Err(String::from(
                "could not find \"should not be found\" in README.md"
            ))
        )
    }

    #[test]
    fn pattern_found() {
        assert_eq!(
            check_contains_substring("README.md", "{name}", "version-sync", "1.2.3"),
            Ok(())
        )
    }
}
