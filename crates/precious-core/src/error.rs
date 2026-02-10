use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreciousError {
    #[error("failed to parse HCL: {0}")]
    HclParse(String),

    #[error("unsupported resource type: {0}")]
    UnsupportedResource(String),

    #[error("missing required field '{field}' on resource '{resource}'")]
    MissingField { resource: String, field: String },

    #[error("invalid value for field '{field}' on resource '{resource}': {reason}")]
    InvalidField {
        resource: String,
        field: String,
        reason: String,
    },

    #[error("pricing lookup failed: {0}")]
    PricingError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}
