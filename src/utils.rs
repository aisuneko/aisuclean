use indicatif::ProgressBar;
use std::{
    fmt::{Display, Formatter, Result},
    io,
    ops::AddAssign,
    process::exit,
};

#[derive(Copy, Clone)]
pub struct DirData {
    pub size: u64,
    pub completed: u64,
    pub error: u64,
}
impl DirData {
    pub fn from(size: u64, completed: u64, error: u64) -> Self {
        Self {
            size: size,
            completed: completed,
            error: error,
        }
    }
}
impl AddAssign for DirData {
    // type Output = Self;
    fn add_assign(self: &mut DirData, other: Self) {
        *self = Self {
            size: self.size + other.size,
            completed: self.completed + other.completed,
            error: self.error + other.error,
        }
    }
}
#[allow(non_snake_case)]
pub fn SubDirError() -> AppError {
    AppError(String::from("is subdirectory or symlink"))
}
#[derive(Debug, PartialEq)]
pub struct AppError(String);
impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "{}", self.0)
    }
}
impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        AppError(error.to_string())
    }
}
impl From<walkdir::Error> for AppError {
    fn from(error: walkdir::Error) -> Self {
        AppError(error.to_string())
    }
}
pub fn make_pb(quiet: bool) -> ProgressBar {
    match quiet {
        true => ProgressBar::hidden(),
        false => ProgressBar::new(0x7ffffff),
    }
}
pub fn eq(x: String) -> ! {
    eprint!("ERROR: {}", x);
    exit(1);
}
