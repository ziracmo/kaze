//! Kaze's engine: finds what takes space on disk and frees it. No UI here, so
//! everything is testable headless.

pub mod discover;
pub mod fmt;
pub mod process;
pub mod scan;
pub mod size;
pub mod worktree;

pub use scan::{Options, Reason, Report, scan};
