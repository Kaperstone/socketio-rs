//! A socket.io message.

#![allow(unused_imports)]

use std::fmt::{Display, format, Formatter, Result as FmtResult};
use std::io::{BufRead, Error as IoError, ErrorKind, Read, Result as IoResult, Write};
use std::str::{FromStr, from_utf8};
use ::SocketError;
use rustc_serialize::{Decodable, Encodable};
use rustc_serialize::json::{Builder, Json, Parser, ToJson};

const ACK_PACKET_WITHOUT_ID: &'static str = "Received ack packet without an ID.";
const ATTACHMENT_ARRAY_TOO_SHORT: &'static str = "Attachments could not be reattached because the attachment array was too short.";
const BUFFER_UNEXPECTED_EOF: &'static str = "Packet type could not be read because the end of the buffer string was reached.";
const MESSAGE_PACKET_ARRAY_MISSING: &'static str = "Packet could not be parsed because the message JSON data was not an array.";
const MESSAGE_PACKET_ARRAY_TOO_SHORT: &'static str = "Packet could not be parsed because the data array did not contain enough elements.";
const MESSAGE_PACKET_NAME_NOT_STRING: &'static str = "Packet could not be parsed because the array element that should've been the packet's name was not a string.";
const UNREACHABLE_UNWRAP_FAILED: &'static str = "This unwrap while parsing a packet should never fail. Please contact the developers of NeoLegends/socketio-rs.";

/// A socket.io message.
#[derive(Clone, Debug, PartialEq, RustcEncodable)]
pub struct Message {
    body: Body,
    namespace: String
}

impl Message {
    /// Constructs a new `Message` from a namespace and a body.
    pub fn new(nsp: &str, body: Body) -> Self {
        Message::_new(nsp.to_owned(), body)
    }

    /// Constructs a new `Message` with the default namespace.
    pub fn with_default_namespace(body: Body) -> Self {
        Message::new("/", body)
    }

    fn _new(nsp: String, body: Body) -> Self {
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

    /// Reattaches detached binary data that is sent after the packet.
    ///
    /// ## Remarks
    /// - You generally shouldn't use this method yourself. Reattaching the
    ///   binary attachments will be done by socketio-rs.
    /// - Traverses the JSON tree up to a depth of 512 elements.
    pub fn reconstruct(&mut self, attachments: &Vec<Vec<u8>>) -> Result<(), SocketError> {
        if let Body::BinaryEvent { attachment_count, ref mut data, .. } = self.body {
            assert!((attachment_count as usize) >= attachments.len(), "Not enough attachments!");
            reconstruct(data, attachments, 512)
        } else {
            Ok(())
        }
    }
}

impl FromStr for Message {
    type Err = SocketError;

    /// Parses a `Message` from a string slice.
    fn from_str(buf: &str) -> Result<Self, Self::Err> {
        let mut chars = buf.chars().peekable();
        match chars.nth(0) {
            Some(ch @ '0' ... '6') => {
                let att_count = match ch {
                    '5' | '6' => {
                        let att_c_res = chars.by_ref().take_while(|&ch| ch != '-')
                                                      .collect::<String>()
                                                      .parse::<u32>();
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

                Ok(Message::_new(nsp, match ch {
                    '0' => Body::Connect,
                    '1' => Body::Disconnect,
                    '2' => {
                        let (name, body) = try!(get_name_and_body(json_body.expect(UNREACHABLE_UNWRAP_FAILED)));
                        Body::Event {
                            data: body,
                            id: id,
                            name: name
                        }
                    },
                    '3' => Body::Ack(id.expect(UNREACHABLE_UNWRAP_FAILED)),
                    '4' => Body::Error(json_body.expect(UNREACHABLE_UNWRAP_FAILED)),
                    '5' => {
                        let (name, body) = try!(get_name_and_body(json_body.expect(UNREACHABLE_UNWRAP_FAILED)));
                        Body::BinaryEvent {
                            attachment_count: att_count.unwrap(),
                            data: body,
                            id: id,
                            name: name
                        }
                    },
                    '6' => Body::BinaryAck(id.unwrap()),
                    _ => unreachable!("This packet type case should never be reached since the parent match should already catch it.")
                }))
            },
            Some(ch) => {
                let msg = format(format_args!("Message type could not be parsed because an illegal character was read: '{}'. Legal values are in the range of [0, 6].", ch));
                Err(SocketError::invalid_data(msg))
            },
            None => Err(SocketError::Io(IoError::new(ErrorKind::UnexpectedEof, BUFFER_UNEXPECTED_EOF)))
        }
    }
}

/// The body of a socket.io message.
#[derive(Clone, Debug, PartialEq, RustcEncodable)]
pub enum Body {
    /// Sent to the server in order to connect to a new namespace.
    Connect,

    /// Sent to the server in order to disconnect from a namespace.
    Disconnect,

    /// An actual socket message / event.
    Event {
        data: Json,
        id: Option<i32>,
        name: String
    },

    /// Send in response to an event to confirm its reception.
    Ack(i32),

    /// An error.
    Error(Json),

    /// A socket message containing binary data.
    BinaryEvent {
        attachment_count: u32,
        data: Json,
        id: Option<i32>,
        name: String
    },

    /// An ack response for binary messages.
    BinaryAck(i32)
}

/// Recursively reconstructs the given JSON message according to socket.io rules.
///
/// ## Parameters
/// - `body: &mut Json`: The JSON message to reconstruct.
/// - `attachments: &Vec<Vec<u8>>`: The binary attachments.
/// - `max_depth: u32`: The maximum depth to search for placeholders in.
pub fn reconstruct(body: &mut Json, attachments: &Vec<Vec<u8>>, max_depth: u32) -> Result<(), SocketError> {
    _reconstruct(body, attachments, max_depth, 0)
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

fn _reconstruct(body: &mut Json, attachments: &Vec<Vec<u8>>, max_depth: u32, depth: u32) -> Result<(), SocketError> {
    if depth >= max_depth {
        return Ok(());
    }

    if body.is_object() {
        // We do not use destructuring here because then the hash map would be
        // borrowed for the entire block and we couldn't replace the placeholder
        // with the binary data.
        // Hoping for #811 to land soon.

        let possible_index = {
            let map = body.as_object().expect(UNREACHABLE_UNWRAP_FAILED);
            if let Some(&Json::Boolean(true)) = map.get("_placeholder") {
                match map.get("num") {
                    Some(&Json::U64(num)) => Some(num as usize),
                    Some(&Json::I64(num)) => Some(num as usize),
                    _ => None
                }
            } else {
                None
            }
        };

        if let Some(index) = possible_index { // We're dealing with a placeholder
            if let Some(vec) = attachments.get(index) {
                *body = vec.to_json();
                Ok(())
            } else {
                // This case should hopefully never happen in real life, since
                // we're controlling the length of the binary array before reconstructing.
                // Might happen in some edge cases though. Yet we're not panicing here
                // since we're dealing with invalid JSON and not a bug in the code.
                Err(SocketError::invalid_data(ATTACHMENT_ARRAY_TOO_SHORT))
            }
        } else { // We're dealing with a top-level object, need to search through the children
            if let Json::Object(ref mut map) = *body {
                for value in map.values_mut() {
                    try!(_reconstruct(value, attachments, max_depth, depth + 1));
                }
                Ok(())
            } else {
                unreachable!(UNREACHABLE_UNWRAP_FAILED);
            }
        }
    } else if let Json::Array(ref mut arr) = *body {
        for value in arr.iter_mut() {
            try!(_reconstruct(value, attachments, max_depth, depth + 1));
        }
        Ok(())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{get_name_and_body};
    use std::collections::BTreeMap;
    use rustc_serialize::json::ToJson;

    #[test]
    fn name_and_body() {
        let j_data = vec!["Hello".to_json(), vec![1, 2, 3, 4, 5, 6].to_json()].to_json();
        let (name, body) = get_name_and_body(j_data).expect("Failed to get name and body.");

        assert_eq!(&name, "Hello");
        assert_eq!(body, vec![1, 2, 3, 4, 5, 6].to_json());
    }

    #[test]
    fn parse_str() {
        let s = r#"2["test-s-string",{"server":"Hello"}]"#;
        let m = s.parse::<Message>().expect("Failed to parse message from string.");

        assert_eq!("/", m.namespace());
        if let Body::Event { ref data, id, ref name } = *m.body() {
            let mut object = BTreeMap::new();
            object.insert("server".to_owned(), "Hello".to_json());
            let object = object.to_json();

            assert_eq!(name, "test-s-string");
            assert_eq!(id, None);
            assert_eq!(data.clone(), object);
        } else {
            panic!("Message body wasn't an event body.");
        }
    }

    #[test]
    fn parse_str_and_reconstruct() {
        let s = r#"52-["test-s-buf",[{"_placeholder":true,"num":0},{"_placeholder":true,"num":1}]]"#;
        let mut m = s.parse::<Message>().expect("Failed to parse message from string.");
        let b_data = vec![vec![1u8, 2u8, 3u8], vec![4u8, 5u8, 6u8]];
        m.reconstruct(&b_data).expect("Reconstructing failed.");

        assert_eq!("/", m.namespace());
        if let Body::BinaryEvent { attachment_count, ref data, id, ref name } = *m.body() {
            let j_b_data = b_data.to_json();

            assert_eq!(attachment_count, 2);
            assert_eq!(name, "test-s-buf");
            assert_eq!(id, None);
            assert_eq!(data.clone(), j_b_data);

            println!("{:?}\n{:?}", m, j_b_data);
        } else {
            panic!("Message body wasn't a binary event body.");
        }
    }

    #[test]
    fn reconstruct_raw_1() {
        let mut object1 = BTreeMap::new();
        object1.insert("_placeholder".to_owned(), true.to_json());
        object1.insert("num".to_owned(), 0.to_json());

        let object2 = object1.clone();
        let mut objects = vec![object1.to_json(), object2.to_json()].to_json();
        let b_data = vec![vec![1u8, 2u8, 3u8], vec![4u8, 5u8, 6u8]];

        reconstruct(&mut objects, &b_data, 512).expect("Inserting the attachments failed.");

        let ideal_result = vec![vec![1u64, 2u64, 3u64], vec![1u64, 2u64, 3u64]].to_json();
        assert_eq!(objects, ideal_result);
    }

    #[test]
    fn reconstruct_raw_2() {
        let mut placeholder = BTreeMap::new();
        placeholder.insert("_placeholder".to_owned(), true.to_json());
        placeholder.insert("num".to_owned(), 0.to_json());

        let mut object = BTreeMap::new();
        object.insert("member1".to_owned(), true.to_json());
        object.insert("b_data".to_owned(), placeholder.to_json());
        let mut object = object.to_json();

        let b_data = vec![vec![1u8, 2u8, 3u8]];

        reconstruct(&mut object, &b_data, 512).expect("Inserting the attachments failed.");

        let mut ideal_result = BTreeMap::new();
        ideal_result.insert("member1".to_owned(), true.to_json());
        ideal_result.insert("b_data".to_owned(), vec![1u8.to_json(), 2u8.to_json(), 3u8.to_json()].to_json());

        assert_eq!(object, ideal_result.to_json());
    }
}