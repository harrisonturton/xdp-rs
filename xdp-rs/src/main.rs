// TODO: Remove this when possible
#![allow(dead_code)]

mod cli;
mod constants;
mod error;
mod sys;
mod umem;

pub type Result<T> = std::result::Result<T, error::Error>;

pub fn main() -> Result<()> {
    match cli::exec() {
        Ok(()) => Ok(()),
        Err(error::Error::Mmap(code)) => {
            println!("error: {}", sys::strerror(code));
            return Ok(());
        }
        Err(error::Error::SetSockOpt(code)) => {
            println!("error: {}", sys::strerror(code));
            return Ok(());
        }
        _ => Ok(()),
    }
}
