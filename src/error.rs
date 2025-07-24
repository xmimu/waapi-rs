use std::fmt;

#[derive(Debug)]
pub enum WaapiError {
    WampError(String),
    SerdeError(serde_json::Error),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl std::error::Error for WaapiError {}

impl fmt::Display for WaapiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaapiError::WampError(msg) => write!(f, "WAMP error: {}", msg),
            WaapiError::SerdeError(e) => write!(f, "Serialization error: {}", e),
            WaapiError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl From<serde_json::Error> for WaapiError {
    fn from(e: serde_json::Error) -> Self {
        WaapiError::SerdeError(e)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for WaapiError {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        WaapiError::Other(e)
    }
}
