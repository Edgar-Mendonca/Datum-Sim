//! Datum Sim engine: the model and its edits, the block library, the compiler, the solvers,
//! results, routing and sweeps. Pure Rust; the window only shows what this computes.

// Numerical code reads best with index loops and explicit NaN-rejecting comparisons.
#![allow(clippy::needless_range_loop, clippy::too_many_arguments, clippy::type_complexity, clippy::neg_cmp_op_on_partial_ord)]

pub mod blocks;
pub mod compile;
pub mod doc;
pub mod examples;
pub mod expr;
mod hardening;
pub mod linalg;
pub mod model;
pub mod route;
pub mod rt;
pub mod solve;
pub mod sweep;
mod verify;
pub mod view;
