# socketio-rs

An socket.io client library in Rust. Runs only on Rust
nightly at the moment.

[![Build Status](https://travis-ci.org/NeoLegends/socketio-rs.svg?branch=master)](https://travis-ci.org/NeoLegends/socketio-rs)

## Usage

To use `socketio-rs`, first add this to your Cargo.toml:

```toml
[dependencies.socketio]
git = "https://github.com/NeoLegends/socketio-rs"
```

Then, add this to your crate root:

```rust
extern crate socketio;
```