pub mod errors;
pub mod process;

pub type ProxyResult<T> = Result<T, errors::ProxyErr>;
pub use process::*;
