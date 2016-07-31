use hyper::error::Error as HyperError;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

wrapped_enum! {
    #[derive(Debug)]
    /// Error
    pub enum Error {
        /// HyperError
        HyperError(HyperError),
        /// IOError
        IOError(::std::io::Error),
        /// StringError
        InternalError(String),
        /// JsonError
        JsonError(::serde_json::Error)
    }
}