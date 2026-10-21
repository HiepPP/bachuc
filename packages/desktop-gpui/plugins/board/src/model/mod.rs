//! The Board's wire types and rules without GPUI. Each rule is a pure function with unit tests
//! next to it; the views and the store only call them.

pub(crate) mod colors;
pub(crate) mod pending;
pub(crate) mod poll;
pub(crate) mod recaps;
pub(crate) mod remove;
pub(crate) mod run;
pub(crate) mod settings;
pub(crate) mod text;
pub(crate) mod timeline;
pub(crate) mod tree;
pub(crate) mod view_poll;

pub(crate) use poll::ConnectionState;
pub(crate) use run::{BoardSnapshot, Run, RunStatus};
pub(crate) use settings::{OrbSettings, OrbState};

/// The separator of every card and rail line.
pub(crate) const SEPARATOR: &str = " · ";
