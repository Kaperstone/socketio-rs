//! A socket.io message.

#![allow(unused_imports)]

use std::fmt::{Display, format, Formatter, Result as FmtResult};
use std::io::{BufRead, Error as IoError, ErrorKind, Read, Result as IoResult, Write};
use std::str::from_utf8;
use ::SocketError;
use rustc_serialize::{Decodable, Encodable};
use rustc_serialize::json::{Builder, Json, Parser};

const ACK_PACKET_WITHOUT_ID: &'static str = "Received ack packet without an ID.";
const BUFFER_UNEXPECTED_EOF: &'static str = "Packet type could not be read because the end of the buffer string was reached.";
const MESSAGE_PACKET_ARRAY_MISSING: &'static str = "Packet could not be parsed because the message JSON data was not an array.";
const MESSAGE_PACKET_ARRAY_TOO_SHORT: &'static str = "Packet could not be parsed because the data array did not contain enough elements.";
const MESSAGE_PACKET_NAME_NOT_STRING: &'static str = "Packet could not be parsed because the array element that should've been the packet's name was not a string.";

/// A socket.io message.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    body: Body,
    namespace: String
}

impl Message {
    /// Constructs a new `Message` from a namespace and a body.
    pub fn new(nsp: &str, body: Body) -> Message {
        Message::_new(nsp.to_owned(), body)
    }

    /// Parses a `Message` from a string slice.
    pub fn from_str(buf: &str) -> Result<Message, SocketError> {
        let mut chars = buf.chars().peekable();
        match chars.nth(0) {
            Some(ch @ '0' ... '6') => {
                let att_count = match ch {
                    '5' | '6' => {
                        let att_c_res = chars.by_ref().take_while(|&ch| ch != '-')
                                                      .collect::<String>()
                                                      .parse::<i32>();
                        Some(try!(att_c_res))
                    },
                    _ => None
                };
                let nsp = match chars.peek().cloned() {
                    Some('/') => chars.by_ref().take_while(|&ch| ch != ',').collect::<String>(),
                    Some(_) => "/".to_owned(),
                    None => return Err(SocketError::Io(IoError::new(ErrorKind::UnexpectedEof, BUFFER_UNEXPECTED_EOF)))
                };
                let id = match chars.peek().cloned() {
                    Some('0' ... '9') => {
                        let parse_res = chars.by_ref().take_while(|&ch| ch.is_digit(10))
                                                      .collect::<String>()
                                                      .parse::<i32>();
                        Some(try!(parse_res))
                    },
                    Some(_) if ch == '3' || ch == '6' => return Err(SocketError::invalid_data(ACK_PACKET_WITHOUT_ID)),
                    Some(_) => None,
                    None => return Err(SocketError::Io(IoError::new(ErrorKind::UnexpectedEof, BUFFER_UNEXPECTED_EOF)))
                };
                let json_body = match ch {
                    '2' | '4' | '5' => Some(try!(Builder::new(chars).build())),
                    _ => None
                };

                Ok(match ch {
                    '0' => Message::_new(nsp, Body::Connect),
                    '1' => Message::_new(nsp, Body::Disconnect),
                    '2' => {
                        let (name, body) = try!(get_name_and_body(json_body.unwrap()));
                        Message::_new(nsp, Body::Event {
                            body: body,
                            id: id,
                            name: name
                        })
                    },
                    '3' => Message::_new(nsp, Body::Ack(id.unwrap())),
                    '4' => Message::_new(nsp, Body::Error(json_body.unwrap())),
                    '5' => {
                        let (name, body) = try!(get_name_and_body(json_body.unwrap()));
                        unimplemented!()
                    },
                    '6' => Message::_new(nsp, Body::BinaryAck(id.unwrap())),
                    _ => unreachable!("This packet type case should never be reached since the parent match should already catch it.")
                })
            },
            Some(ch) => {
                let msg = format(format_args!("Message type could not be parsed because an illegal character was read: '{}'. Legal values are in the range of [0, 6].", ch));
                Err(SocketError::invalid_data(msg))
            },
            None => Err(SocketError::Io(IoError::new(ErrorKind::UnexpectedEof, BUFFER_UNEXPECTED_EOF)))
        }
    }

    /// Constructs a new `Message` with the default namespace.
    pub fn with_default_namespace(body: Body) -> Message {
        Message::new("/", body)
    }

    fn _new(nsp: String, body: Body) -> Message {
        assert!(nsp.starts_with('/'));

        Message {
            body: body,
            namespace: nsp
        }
    }

    /// Gets the message's body.
    pub fn body(&self) -> &Body {
        &self.body
    }

    /// Gets the message's namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
}

/// The body of a socket.io message.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    /// Sent to the server in order to connect to a new namespace.
    Connect,

    /// Sent to the server in order to disconnect from a namespace.
    Disconnect,

    /// An actual socket message / event.
    Event {
        body: Json,
        id: Option<i32>,
        name: String
    },

    /// Send in response to an event to confirm its reception.
    Ack(i32),

    /// An error.
    Error(Json),

    /// A socket message containing binary data.
    BinaryEvent {
        body: Json,
        id: Option<i32>,
        name: String
    },

    /// An ack response for binary messages.
    BinaryAck(i32)
}

fn get_name_and_body(json_body: Json) -> Result<(String, Json), SocketError> {
    let mut arr = match json_body {
        Json::Array(vec) => vec,
        _ => return Err(SocketError::invalid_data(MESSAGE_PACKET_ARRAY_MISSING))
    };
    if arr.len() < 1 {
        return Err(SocketError::invalid_data(MESSAGE_PACKET_ARRAY_TOO_SHORT));
    }
    let name = match arr.remove(0) {
        Json::String(name) => name,
        _ => return Err(SocketError::invalid_data(MESSAGE_PACKET_NAME_NOT_STRING))
    };
    let body = if arr.len() > 0 {
        arr.remove(0)
    } else {
        Json::Null
    };

    Ok((name, body))
}

#[cfg(test)]
mod tests {
    use super::*;
}