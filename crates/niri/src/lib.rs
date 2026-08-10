//! Shared identifiers used by KShell surfaces and Niri configuration.
//!
//! Keeping these values in a small crate prevents the native surfaces and the
//! generated Niri fragment from drifting apart as more shell components are
//! added.

/// Namespace assigned to KShell launcher layer-shell surfaces.
pub const LAUNCHER_NAMESPACE: &str = "my-shell-launcher";

/// Namespace assigned to the KShell top bar layer-shell surface.
pub const BAR_NAMESPACE: &str = "my-shell-bar";

/// Command started by the default Niri bar autostart fragment.
pub const BAR_COMMAND: &str = "kbar";

/// Command started by the default Niri launcher binding.
pub const LAUNCHER_COMMAND: &str = "klauncher";

/// Default Niri shortcut for opening the launcher.
pub const LAUNCHER_BINDING: &str = "Mod+Space";
