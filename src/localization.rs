//! Localization setup using i18n-embed and Fluent.

use anyhow::Result;
use i18n_embed::{
    DesktopLanguageRequester,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;

/// Embedded localization assets.
#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

/// Global Fluent language loader.
pub static LANGUAGE_LOADER: std::sync::LazyLock<FluentLanguageLoader> = std::sync::LazyLock::new(|| {
    let loader = fluent_language_loader!();
    let requested = DesktopLanguageRequester::requested_languages();
    i18n_embed::select(&loader, &Localizations, &requested).expect("Failed to load localizations");
    loader
});

/// Initializes the localization system.
///
/// Must be called before any `fl!()` macro invocation.
///
/// # Errors
/// Returns an error if the localization assets cannot be loaded.
pub fn setup() -> Result<()> {
    std::sync::LazyLock::force(&LANGUAGE_LOADER);
    Ok(())
}

/// Macro to look up a localized string by message ID.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localization::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localization::LANGUAGE_LOADER, $message_id, $($args),*)
    }};
}
