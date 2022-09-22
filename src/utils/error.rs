use std::num::ParseIntError;

/// All errors possible to occur during reconciliation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Any error originating from the `kube-rs` crate
    #[error("Kubernetes reported error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    /// Error in user input or typically missing fields.
    #[error("Invalid User Input: {0}")]
    UserInputError(String),
    /// Error in while converting the string to int
    #[error("Invalid Upscaler CRD: {source}")]
    ParseError {
        #[from]
        source: ParseIntError,
    },
    #[error("CSV Error: {source}")]
    CSVError {
        #[from]
        source: csv::Error,
    },

    #[error("IO Error: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
    },

    #[error("Reqwest Error: {source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("Missing input: {0}")]
    MissingRequiredArgument(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::UserInputError(s)
    }
}
