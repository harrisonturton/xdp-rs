// TODO: Remove this when possible
#![allow(dead_code)]
#![feature(strict_provenance)]

pub mod constants;
pub mod error;
pub mod ring;
pub mod ring2;
pub mod sys;
pub mod umem;

pub type Result<T> = std::result::Result<T, error::Error>;
