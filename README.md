# Keep your Version Numbers in Sync with Cargo.toml

[![](https://img.shields.io/crates/v/version-sync.svg)][crates-io]
[![](https://docs.rs/version-sync/badge.svg)][api-docs]
[![](https://travis-ci.org/mgeisler/version-sync.svg?branch=master)][travis-ci]
[![](https://ci.appveyor.com/api/projects/status/jvvihnnct0pubudv?svg=true)][appveyor]

The `version-sync` crate will help you keep your version numbers in
sync with the crate version defined in `Cargo.toml`.

Rust projects typically reference this version number in several
places, such as the `README.md` file. The `version-sync` crate makes
it easy to add an integration test that checks that `README.md` is
updated when the crate version changes.

## Usage

Add this to your `Cargo.toml`:
```toml
[dev-dependencies]
version-sync = "0.2"
```

Then create a `tests/version-numbers.rs` file with:
```rust
#[macro_use]
extern crate version_sync;

#[test]
fn test_readme_deps() {
    assert_markdown_deps_updated!("README.md");
}

#[test]
fn test_html_root_url() {
    assert_html_root_url_updated!("src/lib.rs");
}
```

This integration test will ensure that the dependencies mentioned in
your `README.md` file is kept in sync with your crate version and that
your `html_root_url` points to the correct documentation on docs.rs.
If everything is well, the test passes:

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
version, the tests fail. In this example, the version number in
`Cargo.toml` has been changed to 0.2.0 while the `README.md` and
`html_root_url` remain unchanged. The tests now fail and the
problematic TOML code and attribute are shown:

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
    version-sync = "0.1"

thread 'test_readme_deps' panicked at 'dependency errors in README.md', tests/version-numbers.rs:6
note: Run with `RUST_BACKTRACE=1` for a backtrace.

---- test_html_root_url stdout ----
	Checking doc attributes in src/lib.rs...
src/lib.rs (line 48) ... expected minor version 2, found 1 in
    #![doc(html_root_url = "https://docs.rs/version-sync/0.1.3")]

thread 'test_html_root_url' panicked at 'html_root_url errors in src/lib.rs', tests/version-numbers.rs:11


failures:
    test_html_root_url
    test_readme_deps

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured

error: test failed
```

## Release History

This is a changelog describing the most important changes per release.

### Version 0.2.0 — September 19th, 2017

Added `assert_html_root_url_updated!` which will check that the
`html_root_url` attribute points to the correct version of the crate
documentation on docs.rs.

### Version 0.1.3 — September 18th, 2017

First public release with support for finding outdated version numbers
in `dependencies` and `dev-dependencies`.

Versions 0.1.0 to 0.1.2 were released under the name `check-versions`.

## License

Version-sync can be distributed according to the [MIT license][mit].
Contributions will be accepted under the same license.

[crates-io]: https://crates.io/crates/version-sync
[api-docs]: https://docs.rs/version-sync/
[travis-ci]: https://travis-ci.org/mgeisler/version-sync
[appveyor]: https://ci.appveyor.com/project/mgeisler/version-sync
[mit]: LICENSE
