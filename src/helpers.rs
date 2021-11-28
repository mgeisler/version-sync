use std::fs::File;
use std::io::{self, Read};

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
pub fn read_file(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
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
pub fn version_matches_request(
    version: &semver::Version,
    request: &semver::VersionReq,
) -> Result<()> {
    use semver::Op;

    for comparator in &request.comparators {
        match comparator.op {
            Op::Tilde | Op::Caret | Op::Exact | Op::Greater | Op::GreaterEq | Op::Wildcard => {
                if comparator.major != version.major {
                    return Err(format!(
                        "expected major version {}, found {}",
                        version.major, comparator.major,
                    ));
                }
                if let Some(minor) = comparator.minor {
                    if minor != version.minor {
                        return Err(format!(
                            "expected minor version {}, found {}",
                            version.minor, minor
                        ));
                    }
                }
                if let Some(patch) = comparator.patch {
                    if patch != version.patch {
                        return Err(format!(
                            "expected patch version {}, found {}",
                            version.patch, patch
                        ));
                    }
                }
                if comparator.pre != version.pre {
                    return Err(format!(
                        "expected pre-release \"{}\", found \"{}\"",
                        version.pre, comparator.pre
                    ));
                }
            }
            _ => {} // We cannot check other operators.
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    use semver::{Version, VersionReq};

    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    use super::*;

    #[cfg(any(feature = "html_root_url_updated", feature = "markdown_deps_updated"))]
    mod test_version_matches_request {
        use super::*;

        #[test]
        fn implicit_compatible() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse("1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn compatible() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("^1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse("^1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn tilde() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("~1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse("~1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn exact() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("=1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse("=1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn greater_or_equal() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse(">=1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse(">=1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn wildcard() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("1.2.*").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse("1.3.*").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn greater() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse(">1.2.3").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse(">1.2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn no_patch() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("1.2").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn no_minor() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("1").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn multiple_comparators() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse(">= 1.2.3, < 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));

            let request = VersionReq::parse(">= 1.2.0, < 2.0").unwrap();
            assert!(version_matches_request(&version, &request).is_err());
        }

        #[test]
        fn unhandled_operator() {
            let version = Version::parse("1.2.3").unwrap();
            let request = VersionReq::parse("< 2.0").unwrap();
            assert_eq!(version_matches_request(&version, &request), Ok(()));
        }

        #[test]
        fn bad_major() {
            let version = Version::parse("2.0.0").unwrap();
            let request = VersionReq::parse("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected major version 2, found 1"))
            );
        }

        #[test]
        fn bad_minor() {
            let version = Version::parse("1.3.0").unwrap();
            let request = VersionReq::parse("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected minor version 3, found 2"))
            );
        }

        #[test]
        fn bad_patch() {
            let version = Version::parse("1.2.4").unwrap();
            let request = VersionReq::parse("1.2.3").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected patch version 4, found 3"))
            );
        }

        #[test]
        fn bad_pre_release() {
            let version = Version::parse("1.2.3-rc2").unwrap();
            let request = VersionReq::parse("1.2.3-rc1").unwrap();
            assert_eq!(
                version_matches_request(&version, &request),
                Err(String::from("expected pre-release \"rc2\", found \"rc1\""))
            );
        }
    }
}
