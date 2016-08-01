use hyper::error::Error as HyperError;
use std::io::Error as StdIOError;
use serde_json::Error as SerdeError;

error_chain! {
    links { }

    foreign_links { 
        HyperError, HyperError, "Hyper Error";
        StdIOError, IOError, "IO Error";
        SerdeError, JsonError, "JSON Error";
    }

    errors { 
        InternalError(t: String) {
            description("internal error")
            display("Internal error: '{}'",  t)
        }
    }
}