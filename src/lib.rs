//! `version-sync` provides macros for keeping version numbers in sync
//! with your crate version.
//!
//! When making a release of a Rust project, you typically need to
//! adjust some version numbers in your code and documentation. This
//! crate gives you macros that covers some typical cases where
//! version numbers need updating:
//!
//! * TOML examples in the `README.md` files that show how to add a
//!   dependency on your crate. See [`assert_markdown_deps_updated`].
//!
//! * A `Changelog.md` file that should at least mention the current
//!   version. See [`assert_contains_regex`].
//!
//! * The [`html_root_url`] attribute that tells other crates where to
//!   find your documentation. See [`assert_html_root_url_updated`].
//!
//! A typical configuration will use an integration test to verify
//! that all version numbers are in sync. Create a
//! `tests/version-numbers.rs` file with:
//!
//! ```rust
//! #[macro_use]
//! extern crate version_sync;
//!
//! #[test]
//! # fn fake_hidden_test_case_1() {}
//! fn test_readme_deps() {
//!     assert_markdown_deps_updated!("README.md");
//! }
//!
//! #[test]
//! # fn fake_hidden_test_case_2() {}
//! fn test_html_root_url() {
//!     assert_html_root_url_updated!("src/lib.rs");
//! }
//!
//! # fn main() {
//! #     test_readme_deps();
//! #     test_html_root_url();
//! # }
//! ```
//!
//! When you run `cargo test`, your version numbers will be
//! automatically checked.
//!
//! [`html_root_url`]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html#crate-sets-html_root_url-attribute-c-html-root
//! [`assert_markdown_deps_updated`]: macro.assert_markdown_deps_updated.html
//! [`assert_html_root_url_updated`]: macro.assert_html_root_url_updated.html
//! [`assert_contains_regex`]: macro.assert_contains_regex.html

#![doc(html_root_url = "https://docs.rs/version-sync/0.6.0")]
#![deny(missing_docs)]

extern crate itertools;
extern crate pulldown_cmark;
extern crate regex;
extern crate semver_parser;
extern crate syn;
extern crate toml;
extern crate url;

use std::fs::File;
use std::io::Read;
use std::result;

use itertools::join;
use semver_parser::range::parse as parse_request;
use semver_parser::range::{Op, VersionReq};
use semver_parser::version::parse as parse_version;
use semver_parser::version::Version;
use url::Url;

/// The common result type, our errors will be simple strings.
type Result<T> = result::Result<T, String>;

/// Return all data from `path`.
fn read_file(path: &str) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Indent every line in text by four spaces.
fn indent(text: &str) -> String {
    join(text.lines().map(|line| String::from("    ") + line), "\n")
}

/// Verify that the version range request matches the given version.
fn version_matches_request(version: &Version, request: &VersionReq) -> Result<()> {
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

mod markdown_deps;
pub use markdown_deps::check_markdown_deps;

/// Assert that dependencies on the current package are up to date.
///
/// The macro will call [`check_markdown_deps`] on the file name given
/// in order to check that the TOML examples found all depend on a
/// current version of your package. The package name is automatically
/// taken from the `$CARGO_PKG_NAME` environment variable and the
/// version is taken from `$CARGO_PKG_VERSION`. These environment
/// variables are automatically set by Cargo when compiling your
/// crate.
///
/// # Usage
///
/// The typical way to use this macro is from an integration test:
///
/// ```rust
/// #[macro_use]
/// extern crate version_sync;
///
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_readme_deps is compiled.
/// fn test_readme_deps() {
///     assert_markdown_deps_updated!("README.md");
/// }
///
/// # fn main() {
/// #     test_readme_deps();
/// # }
/// ```
///
/// Tests are run with the current directory set to directory where
/// your `Cargo.toml` file is, so this will find a `README.md` file
/// next to your `Cargo.toml` file.
///
/// # Panics
///
/// If any TOML code block fails the check, `panic!` will be invoked.
///
/// [`check_markdown_deps`]: fn.check_markdown_deps.html
#[macro_export]
macro_rules! assert_markdown_deps_updated {
    ($path:expr) => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        if let Err(err) = $crate::check_markdown_deps($path, pkg_name, pkg_version) {
            panic!(err);
        }
    };
}

fn url_matches(value: &str, pkg_name: &str, version: &Version) -> Result<()> {
    let url = Url::parse(value).map_err(|err| format!("parse error: {}", err))?;

    // We can only reason about docs.rs.
    if url.domain().is_some() && url.domain() != Some("docs.rs") {
        return Ok(());
    }

    // Since docs.rs redirects HTTP traffic to HTTPS, we will ensure
    // that the scheme is "https" here.
    if url.scheme() != "https" {
        return Err(format!("expected \"https\", found {:?}", url.scheme()));
    }

    let mut path_segments = url
        .path_segments()
        .ok_or_else(|| String::from("no path in URL"))?;

    // The package name should not be empty.
    let name = path_segments
        .next()
        .and_then(|path| if path.is_empty() { None } else { Some(path) })
        .ok_or_else(|| String::from("missing package name"))?;

    // The version number should not be empty.
    let request = path_segments
        .next()
        .and_then(|path| if path.is_empty() { None } else { Some(path) })
        .ok_or_else(|| String::from("missing version number"))?;

    // Finally, we check that the package name and version matches.
    if name != pkg_name {
        Err(format!(
            "expected package \"{}\", found \"{}\"",
            pkg_name, name
        ))
    } else {
        // The Rust API Guidelines[1] suggest using an exact version
        // number, but we have relaxed this a little and allow the
        // user to specify the version as just "1" or "1.2". We might
        // make this more strict in the future.
        //
        // [1]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html
        // #crate-sets-html_root_url-attribute-c-html-root
        parse_request(request)
            .map_err(|err| format!("could not parse version in URL: {}", err))
            .and_then(|request| version_matches_request(version, &request))
    }
}

/// Check version numbers in `html_root_url` attributes.
///
/// This function parses the Rust source file in `path` and looks for
/// `html_root_url` attributes. Such an attribute must specify a valid
/// URL and if the URL points to docs.rs, it must be point to the
/// documentation for `pkg_name` and `pkg_version`.
///
/// # Errors
///
/// If any attribute fails the check, an `Err` is returned with a
/// succinct error message. Status information has then already been
/// printed on `stdout`.
pub fn check_html_root_url(path: &str, pkg_name: &str, pkg_version: &str) -> Result<()> {
    let code = read_file(path).map_err(|err| format!("could not read {}: {}", path, err))?;
    let version = parse_version(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;
    let krate: syn::File = syn::parse_str(&code)
        .map_err(|_| format!("could not parse {}: please run \"cargo build\"", path))?;

    println!("Checking doc attributes in {}...", path);
    for attr in krate.attrs {
        if let syn::AttrStyle::Outer = attr.style {
            continue;
        }
        let (ident, nested_meta_items) = match attr.interpret_meta() {
            Some(syn::Meta::List(syn::MetaList { ident, nested, .. })) => (ident, nested),
            _ => continue,
        };

        if ident != "doc" {
            continue;
        }

        for nested_meta_item in nested_meta_items {
            let meta_item = match nested_meta_item {
                syn::NestedMeta::Meta(ref meta_item) => meta_item,
                _ => continue,
            };

            let check_result = match *meta_item {
                syn::Meta::NameValue(syn::MetaNameValue {
                    ref ident, ref lit, ..
                })
                    if ident == "html_root_url" =>
                {
                    match *lit {
                        // Accept both cooked and raw strings here.
                        syn::Lit::Str(ref s) => url_matches(&s.value(), pkg_name, &version),
                        // A non-string html_root_url is probably an
                        // error, but we leave this check to the
                        // compiler.
                        _ => continue,
                    }
                }
                syn::Meta::Word(ref name) if name == "html_root_url" => {
                    Err(String::from("html_root_url attribute without URL"))
                }
                _ => continue,
            };

            // FIXME: use line number from the syn crate when it
            // preserves span information. Here we simply find the
            // first source line that contains "html_root_url".
            //
            // We know such a line must exist since we would have
            // continue the loop above if it wasn't present.
            let (line_no, source_line) = code
                .lines()
                .enumerate()
                .find(|&(_, line)| line.contains("html_root_url"))
                .expect("html_root_url attribute not present");

            match check_result {
                Ok(()) => {
                    println!("{} (line {}) ... ok", path, line_no + 1);
                    return Ok(());
                }
                Err(err) => {
                    println!("{} (line {}) ... {} in", path, line_no + 1, err);
                    println!("{}\n", indent(source_line));
                    return Err(format!("html_root_url errors in {}", path));
                }
            }
        }
    }

    Ok(())
}

/// Assert that the `html_root_url` attribute is up to date.
///
/// Library code is [expected to set `html_root_url`][api-guidelines]
/// to point to docs.rs so that rustdoc can generate correct links
/// when referring to this crate.
///
/// The macro will call [`check_html_root_url`] on the file name given
/// in order to check that the `html_root_url` is points to the
/// current version of your package documentation on docs.rs. The
/// package name is automatically taken from the `$CARGO_PKG_NAME`
/// environment variable and the version is taken from
/// `$CARGO_PKG_VERSION`. These environment variables are
/// automatically set by Cargo when compiling your crate.
///
/// # Usage
///
/// The typical way to use this macro is from an integration test:
///
/// ```rust
/// #[macro_use]
/// extern crate version_sync;
///
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_html_root_url is compiled.
/// fn test_html_root_url() {
///     assert_html_root_url_updated!("src/lib.rs");
/// }
///
/// # fn main() {
/// #     test_html_root_url();
/// # }
/// ```
///
/// Tests are run with the current directory set to directory where
/// your `Cargo.toml` file is, so this will find the `src/lib.rs`
/// crate root.
///
/// # Panics
///
/// If the `html_root_url` fails the check, `panic!` will be invoked.
///
/// [api-guidelines]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html#crate-sets-html_root_url-attribute-c-html-root
/// [`check_html_root_url`]: fn.check_html_root_url.html
#[macro_export]
macro_rules! assert_html_root_url_updated {
    ($path:expr) => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        if let Err(err) = $crate::check_html_root_url($path, pkg_name, pkg_version) {
            panic!(err);
        }
    };
}

mod contains_regex;
pub use contains_regex::check_contains_regex;

/// Assert that versions numbers are up to date via a regex.
///
/// This macro allows you verify that the current version number is
/// mentioned in a particular file, such as a changelog file. You do
/// this by specifying a regular expression which will be matched
/// against the file.
///
/// The macro calls [`check_contains_regex`] on the file name given.
/// The package name and current package version is automatically
/// taken from the `$CARGO_PKG_NAME` and `$CARGO_PKG_VERSION`
/// environment variables. These environment variables are
/// automatically set by Cargo when compiling your crate.
///
/// # Usage
///
/// The typical way to use this macro is from an integration test:
///
/// ```rust
/// #[macro_use]
/// extern crate version_sync;
///
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_readme_mentions_version is
/// # // compiled.
/// fn test_readme_mentions_version() {
///     assert_contains_regex!("README.md", "^### Version {version}");
/// }
///
/// # fn main() {
/// #     test_readme_mentions_version();
/// # }
/// ```
///
/// Tests are run with the current directory set to directory where
/// your `Cargo.toml` file is, so this will find a `README.md` file
/// next to your `Cargo.toml` file. It will then check that there is a
/// heading mentioning the current version of your crate.
///
/// The regular expression can contain placeholders which are replaced
/// before the regular expression search begins:
///
/// * `{version}`: the current version number of your package.
/// * `{name}`: the name of your package.
///
/// This way you can search for things like `"Latest version of {name}
/// is: {version}"` and make sure you update your READMEs and
/// changelogs consistently.
///
/// # Panics
///
/// If the regular expression cannot be found, `panic!` will be
/// invoked and your integration test will fail.
///
/// [`check_contains_regex`]: fn.check_contains_regex.html
#[macro_export]
macro_rules! assert_contains_regex {
    ($path:expr, $format:expr) => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        if let Err(err) = $crate::check_contains_regex($path, $format, pkg_name, pkg_version) {
            panic!(err);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

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

    mod test_url_matches {
        use super::*;

        #[test]
        fn good_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo/1.2.3", "foo", &ver),
                Ok(())
            );
        }

        #[test]
        fn trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo/1.2.3/", "foo", &ver),
                Ok(())
            );
        }

        #[test]
        fn without_patch() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1.2", "foo", &ver), Ok(()));
        }

        #[test]
        fn without_minor() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://docs.rs/foo/1", "foo", &ver), Ok(()));
        }

        #[test]
        fn different_domain() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(url_matches("https://example.net/foo/", "bar", &ver), Ok(()));
        }

        #[test]
        fn different_domain_http() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("http://example.net/foo/1.2.3", "foo", &ver),
                Ok(())
            );
        }

        #[test]
        fn http_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("http://docs.rs/foo/1.2.3", "foo", &ver),
                Err(String::from("expected \"https\", found \"http\""))
            );
        }

        #[test]
        fn bad_scheme() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("mailto:foo@example.net", "foo", &ver),
                Err(String::from("expected \"https\", found \"mailto\""))
            );
        }

        #[test]
        fn no_package() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs", "foo", &ver),
                Err(String::from("missing package name"))
            );
        }

        #[test]
        fn no_package_trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/", "foo", &ver),
                Err(String::from("missing package name"))
            );
        }

        #[test]
        fn no_version() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo", "foo", &ver),
                Err(String::from("missing version number"))
            );
        }

        #[test]
        fn no_version_trailing_slash() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo/", "foo", &ver),
                Err(String::from("missing version number"))
            );
        }

        #[test]
        fn bad_url() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("docs.rs/foo/bar", "foo", &ver),
                Err(String::from("parse error: relative URL without a base"))
            );
        }

        #[test]
        fn bad_pkg_version() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo/1.2.bad/", "foo", &ver),
                Err(String::from(
                    "could not parse version in URL: \
                     encountered unexpected token: AlphaNumeric(\"bad\")"
                ))
            );
        }

        #[test]
        fn wrong_pkg_name() {
            let ver = parse_version("1.2.3").unwrap();
            assert_eq!(
                url_matches("https://docs.rs/foo/1.2.3/", "bar", &ver),
                Err(String::from("expected package \"bar\", found \"foo\""))
            );
        }
    }

    mod test_check_html_root_url {
        use super::*;

        #[test]
        fn bad_path() {
            let no_such_file = if cfg!(unix) {
                "No such file or directory (os error 2)"
            } else {
                "The system cannot find the file specified. (os error 2)"
            };
            let errmsg = format!("could not read no-such-file.md: {}", no_such_file);
            assert_eq!(
                check_html_root_url("no-such-file.md", "foobar", "1.2.3"),
                Err(errmsg)
            );
        }

        #[test]
        fn bad_pkg_version() {
            // This uses the src/lib.rs file from this crate.
            assert_eq!(
                check_html_root_url("src/lib.rs", "foobar", "1.2"),
                Err(String::from(
                    "bad package version \"1.2\": expected more input"
                ))
            );
        }
    }

}
