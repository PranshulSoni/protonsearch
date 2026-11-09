//! Safe native action execution. OWNER: sources-content.
//!
//! Every action arrives as a structured ps_ipc::ActionSpec. Destructive
//! actions must require an explicit confirmation flag from the caller; the
//! engine enforces the permission policy, this module only executes.

use ps_ipc::{ActionSpec, NativeCommand, OpenOutcome};

/// Execute a user-invoked action. Approval handling happens in ps-core / UI.
pub fn execute(_action: &ActionSpec) -> OpenOutcome {
    OpenOutcome { ok: false, message: "not implemented yet".into() }
}

/// Run a reviewed native command.
pub fn run_native(_command: NativeCommand) -> OpenOutcome {
    OpenOutcome { ok: false, message: "not implemented yet".into() }
}
