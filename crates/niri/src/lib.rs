//! Shared Niri IPC client and identifiers used by KShell surfaces.
//!
//! Keeping these values in a small crate prevents the native surfaces and the
//! generated Niri fragment from drifting apart as more shell components are
//! added.

pub mod connection;
pub mod protocol;
pub mod state;

pub use connection::{focus_workspace, focus_workspace_id, spawn_event_stream};
pub use state::{Workspace, WorkspaceState};

/// Namespace assigned to KShell launcher layer-shell surfaces.
pub const LAUNCHER_NAMESPACE: &str = "my-shell-launcher";

/// Namespace assigned to the KShell top bar layer-shell surface.
pub const BAR_NAMESPACE: &str = "my-shell-bar";

/// Namespace assigned to the KShell volume panel layer-shell surface.
pub const VOLUME_NAMESPACE: &str = "my-shell-volume-popup";

/// Namespace assigned to the KShell volume panel click-catcher surface.
pub const VOLUME_CLICK_CATCHER_NAMESPACE: &str = "my-shell-volume-click-catcher";

/// Command started by the default Niri bar autostart fragment.
pub const BAR_COMMAND: &str = "kbar";

/// Command started by the default Niri launcher binding.
pub const LAUNCHER_COMMAND: &str = "klauncher";

/// Default Niri shortcut for opening the launcher.
pub const LAUNCHER_BINDING: &str = "Mod+Space";
