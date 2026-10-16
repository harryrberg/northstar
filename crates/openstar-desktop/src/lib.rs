// SPDX-License-Identifier: GPL-3.0-only
//! Experimental desktop adapter. Editing semantics remain in openstar-core.
pub const APP_TITLE: &str = concat!("OpenStar 4.0 (", env!("CARGO_PKG_VERSION"), ")");

pub mod directory;
pub mod input;
pub mod projection;
pub mod recovery;
pub mod screen;
pub mod session;
