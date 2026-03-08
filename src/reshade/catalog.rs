//! Curated list of known ReShade shader repositories.

use crate::reshade::config::ShaderRepo;

/// A known shader repository from the community catalog.
#[derive(Debug)]
pub struct CatalogEntry {
    /// Display name shown in the Shaders tab.
    pub name: &'static str,
    /// Short description of the shaders included.
    pub description: &'static str,
    /// Local directory name under `ReShade_shaders/`.
    pub local_name: &'static str,
    /// Remote HTTPS URL.
    pub url: &'static str,
    /// Optional branch; `None` clones the default branch.
    pub branch: Option<&'static str>,
}

impl CatalogEntry {
    /// Converts this catalog entry into a [`ShaderRepo`] suitable for syncing.
    pub fn to_shader_repo(&self) -> ShaderRepo {
        ShaderRepo {
            url: self.url.to_owned(),
            local_name: self.local_name.to_owned(),
            branch: self.branch.map(str::to_owned),
            enabled_by_default: false,
        }
    }
}

/// All known shader repositories, in display order.
pub static KNOWN_REPOS: &[CatalogEntry] = &[
    CatalogEntry {
        name: "crosire / reshade-shaders",
        description: "Official ReShade shader collection (slim branch)",
        local_name: "reshade-shaders",
        url: "https://github.com/crosire/reshade-shaders",
        branch: Some("slim"),
    },
    CatalogEntry {
        name: "Marty McFly / qUINT",
        description: "MXAO, ADOF, Screen Space Reflection, Bloom, Sharpen, Lightroom",
        local_name: "martymc-shaders",
        url: "https://github.com/martymcmodding/qUINT",
        branch: None,
    },
    CatalogEntry {
        name: "CeeJayDK / SweetFX",
        description: "Classic post-processing suite",
        local_name: "sweetfx-shaders",
        url: "https://github.com/CeeJayDK/SweetFX",
        branch: None,
    },
    CatalogEntry {
        name: "BlueSkyDefender / AstrayFX",
        description: "AstrayFX effect collection",
        local_name: "astrayfx-shaders",
        url: "https://github.com/BlueSkyDefender/AstrayFX",
        branch: None,
    },
    CatalogEntry {
        name: "prod80 / prod80-ReShade-Repository",
        description: "DOF, Flares, Bloom, Tonemapping, Sharpening",
        local_name: "prod80-shaders",
        url: "https://github.com/prod80/prod80-ReShade-Repository",
        branch: None,
    },
    CatalogEntry {
        name: "FransBouma / OtisFX",
        description: "Cinematic DOF, Emphasize, Adaptive Fog",
        local_name: "otisfx-shaders",
        url: "https://github.com/FransBouma/OtisFX",
        branch: None,
    },
    CatalogEntry {
        name: "Fubaxiusz / fubax-shaders",
        description: "Perfect Perspective, Chromakey, Filmic Anamorphic Sharpen",
        local_name: "fubax-shaders",
        url: "https://github.com/Fubaxiusz/fubax-shaders",
        branch: None,
    },
    CatalogEntry {
        name: "Daodan317081 / reshade-shaders",
        description: "Color Isolation, AspectRatio, Hotsampling Helper, Comic",
        local_name: "daodan-shaders",
        url: "https://github.com/Daodan317081/reshade-shaders",
        branch: None,
    },
    CatalogEntry {
        name: "luloco250 / FXShaders",
        description: "Arcane Bloom, Magic Bloom, Mat Cap, Hex Lens Flare",
        local_name: "fxshaders",
        url: "https://github.com/luloco250/FXShaders",
        branch: None,
    },
    CatalogEntry {
        name: "brussell1 / Shaders",
        description: "Fake DOF, Eye Adaptation",
        local_name: "brussell-shaders",
        url: "https://github.com/brussell1/Shaders",
        branch: None,
    },
    CatalogEntry {
        name: "guestrr / ReshadeShaders",
        description: "Bumpmapping, Deblur, Fast Sharpen",
        local_name: "guestrr-shaders",
        url: "https://github.com/guestrr/ReshadeShaders",
        branch: None,
    },
    CatalogEntry {
        name: "Zackin5 / Filmic-Tonemapping-ReShade",
        description: "Filmic tonemap shaders (Hejl ALU, Reinhard, Uncharted)",
        local_name: "filmic-tonemapping-shaders",
        url: "https://github.com/Zackin5/Filmic-Tonemapping-ReShade",
        branch: None,
    },
    CatalogEntry {
        name: "Matsilagi / reshade-retroarch-shaders",
        description: "Ported Retroarch shaders (mostly CRT)",
        local_name: "retroarch-shaders",
        url: "https://github.com/Matsilagi/reshade-retroarch-shaders",
        branch: None,
    },
    CatalogEntry {
        name: "Matsilagi / reshade-shadertoy-shaders",
        description: "Ported Shadertoy shaders",
        local_name: "shadertoy-shaders",
        url: "https://github.com/Matsilagi/reshade-shadertoy-shaders",
        branch: None,
    },
    CatalogEntry {
        name: "Matsilagi / reshade-unity-shaders",
        description: "Ported Unity Asset shaders (VHS, Ditherpack, RetroTV)",
        local_name: "unity-shaders",
        url: "https://github.com/Matsilagi/reshade-unity-shaders",
        branch: None,
    },
    CatalogEntry {
        name: "dddfault / NativeEnhancer-FE-DX10",
        description: "Film Simulation LUT Pack (DX10/DX11)",
        local_name: "nativeenhancer-shaders",
        url: "https://github.com/dddfault/NativeEnhancer-FE-DX10",
        branch: None,
    },
];
