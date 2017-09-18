# version-sync

[![](https://img.shields.io/crates/v/version-sync.svg)][crates-io]
[![](https://docs.rs/version-sync/badge.svg)][api-docs]
[![](https://travis-ci.org/mgeisler/version-sync.svg?branch=master)][travis-ci]
[![](https://ci.appveyor.com/api/projects/status/jvvihnnct0pubudv?svg=true)][appveyor]

The `version-sync` crate is a simple crate that will help you
remember to update the versions numbers in your `README.md` file.

## Usage

Add this to your `Cargo.toml`:
```toml
[dev-dependencies]
version-sync = "0.1"
```

Then create a `tests/version-numbers.rs` with:
```rust
#[macro_use]
extern crate version_sync;

#[test]
fn test_readme_deps() {
    assert_markdown_deps_updated!("README.md");
}
```

This test will ensure that the dependencies mentioned in your
`README.md` file is kept in sync with your crate version:
```
$ cargo test --test version-numbers -- --nocapture
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/version_numbers-3b40b9d452dd9385

running 1 test
Checking code blocks in README.md...
README.md (line 10) ... ok
test test_readme_deps ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Here the `README.md` file still references version 0.1.0, while the
version number in `Cargo.toml` has been changed to 0.2.0. The test
fails and the code block with the error is shown:

```
$ cargo test --test version-numbers -- --nocapture
    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs
     Running target/debug/deps/version_numbers-8fbc5f3b97f4ec3a

running 1 test
Checking code blocks in README.md...
README.md (line 10) ... expected minor version 2, found 1 in
    [dev-dependencies]
    version-sync = "0.1"

thread 'test_readme_deps' panicked at 'dependency errors in README.md', tests/version-numbers.rs:6:4
note: Run with `RUST_BACKTRACE=1` for a backtrace.
test test_readme_deps ... FAILED

failures:

failures:
    test_readme_deps

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out

error: test failed, to rerun pass '--test version-numbers'
```

## Release History

This is a changelog describing the most important changes per release.

### Version 0.1.3 — September 18th, 2017

Idential to version 0.1.2, but release under the name `version-sync`.

### Version 0.1.2 — September 18th, 2017

Identical to version 0.1.1, but with better deprecation notices.

### Version 0.1.1 — September 18th, 2017

The crate will be renamed to [`version-sync`][version-sync] and this
is the last release of the crate under the name `check-versions`.

Version 0.1.1 is identical in functionality to version 0.1.0, except
that using the crate will trigger a deprecation warning with
instructions to use [`version-sync`][version-sync] instead.

### Version 0.1.0 — September 10th, 2017

First public release with support for finding outdated version numbers
in `dependencies` and `dev-dependencies`.

## License

Version-sync can be distributed according to the [MIT license][mit].
Contributions will be accepted under the same license.

[crates-io]: https://crates.io/crates/version-sync
[api-docs]: https://docs.rs/version-sync/
[travis-ci]: https://travis-ci.org/mgeisler/version-sync
[appveyor]: https://ci.appveyor.com/project/mgeisler/version-sync
[mit]: LICENSE
