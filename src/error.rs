use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodonxError {
    #[error("unmatched directive at {file}:{line}: {message}")]
    Directive {
        file: String,
        line: usize,
        message: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
}
