// Imports are inside the function bodies to scope them in a cfg block.

/// The common result type, our errors will be simple strings.
pub type Result<T> = std::result::Result<T, String>;

#[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
fn join<T>(iter: T, sep: &str) -> String
where
    T: IntoIterator,
    T::Item: std::fmt::Display,
{
    let mut buf = String::new();
    let mut iter = iter.into_iter();
    if let Some(item) = iter.next() {
        let item = item.to_string();
        buf.push_str(&item);
    } else {
        return buf;
    }
    for item in iter {
        buf.push_str(sep);
        let item = item.to_string();
        buf.push_str(&item);
    }
    buf
}

/// Return all data from `path`. Line boundaries are normalized from
/// "\r\n" to "\n" to make sure "^" and "$" will match them. See
/// https://github.com/rust-lang/regex/issues/244 for details.
pub fn read_file(path: &str) -> std::io::Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf.replace("\r\n", "\n"))
}

/// Indent every line in text by four spaces.
#[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
pub fn indent(text: &str) -> String {
    join(text.lines().map(|line| String::from("    ") + line), "\n")
}

/// Verify that the version range request matches the given version.
#[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
pub fn version_matches_request(version: &semver_parser::version::Version,
        request: &semver_parser::range::VersionReq) -> Result<()> {
    use semver_parser::range::Op;
    if request.predicates.len() != 1 {
        // Can only handle simple dependencies
        return Ok(());
    }

    let pred = &request.predicates[0];
    match pred.op {
        Op::Tilde | Op::Compatible => {
            if pred.major != version.major {
                return Err(format!(
                    "expected major version {}, found {}",
                    version.major, pred.major,
                ));
            }
            if let Some(minor) = pred.minor {
                if minor != version.minor {
                    return Err(format!(
                        "expected minor version {}, found {}",
                        version.minor, minor
                    ));
                }
            }
            if let Some(patch) = pred.patch {
                if patch != version.patch {
                    return Err(format!(
                        "expected patch version {}, found {}",
                        version.patch, patch
                    ));
                }
            }
            if pred.pre != version.pre {
                return Err(format!(
                    "expected pre-release {:?}, found {:?}",
                    join(&version.pre, "."),
                    join(&pred.pre, ".")
                ));
            }
        }
        _ => return Ok(()), // We cannot check other operators.
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    use semver_parser::range::parse as parse_request;
    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    use semver_parser::version::parse as parse_version;

    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    use super::*;

    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    mod test_version_matches_request {
        use super::*;

        #[test]
        fn implicit_compatible() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn compatible() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("^1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn tilde() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("~1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn no_patch() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1.2").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn no_minor() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("1").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn multiple_predicates() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request(">= 1.2.3, < 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn unhandled_operator() {
            let version = parse_version("1.2.3").unwrap();
            let request = parse_request("< 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn bad_major() {
            let version = parse_version("2.0.0").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected major version 2, found 1"))
            );
        }

        #[test]
        fn bad_minor() {
            let version = parse_version("1.3.0").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected minor version 3, found 2"))
            );
        }

        #[test]
        fn bad_patch() {
            let version = parse_version("1.2.4").unwrap();
            let request = parse_request("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected patch version 4, found 3"))
            );
        }

        #[test]
        fn bad_pre_release() {
            let version = parse_version("1.2.3-rc2").unwrap();
            let request = parse_request("1.2.3-rc1").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected pre-release \"rc2\", found \"rc1\""))
            );
        }
    }
}
