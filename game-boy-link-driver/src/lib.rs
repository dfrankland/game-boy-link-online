#![no_std]

mod controller;
mod peripheral;

pub use controller::*;
pub use peripheral::*;

pub const BUF_LEN: usize = 2048;
