use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("logging system failure")]
    Logging(#[from] flexi_logger::FlexiLoggerError),
    #[error("WASM system failure")]
    WASM(#[from] Box<dyn std::error::Error + Send + Sync>),
}
