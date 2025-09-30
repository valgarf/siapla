use std::fmt::Display;

use thiserror::Error;

pub mod app_state;
pub mod entity;
pub mod gql;
pub mod scheduling;

#[derive(Error, Debug)]
pub struct SiaplaError {
    msg: String,
}

impl SiaplaError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}
impl Display for SiaplaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}
