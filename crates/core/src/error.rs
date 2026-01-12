use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

pub type VnResult<T> = Result<T, VnError>;

#[derive(Debug, Error, Diagnostic)]
pub enum VnError {
    #[error("script validation failed: {0}")]
    #[diagnostic(code("vn.invalid_script"))]
    InvalidScript(String),
    #[error("script exhausted")]
    #[diagnostic(code("vn.end_of_script"))]
    EndOfScript,
    #[error("choice index out of range")]
    #[diagnostic(code("vn.invalid_choice"))]
    InvalidChoice,
    #[error("resource limit exceeded: {0}")]
    #[diagnostic(code("vn.resource_limit"))]
    ResourceLimit(String),
    #[error("security policy violation: {0}")]
    #[diagnostic(code("vn.security_policy"))]
    SecurityPolicy(String),
    #[error("serialization error: {message}")]
    #[diagnostic(code("vn.serialization"))]
    Serialization {
        message: String,
        #[source_code]
        src: String,
        #[label("here")]
        span: SourceSpan,
    },
    #[error("binary format error: {0}")]
    #[diagnostic(code("vn.binary_format"))]
    BinaryFormat(String),
}
