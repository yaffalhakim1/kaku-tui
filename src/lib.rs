// kaku-tui library — exposes client + types to binary and examples.
//
// #![allow(dead_code)] — most of these modules hold fields/methods intended for
// future phases (cancel handling, full event taxonomy, etc.). The compiler will
// surface real "this is unused AND has no future" candidates the moment we go
// to add a feature; suppressing the cosmetic noise is a fair tradeoff.

#![allow(dead_code)]

pub mod app;
pub mod client;
pub mod theme;
pub mod ui;
