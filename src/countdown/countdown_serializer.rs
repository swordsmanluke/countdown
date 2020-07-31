use crate::countdown::Countdown;
use std::io::Error;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum CountdownError {
    NotFound(String),
    SaveError(String),
    IoError(std::io::Error)
}
impl fmt::Display for CountdownError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let msg = match self {
            CountdownError::NotFound(missing) => format!("{} was not found", missing),
            CountdownError::SaveError(name) => format!("Failed to save {}", name),
            CountdownError::IoError(err) => format!("IO Error: {}", err)
        };
        write!(f, "{}", msg)
    }
}
impl std::error::Error for CountdownError {}

pub trait CountdownService {
    fn save(&mut self, cd: Countdown) -> Result<(), CountdownError>;
    fn load(&mut self, name: &str) -> Result<Countdown, CountdownError>;
    fn delete(&mut self, name: &str) -> Result<(), CountdownError>;
    fn list(&mut self) -> Result<Vec<String>, CountdownError>;
}

impl From<std::io::Error> for CountdownError {
    fn from(err: Error) -> Self {
        CountdownError::IoError(err)
    }
}