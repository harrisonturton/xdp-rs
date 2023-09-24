#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to create socket: {0}")]
    CreateSocket(&'static str),
}

#[derive(Debug)]
pub enum CheckResult {
    Pass(i32),
    Fail(i32),
}

#[must_use]
pub fn check(code: i32) -> CheckResult {
    if code < 0 {
        CheckResult::Fail(code)
    } else {
        CheckResult::Pass(code)
    }
}

impl CheckResult {
    #[must_use]
    pub fn or_err<E: std::error::Error>(self, err: E) -> Result<i32, E> {
        match self {
            Self::Pass(code) => Ok(code),
            Self::Fail(code) => Err(err),
        }
    }

    #[must_use]
    pub fn map_err<F, O: FnOnce(i32) -> F>(self, op: O) -> Result<i32, F> {
        match self {
            Self::Pass(code) => Ok(code),
            Self::Fail(code) => Err(op(code)),
        }
    }
}
