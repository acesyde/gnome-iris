//! Application configuration types — serialized as JSON to `$XDG_DATA_HOME/iris/`.

use serde::{Deserialize, Serialize};

/// Global application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::module_name_repetitions)]
pub struct GlobalConfig {
    /// Ordered list of shader repositories.
    pub shader_repos: Vec<ShaderRepo>,
    /// When true, shaders from all repos are merged into a single `Merged/` directory.
    pub merge_shaders: bool,
    /// How many hours between automatic update checks.
    pub update_interval_hours: u64,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            shader_repos: default_shader_repos(),
            merge_shaders: true,
            update_interval_hours: 4,
        }
    }
}

/// A remote Git repository containing `ReShade` shaders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderRepo {
    /// Remote URL (HTTPS).
    pub url: String,
    /// Local directory name under `ReShade_shaders/`.
    pub local_name: String,
    /// Optional branch name; clones the default branch when `None`.
    pub branch: Option<String>,
    /// Whether this repo is enabled for new games by default.
    pub enabled_by_default: bool,
}

/// Per-game shader repository overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShaderOverrides {
    /// `local_name`s of repos disabled for this game.
    pub disabled_repos: Vec<String>,
}

fn default_shader_repos() -> Vec<ShaderRepo> {
    vec![
        ShaderRepo {
            url: "https://github.com/crosire/reshade-shaders".into(),
            local_name: "reshade-shaders".into(),
            branch: Some("slim".into()),
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/martymcmodding/qUINT".into(),
            local_name: "martymc-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/CeeJayDK/SweetFX".into(),
            local_name: "sweetfx-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/BlueSkyDefender/AstrayFX".into(),
            local_name: "astrayfx-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
        ShaderRepo {
            url: "https://github.com/prod80/prod80-ReShade-Repository".into(),
            local_name: "prod80-shaders".into(),
            branch: None,
            enabled_by_default: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> GlobalConfig {
        GlobalConfig {
            shader_repos: vec![ShaderRepo {
                url: "https://github.com/crosire/reshade-shaders".into(),
                local_name: "reshade-shaders".into(),
                branch: Some("slim".into()),
                enabled_by_default: true,
            }],
            merge_shaders: true,
            update_interval_hours: 4,
        }
    }

    #[test]
    fn roundtrip_global_config() {
        let config = sample_config();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: GlobalConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.shader_repos.len(), 1);
        assert_eq!(decoded.shader_repos[0].local_name, "reshade-shaders");
        assert_eq!(decoded.update_interval_hours, 4);
        assert!(decoded.merge_shaders);
    }

    #[test]
    fn default_shader_repos_are_not_empty() {
        let config = GlobalConfig::default();
        assert!(!config.shader_repos.is_empty());
    }

    #[test]
    fn roundtrip_shader_overrides() {
        let overrides = ShaderOverrides {
            disabled_repos: vec!["sweetfx-shaders".into()],
        };
        let json = serde_json::to_string(&overrides).unwrap();
        let decoded: ShaderOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.disabled_repos, vec!["sweetfx-shaders"]);
    }
}
