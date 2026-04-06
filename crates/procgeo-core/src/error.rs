use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("attribute not found: {0}")]
    AttributeNotFound(String),

    #[error("attribute type mismatch: {0}")]
    AttributeTypeMismatch(String),

    #[error("group not found: {0}")]
    GroupNotFound(String),

    #[error("invalid handle")]
    InvalidHandle,

    #[error("invalid topology")]
    InvalidTopology,
}
