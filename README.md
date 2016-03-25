# etcd

[![Build Status](https://travis-ci.org/jimmycuadra/rust-etcd.svg?branch=master)](https://travis-ci.org/jimmycuadra/rust-etcd)

An [etcd](https://github.com/coreos/etcd) client library for Rust.

* [etcd](https://crates.io/crates/etcd) on crates.io
* [Documentation](http://jimmycuadra.github.io/rust-etcd/etcd/) for the latest crates.io release

## Nightly Rust

If you're using etcd in a program that is building on nightly Rust, use this feature profile:

``` toml
[dependencies.etcd]
default-features = false
features = ["nightly"]
version = "whatever version you want"
```

If you're building etcd directly from source, the equivalent Cargo commmand is `cargo build --no-default-features --features nightly`.

## Running the tests

* Install Docker and Docker Compose.
* Run `make`. This will drop you into a Bash shell in a container.
* Inside the container, run `cargo test`.

## License

[MIT](http://opensource.org/licenses/MIT)
