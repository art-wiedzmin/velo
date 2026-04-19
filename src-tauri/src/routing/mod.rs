//! Routing-related subsystems split into two concerns:
//!
//! - [`processes`]: enumerate running user-space processes for the per-app
//!   routing picker (dedup by exe path, filter `%SystemRoot%`).
//! - [`elevation`]: Windows admin-privilege detection and UAC relaunch, needed
//!   before sing-box's TUN inbound can bring up a virtual NIC.

mod elevation;
mod processes;

pub use elevation::{is_elevated, relaunch_as_admin};
pub use processes::{processes_snapshot, ProcessInfo};
