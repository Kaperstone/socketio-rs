//! A socket.io client library written in and for Rust.
//!
//! The first goal of this library is to reach feature-complete
//! status and full interoperability with the JS implementation.
//! Performance is always being worked on, though not focused on
//! primarily. Major improvements in that area can come once the
//! library is working properly and stable.

#![crate_name = "socketio"]
#![crate_type = "lib"]

extern crate engineio;
extern crate rustc_serialize;
extern crate url;

mod client;
mod error;
mod manager;
mod message;

pub use client::Client;
pub use error::SocketError;
pub use manager::Manager;
pub use message::Message;