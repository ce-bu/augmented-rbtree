#![allow(dead_code)]

pub(crate) mod common;
pub(crate) mod dumper;
pub(crate) mod limited_allocator;

pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
