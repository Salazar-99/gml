#[derive(Debug)]
pub struct GmlError {
    pub message: String,
}

impl std::fmt::Display for GmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GmlError: {}", self.message)
    }
}

impl std::error::Error for GmlError {}

impl From<String> for GmlError {
    fn from(message: String) -> Self {
        GmlError { message }
    }
}

impl From<&str> for GmlError {
    fn from(message: &str) -> Self {
        GmlError { message: message.to_string() }
    }
}
