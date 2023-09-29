// TODO: Remove this when possible
#![allow(dead_code)]
#![feature(strict_provenance)]

mod cli;
mod constants;
mod error;
mod ring;
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
        Err(error::Error::Socket(code)) => {
            println!("socket error: {}", sys::strerror(code));
            return Ok(());
        }
        Err(error::Error::SetSockOpt(code)) => {
            println!("setsockopt error: {}", sys::strerror(code));
            return Ok(());
        }
        Err(error::Error::GetSockOpt(code)) => {
            println!("getsockopt error: {}", sys::strerror(code));
            return Ok(());
        }
        Err(error::Error::Bind(code)) => {
            println!("bind error: {}", sys::strerror(code));
            return Ok(());
        }
        Err(error::Error::IfNameToIndex(code)) => {
            println!("if_nametoindex error: {}", sys::strerror(code));
            return Ok(());
        }
        _ => Ok(()),
    }
}
