//! The Board's wire types and rules without GPUI. Each rule is a pure function with unit tests
//! next to it; the views and the store only call them.

pub(crate) mod colors;
pub(crate) mod poll;
pub(crate) mod remove;
pub(crate) mod run;
pub(crate) mod settings;
pub(crate) mod text;
pub(crate) mod tree;

pub(crate) use poll::ConnectionState;
pub(crate) use run::{BoardSnapshot, Run, RunStatus};
pub(crate) use settings::OrbSettings;
#[allow(unused_imports)] // TASK-193's orb reads the state.
pub(crate) use settings::OrbState;

/// The separator of every card and rail line.
pub(crate) const SEPARATOR: &str = " · ";
