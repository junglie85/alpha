use crate::error::Error;
use flexi_logger::Logger;

pub fn init(max_level: &str) -> Result<(), Error> {
    Logger::try_with_str(max_level)?.start()?;

    Ok(())
}
