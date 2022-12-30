use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Version {
    HTTP11,
    HTTP2,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Type {
    HTTP,
    HTTPS,
}

impl Default for Type {
    fn default() -> Self {
        Self::HTTP
    }
}
