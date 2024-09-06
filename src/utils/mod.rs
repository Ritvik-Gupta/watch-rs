use std::error::Error;

pub type OpenResult<T=()> = Result<T, Box<dyn Error>>;