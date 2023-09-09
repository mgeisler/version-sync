#![cfg(feature = "contains_regex")]
use regex::{escape, Regex, RegexBuilder};
use semver::{Version, VersionReq};

use crate::helpers::{read_file, version_matches_request, Result};

/// Matches a full or partial SemVer version number.
const SEMVER_RE: &str = concat!(
    r"(?P<major>0|[1-9]\d*)",
    r"(?:\.(?P<minor>0|[1-9]\d*)",
    r"(?:\.(?P<patch>0|[1-9]\d*)",
    r"(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)",
    r"(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?",
    r"(?:\+(?P<buildmetadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?",
    r")?", // Close patch plus prerelease and buildmetadata.
    r")?", // Close minor.
);

/// Check that `path` contain the regular expression given by
/// `template`.
///
/// This function only checks that there is at least one match for the
/// `template` given. Use [`check_only_contains_regex`] if you want to
/// ensure that all references to your package version is up to date.
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
    // Expand the placeholders in the template.
    let pattern = template
        .replace("{name}", &escape(pkg_name))
        .replace("{version}", &escape(pkg_version));
    let mut builder = RegexBuilder::new(&pattern);
    builder.multi_line(true);
    let re = builder
        .build()
        .map_err(|err| format!("could not parse template: {}", err))?;
    let text = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;

    println!("Searching for \"{pattern}\" in {path}...");
    match re.find(&text) {
        Some(m) => {
            let line_no = text[..m.start()].lines().count();
            println!("{} (line {}) ... ok", path, line_no + 1);
            Ok(())
        }
        None => Err(format!("could not find \"{pattern}\" in {path}")),
    }
}

/// Check that `path` only contains matches to the regular expression
/// given by `template`.
///
/// While the [`check_contains_regex`] function verifies the existance
/// of _at least one match_, this function verifies that _all matches_
/// use the correct version number. Use this if you have a file which
/// should always reference the current version of your package.
///
/// The check proceeds in two steps:
///
/// 1. Replace `{version}` in `template` by a regular expression which
///    will match _any_ SemVer version number. This allows, say,
///    `"docs.rs/{name}/{version}/"` to match old and outdated
///    occurrences of your package.
///
/// 2. Find all matches in the file and check the version number in
///    each match for compatibility with `pkg_version`. It is enough
///    for the version number to be compatible, meaning that
///    `"foo/{version}/bar" matches `"foo/1.2/bar"` when `pkg_version`
///    is `"1.2.3"`.
///
/// It is an error if there are no matches for `template` at all.
///
/// The matching is done in multi-line mode, which means that `^` in
/// the regular expression will match the beginning of any line in the
/// file, not just the very beginning of the file.
///
/// # Errors
///
/// If any of the matches are incompatible with `pkg_version`, an
/// `Err` is returned with a succinct error message. Status
/// information has then already been printed on `stdout`.
pub fn check_only_contains_regex(
    path: &str,
    template: &str,
    pkg_name: &str,
    pkg_version: &str,
) -> Result<()> {
    let version = Version::parse(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;

    let pattern = template
        .replace("{name}", &escape(pkg_name))
        .replace("{version}", SEMVER_RE);
    let re = RegexBuilder::new(&pattern)
        .multi_line(true)
        .build()
        .map_err(|err| format!("could not parse template: {}", err))?;

    let semver_re = Regex::new(SEMVER_RE).unwrap();

    let text = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;

    println!("Searching for \"{template}\" in {path}...");
    let mut errors = 0;
    let mut has_match = false;

    for m in re.find_iter(&text) {
        has_match = true;
        let line_no = text[..m.start()].lines().count() + 1;

        for semver in semver_re.find_iter(m.as_str()) {
            let semver_request = VersionReq::parse(semver.as_str())
                .map_err(|err| format!("could not parse version: {}", err))?;
            let result = version_matches_request(&version, &semver_request);
            match result {
                Err(err) => {
                    errors += 1;
                    println!(
                        "{} (line {}) ... found \"{}\", which does not match version \"{}\": {}",
                        path,
                        line_no,
                        semver.as_str(),
                        pkg_version,
                        err
                    );
                }
                Ok(()) => {
                    println!("{path} (line {line_no}) ... ok");
                }
            }
        }
    }

    if !has_match {
        return Err(format!(
            "{path} ... found no matches for \"{template}\""
        ));
    }

    if errors > 0 {
        return Err(format!("{path} ... found {errors} errors"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn bad_regex() {
        // Check that the error from a bad pattern doesn't contain
        // the (?m) prefix.
        assert_eq!(
            check_contains_regex("README.md", "Version {version} [ups", "foobar", "1.2.3"),
            Err([
                    r"could not parse template: regex parse error:",
                    r"    Version 1\.2\.3 [ups",
                    r"                    ^",
                    r"error: unclosed character class"
                ]
                .join("\n"))
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
            Err([
                    r#"could not find "escaped: foo\*bar-1\.2\.3,"#,
                    r#"not escaped: foo*bar-1.2.3" in README.md"#
                ]
                .join(" "))
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
        let mut file = tempfile::NamedTempFile::new().unwrap();

        file.write_all(b"first line\r\nsecond line\r\nthird line\r\n")
            .unwrap();
        assert_eq!(
            check_contains_regex(file.path().to_str().unwrap(), "^second line$", "", ""),
            Ok(())
        )
    }

    #[test]
    fn semver_regex() {
        // We anchor the regex here to better match the behavior when
        // users call check_only_contains_regex with a string like
        // "foo {version}" which also contains more than just
        // "{version}".
        let re = Regex::new(&format!("^{SEMVER_RE}$")).unwrap();
        assert!(re.is_match("1.2.3"));
        assert!(re.is_match("1.2"));
        assert!(re.is_match("1"));
        assert!(re.is_match("1.2.3-foo.bar.baz.42+build123.2021.12.11"));
        assert!(!re.is_match("01"));
        assert!(!re.is_match("01.02.03"));
    }

    #[test]
    fn only_contains_success() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            b"first:  docs.rs/foo/1.2.3/foo/fn.bar.html
              second: docs.rs/foo/1.2.3/foo/fn.baz.html",
        )
        .unwrap();

        assert_eq!(
            check_only_contains_regex(
                file.path().to_str().unwrap(),
                "docs.rs/{name}/{version}/{name}/",
                "foo",
                "1.2.3"
            ),
            Ok(())
        )
    }

    #[test]
    fn only_contains_success_compatible() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            b"first:  docs.rs/foo/1.2/foo/fn.bar.html
              second: docs.rs/foo/1/foo/fn.baz.html",
        )
        .unwrap();

        assert_eq!(
            check_only_contains_regex(
                file.path().to_str().unwrap(),
                "docs.rs/{name}/{version}/{name}/",
                "foo",
                "1.2.3"
            ),
            Ok(())
        )
    }

    #[test]
    fn only_contains_failure() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(
            b"first:  docs.rs/foo/1.0.0/foo/ <- error
              second: docs.rs/foo/2.0.0/foo/ <- ok
              third:  docs.rs/foo/3.0.0/foo/ <- error",
        )
        .unwrap();

        assert_eq!(
            check_only_contains_regex(
                file.path().to_str().unwrap(),
                "docs.rs/{name}/{version}/{name}/",
                "foo",
                "2.0.0"
            ),
            Err(format!("{} ... found 2 errors", file.path().display()))
        )
    }

    #[test]
    fn only_contains_fails_if_no_match() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"not a match").unwrap();

        assert_eq!(
            check_only_contains_regex(
                file.path().to_str().unwrap(),
                "docs.rs/{name}/{version}/{name}/",
                "foo",
                "1.2.3"
            ),
            Err(format!(
                r#"{} ... found no matches for "docs.rs/{{name}}/{{version}}/{{name}}/""#,
                file.path().display()
            ))
        );
    }
}
