#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to create socket: {0}")]
    Socket(i32),
    #[error("failed to bind: {0}")]
    Bind(i32),
    #[error("failed to bpf: {0}")]
    Bpf(i32),
    #[error("failed to if_nametoindex: {0}")]
    IfNameToIndex(i32),
    #[error("failed to mmap: {0}")]
    Mmap(i32),
    #[error("failed to munmap: {0}")]
    Munmap(i32),
    #[error("failed to setsockopt: {0}")]
    SetSockOpt(i32),
    #[error("failed to getsockopt: {0}")]
    GetSockOpt(i32),
    #[error("invalid argument: {0}")]
    InvalidArgument(&'static str),
    #[error("EFAULT: {0}")]
    Efault(&'static str),
}
