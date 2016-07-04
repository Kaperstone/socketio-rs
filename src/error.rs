use std::error::Error;
use std::fmt::{Display, Formatter, format, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind};
use std::num::{ParseFloatError, ParseIntError};
use engineio::{EngineError, Void};
use rustc_serialize::base64::FromBase64Error;
use rustc_serialize::json::ParserError;

/// An error within socket.io.
#[derive(Debug)]
pub enum SocketError {
    /// An error occured while parsing the base-64 encoded binary data.
    Base64(FromBase64Error),

    /// An error within engine.io occured.
    Engine(EngineError),

    /// The action could not be performed because the component was in
    /// an invalid state.
    InvalidState(Box<Error + Send + Sync>),

    /// An I/O error occured.
    Io(IoError),

    /// An error occured while parsing string data from UTF-8.
    Utf8,

    #[doc(hidden)]
    __Nonexhaustive(Void)
}

impl SocketError {
    /// Creates an `InvalidData`-variant.
    pub fn invalid_data<E: Into<Box<Error + Send + Sync>>>(err: E) -> SocketError {
        SocketError::Io(IoError::new(ErrorKind::InvalidData, err))
    }

    /// Creates a `SocketError::InvalidState` variant. Mainly
    /// used in combination with string literals.
    ///
    /// ## Example
    /// ```
    /// # use socketio::SocketError;
    /// let e = SocketError::invalid_state("Data was invalid.");
    /// ```
    pub fn invalid_state<E: Into<Box<Error + Send + Sync>>>(err: E) -> SocketError {
        SocketError::InvalidState(err.into())
    }
}

impl Display for SocketError {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str(self.description())
    }
}

impl Error for SocketError {
    fn description(&self) -> &str {
        match *self {
            SocketError::Base64(ref err) => err.description(),
            SocketError::Engine(ref err) => err.description(),
            SocketError::InvalidState(ref err) => err.description(),
            SocketError::Io(ref err) => err.description(),
            SocketError::Utf8 => "UTF-8 data was invalid.",
            _ => "Unknown socket.io error."
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            SocketError::Base64(ref err) => err.cause(),
            SocketError::Engine(ref err) => err.cause(),
            SocketError::InvalidState(ref err) => err.cause(),
            SocketError::Io(ref err) => err.cause(),
            SocketError::Utf8 => None,
            _ => None
        }
    }
}

impl From<ParserError> for SocketError {
    fn from(err: ParserError) -> Self {
        let io_err = match err {
            ParserError::IoError(err) => err,
            ParserError::SyntaxError(err, line, col) => {
                let msg = format(format_args!("JSON parser error '{:?}' on line {} and column {}.", err, line, col));
                IoError::new(ErrorKind::InvalidData, msg)
            }
        };
        SocketError::Io(io_err)
    }
}

impl From<FromBase64Error> for SocketError {
    fn from(err: FromBase64Error) -> Self {
        SocketError::Base64(err)
    }
}

impl From<EngineError> for SocketError {
    fn from(err: EngineError) -> Self {
        SocketError::Engine(err)
    }
}

impl From<IoError> for SocketError {
    fn from(err: IoError) -> Self {
        SocketError::Io(err)
    }
}

impl From<ParseFloatError> for SocketError {
    fn from(err: ParseFloatError) -> Self {
        SocketError::Io(IoError::new(ErrorKind::InvalidData, err.description()))
    }
}

impl From<ParseIntError> for SocketError {
    fn from(err: ParseIntError) -> Self {
        SocketError::Io(IoError::new(ErrorKind::InvalidData, err.description()))
    }
}