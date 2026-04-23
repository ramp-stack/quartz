use std::collections::HashMap;
use prism::canvas::Image;

/// A general-purpose image cache keyed by string identifiers.
///
/// `ImageCache` stores [`Image`] values indexed by arbitrary string keys.
/// Games define their own key conventions (e.g. `"circle_48_255_95_210"`).
///
/// The primary entry point is [`get_or_create`](ImageCache::get_or_create),
/// which returns a cached `Image` if one exists for the key, or calls the
/// provided closure to generate it on a cache miss. Because `Image` uses
/// `Arc<RgbaImage>` internally, clones are cheap (reference-count bump).
///
/// # Example
/// ```ignore
/// let img = cache.get_or_create("player_circle", || {
///     quartz::solid_circle(80.0, Color(80, 220, 160, 255))
/// });
/// ```
///
/// An `ImageCache` is embedded inside [`Canvas`](crate::canvas::Canvas)
/// and exposed through convenience methods. You can also create standalone
/// instances for caching outside of the canvas lifecycle.
#[derive(Clone, Debug, Default)]
pub struct ImageCache {
    entries: HashMap<String, Image>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Return a cached image, or generate and cache it via the closure.
    pub fn get_or_create(&mut self, key: impl Into<String>, f: impl FnOnce() -> Image) -> Image {
        let key = key.into();
        if let Some(img) = self.entries.get(&key) {
            return img.clone();
        }
        let img = f();
        self.entries.insert(key, img.clone());
        img
    }

    /// Retrieve a previously cached image.
    pub fn get(&self, key: &str) -> Option<&Image> {
        self.entries.get(key)
    }

    /// Insert an image into the cache, replacing any existing entry.
    pub fn insert(&mut self, key: impl Into<String>, image: Image) {
        self.entries.insert(key.into(), image);
    }

    /// Remove a cached image by key.
    pub fn remove(&mut self, key: &str) -> Option<Image> {
        self.entries.remove(key)
    }

    /// Returns `true` if the cache contains an entry for the given key.
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove all cached images.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of cached images.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retain only entries for which the predicate returns `true`.
    pub fn retain(&mut self, mut f: impl FnMut(&str, &mut Image) -> bool) {
        self.entries.retain(|k, v| f(k.as_str(), v));
    }
}
