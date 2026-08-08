//! Errors exposed by the deliberately small WSOLA API.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsolaError {
    InvalidRatio(String),
    InputTooShort {
        provided: usize,
        minimum: usize,
    },
    BufferOverflow {
        buffer: &'static str,
        requested: usize,
        available: usize,
    },
    InvalidState(&'static str),
}

impl fmt::Display for WsolaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRatio(message) => write!(f, "invalid stretch ratio: {message}"),
            Self::InputTooShort { provided, minimum } => {
                write!(
                    f,
                    "input too short: {provided} samples provided, {minimum} required"
                )
            }
            Self::BufferOverflow {
                buffer,
                requested,
                available,
            } => write!(
                f,
                "buffer overflow in {buffer}: requested {requested}, available {available}"
            ),
            Self::InvalidState(message) => write!(f, "invalid state: {message}"),
        }
    }
}

impl std::error::Error for WsolaError {}
