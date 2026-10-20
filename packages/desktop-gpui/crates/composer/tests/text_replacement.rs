//! Runs the tests of the text replacement lookup in the vendored gpui-base input. That crate is
//! not a workspace member, so cargo cannot test it directly.
#[path = "../../../vendor/gpui-base/src/input/base/text_replacement.rs"]
mod text_replacement;
