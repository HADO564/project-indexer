use thiserror::Error;

#[derive(Debug, Error)]
pub enum IconError {
    #[error("icon is too large: {size} bytes (maximum {max})")]
    TooLarge { size: usize, max: usize },

    #[error("icon is not valid XML: {0}")]
    Malformed(String),

    #[error("icon has no <svg> root")]
    NotAnSvg,

    #[error("icon has nothing drawable left after sanitizing")]
    NothingDrawable,

    #[error("icon store error: {0}")]
    Io(String),
}
