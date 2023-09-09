# Keep your Version Numbers in Sync with Cargo.toml

[![](https://github.com/mgeisler/version-sync/workflows/build/badge.svg)][build-status]
[![](https://codecov.io/gh/mgeisler/version-sync/branch/master/graph/badge.svg)][codecov]
[![](https://img.shields.io/crates/v/version-sync.svg)][crates-io]
[![](https://docs.rs/version-sync/badge.svg)][api-docs]

Rust projects typically reference the crate version number in several
places, such as the `README.md` file. The version-sync crate makes it
easy to add an integration test that checks that `README.md` is
updated when the crate version changes.

## Usage

Add this to your `Cargo.toml`:
```toml
[dev-dependencies]
version-sync = "0.9"
```

Then create a `tests/version-numbers.rs` file with:
```rust
#[test]
fn test_readme_deps() {
    version_sync::assert_markdown_deps_updated!("README.md");
}

#[test]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
```

This integration test will ensure that the dependencies mentioned in
your `README.md` file are kept in sync with your crate version and
that your `html_root_url` points to the correct documentation on
docs.rs. If everything is well, the test passes:

```
$ cargo test
    Finished debug [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/version_numbers-504f17c82f1defea

running 2 tests
test test_readme_deps ... ok
test test_html_root_url ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured
```

If the README or `html_root_url` is out of sync with the crate
version, the tests fail. In this example, the crate is called
`your-crate` and the version number in `Cargo.toml` has been changed
to 0.2.0 while the `README.md` and `html_root_url` still use 0.1.2.
The tests now fail and the problematic TOML code and attribute are
shown:

```
$ cargo test
    Finished debug [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/version_numbers-f399bac3e468d035

running 2 tests
test test_readme_deps ... FAILED
test test_html_root_url ... FAILED

failures:

---- test_readme_deps stdout ----
	Checking code blocks in README.md...
README.md (line 20) ... expected minor version 2, found 1 in
    [dev-dependencies]
    your-crate = "0.1"

thread 'test_readme_deps' panicked at 'dependency errors in README.md', tests/version-numbers.rs:6
note: Run with `RUST_BACKTRACE=1` for a backtrace.

---- test_html_root_url stdout ----
	Checking doc attributes in src/lib.rs...
src/lib.rs ... expected minor version 2, found 1 in
    #![doc(html_root_url = "https://docs.rs/your-crate/0.1.2")]

thread 'test_html_root_url' panicked at 'html_root_url errors in src/lib.rs', tests/version-numbers.rs:11


failures:
    test_html_root_url
    test_readme_deps

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured

error: test failed
```

### Excluding TOML Code

You can add `no_sync` to the language line in a code block to exclude
it from the checks done by `assert_markdown_deps_updated!`:

~~~markdown
```toml,no_sync
[dependencies]
your_crate = "0.1.2"
```
~~~

## Release History

This is a changelog describing the most important changes per release.

### Version 0.9.4 — 2021-12-14

* [#115](https://github.com/mgeisler/version-sync/pull/115): Expand
  `assert_markdown_deps_updated!` to also check version numbers such
  as `=1.2.3`, `>1.2.3`, and `>=1.2.3`.
* [#119](https://github.com/mgeisler/version-sync/pull/119): Add new
  `assert_only_contains_regex!` macro which can ensure that all
  version numbers in a file are updated.

### Version 0.9.3 — 2021-09-20

* [#108](https://github.com/mgeisler/version-sync/pull/108): Make all
  dependencies optional. Thanks @rlee287!
* [#109](https://github.com/mgeisler/version-sync/pull/109): Forbid
  the use of unsafe code.
* [#110](https://github.com/mgeisler/version-sync/pull/110): Add
  simple `check_contains_version` function.

### Version 0.9.2 — 2021-02-13

* [#94](https://github.com/mgeisler/version-sync/pull/94): Update
  pulldown-cmark to 0.8.
* [#95](https://github.com/mgeisler/version-sync/pull/95): Fix
  `non_fmt_panic` lint error in latest nightly.

### Version 0.9.1 — 2020-07-07

* [#91](https://github.com/mgeisler/version-sync/pull/91): Pull in
  fewer dependencies. This optimizes the build time by 1-2 seconds.
* [#92](https://github.com/mgeisler/version-sync/pull/92): Normalize
  `\r\n` to `\n` to ensure `^` and `$` always match line boundaries.

### Version 0.9.0 — 2020-03-30

Drop support for Rust 1.31.0 since our dependencies keep releasing new
patch versions that push up the minimum required Rust version. At the
time of writing, the code compiles with Rust 1.36, but this will
likely become outdated soon.

Issues closed:

* [#83][issue-83]: version-sync fails to parse TOML blocks when
  inside blockquotes
* [#84][issue-84]: Release update to crates.io with syn 1.0

### Version 0.8.1 — 2019-04-03

Dependencies were relaxed to make it easier to upgrade version-sync.

### Version 0.8.0 — 2019-03-28

We now use [Rust 2018][rust-2018], which means we require Rust version
1.31.0 or later. The `assert_html_root_url_updated!` macro will again
report accurate line numbers based on span information from the syn
crate.

### Version 0.7.0 — 2019-01-14

Special characters are now correctly escaped in the `{name}` and
`{version}` placeholders in `assert_contains_regex!`.

Dependencies were updated and version-sync now requires Rust version
1.27.2 or later.

### Version 0.6.0 — 2018-11-22

You can use `assert_contains_regex!` to grep files for the current
version number. The search is done with a regular expression where
`{version}` is replaced with the current version number.

Git dependencies are now always accepted, which means that blocks like

~~~markdown
```toml
[dependencies]
your_crate = { git = "..." }
```
~~~

will work without you having to add `no_sync`.

Issues closed:

* [#17][issue-17]: Allow to check non-markdown sources
* [#39][issue-39]: Version 0.5 requires Rust version 1.21.0
* [#42][issue-42]: Handle Git dependencies


### Version 0.5.0 — 2017-11-19

Dependencies were updated and version-sync now requires Rust version
1.21 or later.

Error messages from `assert_html_root_url_updated!` now again include
line numbers (based on a heuristic until the syn crate can provide the
information).

### Version 0.4.0 — 2017-11-01

This release replaces the dependency on the abandoned syntex_syntax
with with a dependency on the much lighter syn crate. This improves
compilation speed. Unfortunately, the syn crate does not provide
information about line numbers, so error messages are are no longer as
good. We might be able to work around that in a later version.

### Version 0.3.1 — 2017-09-26

This release fixes a small problem with the handling of pre-release
identifiers.

Issues closed:

* [#19][issue-19]: Pre-release identifiers were ignored.

### Version 0.3.0 — 2017-09-23

When checking dependencies in READMEs, TOML blocks can now be excluded
from the check by adding `no_sync` to the language line:

~~~markdown
```toml,no_sync
[dependencies]
your_crate = "0.1"
```
~~~

This TOML block will not be checked. This is similar to `no_run` for
Rust code blocks.

### Version 0.2.0 — 2017-09-20

Added `assert_html_root_url_updated!` which will check that the
`html_root_url` attribute points to the correct version of the crate
documentation on docs.rs.

### Version 0.1.3 — 2017-09-18

First public release with support for finding outdated version numbers
in `dependencies` and `dev-dependencies`.

Versions 0.1.0 to 0.1.2 were released under the name check-versions.

## License

Version-sync can be distributed according to the [MIT license][mit].
Contributions will be accepted under the same license.

[build-status]: https://github.com/mgeisler/version-sync/actions?query=workflow%3Abuild+branch%3Amaster
[codecov]: https://codecov.io/gh/mgeisler/version-sync
[crates-io]: https://crates.io/crates/version-sync
[api-docs]: https://docs.rs/version-sync/0.9.4/
[rust-2018]: https://doc.rust-lang.org/edition-guide/rust-2018/
[mit]: LICENSE
[issue-17]: https://github.com/mgeisler/version-sync/issues/17
[issue-19]: https://github.com/mgeisler/version-sync/issues/19
[issue-39]: https://github.com/mgeisler/version-sync/issues/39
[issue-42]: https://github.com/mgeisler/version-sync/issues/42
[issue-83]: https://github.com/mgeisler/version-sync/issues/83
[issue-84]: https://github.com/mgeisler/version-sync/issues/84
