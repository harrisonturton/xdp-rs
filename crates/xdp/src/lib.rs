// TODO: Remove this when possible
#![allow(dead_code)]
#![feature(strict_provenance)]
#![feature(iterator_try_collect)]

pub mod channel;
pub mod constants;
pub mod error;
pub mod ring;
pub mod socket;
pub mod sys;
pub mod umem;

pub type Result<T> = std::result::Result<T, error::Error>;
