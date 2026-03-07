# Data Model

## `Shared<T>` — Shared Mutable State

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// Generic wrapper for shared, async-safe mutable state.
pub type Shared<C> = Arc<RwLock<C>>;
```

Use `Shared<T>` whenever multiple async tasks or components need to read/write the same data. Instantiate with:

```rust
let cache = Arc::new(RwLock::new(MyCache::init()));
```

Pass clones to child components as part of their `Init` type.

### Async Access Rules

Always drop read guards before acquiring write guards to avoid deadlocks:

```rust
// Good — drop the read guard before writing.
let value = {
    let cache = shared.read().await;
    cache.get(&key).cloned()
    // guard dropped here
};

if value.is_none() {
    let mut cache = shared.write().await;
    cache.insert(key, compute_value());
}

// Bad — holding the read guard while trying to write will deadlock.
let cache = shared.read().await;
let _ = cache.get(&key);
shared.write().await.insert(key, val); // deadlock!
```

---

## SHA-512 UIDs — Content-Addressable Identity

Files are identified by the SHA-512 hash of their bytes, making UIDs path-independent (renaming a file does not invalidate its cache entry).

```rust
use sha2::{Digest, Sha512};

/// Returns the hex-encoded SHA-512 hash of the given bytes.
#[must_use]
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// Usage — compute UID from file content:
let uid = hash_bytes(&std::fs::read(&path)?);
```

Store UIDs as `String` and use them as `HashMap` keys.

---

## Cache Pattern: `Entry` vs Domain Struct

The cache stores a lightweight `Entry` (serialized to disk). The full domain struct is reconstructed on demand.

```rust
use serde::{Deserialize, Serialize};

/// Serializable cache entry — stored in JSON on disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entry {
    pub title: String,
    pub state: bool,             // false = known-broken file, skip re-parsing
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    // ... other cheap metadata
}

impl Entry {
    /// Mark an entry as broken (failed to parse) so it is skipped on future loads.
    #[must_use]
    pub fn broken(title: String) -> Self {
        Self { title, state: false, ..Default::default() }
    }
}

/// Full domain struct — never serialized, reconstructed from Entry + file.
#[derive(Debug, Clone)]
pub struct DomainItem {
    path: std::path::PathBuf,
    uid: String,
    // ... all fields, including computed ones
}
```

`impl From<DomainItem> for Entry` converts from the full struct to the cache entry when writing.

---

## `MusicCache` — Two-Tier Disk Cache

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Map from UID → metadata entry + map from library path → sorted list of UIEntries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MusicCache {
    /// All known items, keyed by content hash.
    pub items: HashMap<String, Entry>,
    /// Cached library scans, keyed by directory path.
    pub libraries: HashMap<PathBuf, Vec<UIEntry>>,
}

impl MusicCache {
    /// Initialises the cache: creates the file if missing, loads if present, falls back to default.
    #[must_use]
    pub fn init() -> Self {
        let path = cache_path().expect("Could not determine cache path");
        if !path.exists() {
            std::fs::File::create_new(&path).expect("Could not create cache file");
            let cache = Self::default();
            cache.write_to_path(&path).expect("Could not initialize cache");
        }
        Self::load_from_path(&path).unwrap_or_default()
    }

    /// Deserializes the cache from a JSON file.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::deserialize(&mut serde_json::Deserializer::from_str(&content))?)
    }

    /// Serializes the cache to a JSON file (pretty-printed for human readability).
    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
```

Obtain the cache file path via `directories::ProjectDirs`:

```rust
use directories::ProjectDirs;

fn cache_path() -> Option<std::path::PathBuf> {
    let dirs = ProjectDirs::from("org", "example", "myapp")?;
    let dir = dirs.cache_dir().to_owned();
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("items.json"))
}
```

---

## `UIEntry` — Minimal Display Struct

A lightweight struct for the UI that does not require the full domain struct to be loaded. Used as the cached representation in `MusicCache::libraries`.

```rust
/// Minimal entry for UI display — may refer to a file that no longer exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIEntry {
    pub title: String,
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
}
```

---

## `Target` Enum — Progressive Loading

Before files are fully parsed, the UI shows cached `UIEntry` data. After parsing, it upgrades to the full `DomainItem`. The `Target` enum represents either state.

```rust
use derive_more::From;

/// A domain item that may be fully loaded or just a cache placeholder.
#[derive(Debug, Clone, From)]
pub enum Target {
    /// Fast placeholder from the on-disk cache; shown immediately.
    Cache(UIEntry),
    /// Fully loaded item with all metadata.
    Item(DomainItem),
}

impl Target {
    /// Returns `true` if this target has not yet been fully loaded.
    #[must_use]
    pub const fn is_cache(&self) -> bool {
        matches!(self, Self::Cache(_))
    }

    /// Returns the file path regardless of which variant is active.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::Cache(e) => &e.path,
            Self::Item(i) => i.path(),
        }
    }

    /// Returns the display title regardless of which variant is active.
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::Cache(e) => e.title.clone(),
            Self::Item(i) => i.title().to_owned(),
        }
    }

    /// Upgrades a `Cache` variant to a fully loaded `Item`, or returns the existing `Item`.
    pub async fn realize(&self, cache: Shared<MusicCache>) -> anyhow::Result<DomainItem> {
        match self {
            Self::Cache(entry) => DomainItem::new(&entry.path, cache).await,
            Self::Item(item) => Ok(item.clone()),
        }
    }
}
```

---

## Streaming Library Loading

Rather than loading all items before updating the UI, use a `relm4::Sender<Target>` channel so that each item appears as soon as it is parsed:

```rust
/// Loads all items from `path` recursively, sending each one as it is found.
///
/// Uses the cache for directories that have been scanned before.
pub async fn load_library(
    path: &Path,
    tx: relm4::Sender<Target>,
    cache: Shared<MusicCache>,
) -> anyhow::Result<()> {
    // 1. Fast path: return cached entries immediately.
    let cached = {
        let c = cache.read().await;
        c.libraries.get(path).cloned()
        // read guard dropped
    };

    if let Some(entries) = cached {
        for entry in entries {
            tx.send(Target::Cache(entry))
              .map_err(|e| anyhow::anyhow!("Send error: {e:?}"))?;
        }
        return Ok(());
    }

    // 2. Slow path: parse files, send each result, then cache the list.
    let entries = load_dir(path, tx, cache.clone()).await?;

    let mut c = cache.write().await;
    c.libraries.insert(path.to_owned(), entries);
    if let Some(p) = cache_path() {
        c.write_to_path(&p).ok();
    }

    Ok(())
}
```

In the receiving component, create a channel and spawn two tasks — one to produce and one to consume:

```rust
let (tx, mut rx) = relm4::channel();
let cache_clone = self.cache.clone();

// Producer task
relm4::spawn(glib::clone!(#[strong] path, async move {
    if let Err(e) = load_library(&path, tx, cache_clone).await {
        log::error!("Library load error: {e}");
    }
}));

// Consumer task — forwards each Target to the component's own input
relm4::spawn(glib::clone!(#[strong] sender, async move {
    while let Some(target) = rx.recv().await {
        sender.input_sender().emit(Controls::NewTarget(target));
    }
    sender.input_sender().emit(Controls::LoadFinished);
}));
```

### Image Cache

The image cache is a simple in-memory `HashMap` keyed by `(uid, Variant)`:

```rust
pub type ImageCache = HashMap<(String, ImageVariant), relm4::gtk::gdk::Texture>;
```

Access follows the same read-first, drop-guard, write-if-miss pattern as the music cache.
