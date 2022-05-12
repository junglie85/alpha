use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("logging system failure")]
    Logging(#[from] flexi_logger::FlexiLoggerError),
}
