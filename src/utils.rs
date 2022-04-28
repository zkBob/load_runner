#[derive(Debug)]
pub enum TestError {
    NetworkError(reqwest::Error),
    GeneratorError(String),
    SavingError(std::io::Error),
    SerializationError(serde_json::Error),
    ConfigError(String),
    BadResponse(String),
    MpscError
}