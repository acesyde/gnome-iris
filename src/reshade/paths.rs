//! Canonical path constants for the iris data directory layout.
//!
//! Every path segment used by the domain layer is defined here.
//! No module should contain a raw path string literal that is also
//! used somewhere else — import the constant instead.

/// Directory that holds all cloned shader repositories and the merged tree.
///
/// Located at `$XDG_DATA_HOME/iris/ReShade_shaders/`.
pub const RESHADE_SHADERS_DIR: &str = "ReShade_shaders";

/// Sub-directory inside [`RESHADE_SHADERS_DIR`] that contains the merged symlink tree.
///
/// Located at `$XDG_DATA_HOME/iris/ReShade_shaders/Merged/`.
pub const MERGED_DIR: &str = "Merged";

/// File name of the JSON cache that records the latest/installed `ReShade` versions.
///
/// Located at `$XDG_DATA_HOME/iris/reshade_state.json`.
pub const RESHADE_STATE_FILE: &str = "reshade_state.json";

/// File name of the JSON file that stores `GlobalConfig`.
///
/// Located at `$XDG_DATA_HOME/iris/config.json`.
pub const CONFIG_FILE: &str = "config.json";

/// File name of the JSON file that stores the saved game list.
///
/// Located at `$XDG_DATA_HOME/iris/games.json`.
pub const GAMES_FILE: &str = "games.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshade_shaders_dir_value() {
        assert_eq!(RESHADE_SHADERS_DIR, "ReShade_shaders");
    }

    #[test]
    fn merged_dir_value() {
        assert_eq!(MERGED_DIR, "Merged");
    }

    #[test]
    fn reshade_state_file_value() {
        assert_eq!(RESHADE_STATE_FILE, "reshade_state.json");
    }

    #[test]
    fn config_file_value() {
        assert_eq!(CONFIG_FILE, "config.json");
    }

    #[test]
    fn games_file_value() {
        assert_eq!(GAMES_FILE, "games.json");
    }
}
