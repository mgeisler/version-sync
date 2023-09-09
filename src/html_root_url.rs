#![cfg(feature = "html_root_url_updated")]
use semver::{Version, VersionReq};
use syn::spanned::Spanned;
use syn::token;
use url::Url;

use crate::helpers::{indent, read_file, version_matches_request, Result};

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
        Err(format!("expected package \"{pkg_name}\", found \"{name}\""))
    } else {
        // The Rust API Guidelines[1] suggest using an exact version
        // number, but we have relaxed this a little and allow the
        // user to specify the version as just "1" or "1.2". We might
        // make this more strict in the future.
        //
        // [1]: https://rust-lang-nursery.github.io/api-guidelines/documentation.html
        // #crate-sets-html_root_url-attribute-c-html-root
        VersionReq::parse(request)
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
    let version = Version::parse(pkg_version)
        .map_err(|err| format!("bad package version {:?}: {}", pkg_version, err))?;
    let krate: syn::File = syn::parse_file(&code)
        .map_err(|_| format!("could not parse {}: please run \"cargo build\"", path))?;

    println!("Checking doc attributes in {path}...");
    for attr in krate.attrs {
        if let syn::AttrStyle::Outer = attr.style {
            continue;
        }

        if !attr.path().is_ident("doc") {
            continue;
        }

        if let syn::Meta::List(ref list) = attr.meta {
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("html_root_url") {
                    let check_result = match meta.value() {
                        Ok(value) => match value.parse()? {
                            syn::Lit::Str(ref s) => url_matches(&s.value(), pkg_name, &version),
                            _ => return Ok(()),
                        },
                        Err(_err) => Err(String::from("html_root_url attribute without URL")),
                    };

                    // FIXME: the proc-macro2-0.4.27 crate hides accurate span
                    // information behind a procmacro2_semver_exempt flag: the
                    // start line is correct, but the end line is always equal
                    // to the start. Luckily, most html_root_url attributes
                    // are on a single line, so the code below works okay.
                    let first_line = attr.span().start().line;
                    let last_line = attr.span().end().line;
                    // Getting the source code for a span is tracked upstream:
                    // https://github.com/alexcrichton/proc-macro2/issues/110.
                    let source_lines = code.lines().take(last_line).skip(first_line - 1);
                    match check_result {
                        Ok(()) => {
                            println!("{} (line {}) ... ok", path, first_line);
                            return Ok(());
                        }
                        Err(err) => {
                            println!("{} (line {}) ... {} in", path, first_line, err);
                            for line in source_lines {
                                println!("{}", indent(line));
                            }
                            return Err(meta.error(format!("html_root_url errors in {}", path)));
                        }
                    }
                }
                // Need to advance the input stream by parsing it.
                // Otherwise syn gets stuck parsing the wrong tokens.
                else if meta.input.peek(token::Eq) {
                    let value = meta.value()?;
                    value.parse::<proc_macro2::TokenTree>()?;
                } else if meta.input.peek(token::Paren) {
                    let value;
                    syn::parenthesized!(value in meta.input);
                    // There can be multiple elements before end.
                    while !value.is_empty() {
                        value.parse::<proc_macro2::TokenTree>()?;
                    }
                } else {
                    return Err(meta.error("unknown doc attribute"));
                }

                Ok(())
            })
            .map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test_url_matches {
    use super::*;

    #[test]
    fn good_url() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo/1.2.3", "foo", &ver),
            Ok(())
        );
    }

    #[test]
    fn trailing_slash() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo/1.2.3/", "foo", &ver),
            Ok(())
        );
    }

    #[test]
    fn without_patch() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(url_matches("https://docs.rs/foo/1.2/", "foo", &ver), Ok(()));
    }

    #[test]
    fn without_minor() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(url_matches("https://docs.rs/foo/1/", "foo", &ver), Ok(()));
    }

    #[test]
    fn different_domain() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(url_matches("https://example.net/foo/", "bar", &ver), Ok(()));
    }

    #[test]
    fn different_domain_http() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("http://example.net/foo/1.2.3", "foo", &ver),
            Ok(())
        );
    }

    #[test]
    fn http_url() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("http://docs.rs/foo/1.2.3", "foo", &ver),
            Err(String::from("expected \"https\", found \"http\""))
        );
    }

    #[test]
    fn bad_scheme() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("mailto:foo@example.net", "foo", &ver),
            Err(String::from("expected \"https\", found \"mailto\""))
        );
    }

    #[test]
    fn no_package() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs", "foo", &ver),
            Err(String::from("missing package name"))
        );
    }

    #[test]
    fn no_package_trailing_slash() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/", "foo", &ver),
            Err(String::from("missing package name"))
        );
    }

    #[test]
    fn no_version() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo", "foo", &ver),
            Err(String::from("missing version number"))
        );
    }

    #[test]
    fn no_version_trailing_slash() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo/", "foo", &ver),
            Err(String::from("missing version number"))
        );
    }

    #[test]
    fn bad_url() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("docs.rs/foo/bar", "foo", &ver),
            Err(String::from("parse error: relative URL without a base"))
        );
    }

    #[test]
    fn bad_pkg_version() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo/1.2.bad/", "foo", &ver),
            Err(String::from(
                "could not parse version in URL: \
                 unexpected character 'b' while parsing patch version number"
            ))
        );
    }

    #[test]
    fn wrong_pkg_name() {
        let ver = Version::parse("1.2.3").unwrap();
        assert_eq!(
            url_matches("https://docs.rs/foo/1.2.3/", "bar", &ver),
            Err(String::from("expected package \"bar\", found \"foo\""))
        );
    }
}

#[cfg(test)]
mod test_check_html_root_url {
    use super::*;

    #[test]
    fn bad_path() {
        let no_such_file = if cfg!(unix) {
            "No such file or directory (os error 2)"
        } else {
            "The system cannot find the file specified. (os error 2)"
        };
        let errmsg = format!("could not read no-such-file.md: {no_such_file}");
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
                "bad package version \"1.2\": unexpected end of input while parsing minor version number"
            ))
        );
    }
}
