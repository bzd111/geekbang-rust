use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrepError {
    #[error("invalid input: {0}")]
    GlobPatternError(#[from] glob::PatternError),
    #[error("Regex pattern error")]
    RegexPattrnError(#[from] regex::Error),
    #[error("IO error")]
    IoError(#[from] std::io::Error),
}
