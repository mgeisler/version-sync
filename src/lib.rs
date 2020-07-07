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
//! #[test]
//! # fn fake_hidden_test_case_1() {}
//! fn test_readme_deps() {
//!     version_sync::assert_markdown_deps_updated!("README.md");
//! }
//!
//! #[test]
//! # fn fake_hidden_test_case_2() {}
//! fn test_html_root_url() {
//!     version_sync::assert_html_root_url_updated!("src/lib.rs");
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

#![doc(html_root_url = "https://docs.rs/version-sync/0.9.1")]
#![deny(missing_docs)]

mod contains_regex;
mod helpers;
mod html_root_url;
mod markdown_deps;

pub use crate::contains_regex::check_contains_regex;
pub use crate::html_root_url::check_html_root_url;
pub use crate::markdown_deps::check_markdown_deps;

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
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_readme_deps is compiled.
/// fn test_readme_deps() {
///     version_sync::assert_markdown_deps_updated!("README.md");
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
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_html_root_url is compiled.
/// fn test_html_root_url() {
///     version_sync::assert_html_root_url_updated!("src/lib.rs");
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
/// #[test]
/// # fn fake_hidden_test_case() {}
/// # // The above function ensures test_readme_mentions_version is
/// # // compiled.
/// fn test_readme_mentions_version() {
///     version_sync::assert_contains_regex!("README.md", "^### Version {version}");
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
