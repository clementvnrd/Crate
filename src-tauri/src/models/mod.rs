pub mod album;
pub mod backup;
pub mod cue;
// Models backing desktop-only services (USB device, diagnostics, Pioneer export) are gated
// so they don't get compiled into the mobile binary.
#[cfg(feature = "desktop")]
pub mod device;
#[cfg(feature = "desktop")]
pub mod diagnostics;
pub mod discovery;
pub mod duplicate;
#[cfg(feature = "desktop")]
pub mod export;
pub mod follow;
pub mod playlist;
pub mod settings;
pub mod smart_rules;
pub mod standalone;
pub mod stats;
pub mod tag;
pub mod track;
pub mod upgrader;

#[allow(unused_imports)]
pub use album::*;
pub use backup::*;
pub use cue::*;
#[cfg(feature = "desktop")]
pub use device::*;
#[cfg(feature = "desktop")]
pub use diagnostics::*;
pub use discovery::*;
pub use duplicate::*;
#[cfg(feature = "desktop")]
pub use export::*;
pub use follow::*;
pub use playlist::*;
pub use settings::*;
pub use smart_rules::*;
pub use standalone::*;
pub use stats::*;
pub use tag::*;
pub use track::*;
pub use upgrader::*;

