//! Fallback conflicting-host facts for platforms with no backend yet: report nothing.
//! An empty vec means "could not tell", never "nothing is running" (see
//! [`super::running_process_names`]).

use super::{Evidence, Known};

pub fn running_processes() -> Vec<String> {
    Vec::new()
}

pub fn static_evidence(_known: &Known) -> Vec<Evidence> {
    Vec::new()
}
