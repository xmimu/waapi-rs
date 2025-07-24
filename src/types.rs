use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WaapiProjectInfo {
    pub name: String,
    pub version: String,
}
