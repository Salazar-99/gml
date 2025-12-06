#[derive(Debug)]
pub struct GmlError {}

impl std::fmt::Display for GmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GmlError")
    }
}

impl std::error::Error for GmlError {}

