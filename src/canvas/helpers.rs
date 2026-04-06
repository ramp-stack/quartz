use super::core::Canvas;
use prism::canvas::Image;

// ── Tag-based object queries ─────────────────────────────────────────────

impl Canvas {
    /// Return names of all objects with a given tag.
    pub fn get_names_by_tag(&self, tag: &str) -> Vec<String> {
        self.store.tag_to_indices.get(tag)
            .map(|indices| {
                indices.iter().filter_map(|&i| self.store.names.get(i).cloned()).collect()
            })
            .unwrap_or_default()
    }

    /// Count objects with a tag.
    pub fn count_by_tag(&self, tag: &str) -> usize {
        self.store.tag_to_indices.get(tag).map_or(0, |v| v.len())
    }

    /// Check if any object with this tag exists.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.store.tag_to_indices.get(tag).map_or(false, |v| !v.is_empty())
    }
}

// ── Object pool system ───────────────────────────────────────────────────

impl Canvas {
    /// Pre-spawn `count` hidden copies of a template object, tagged with `pool_tag`.
    pub fn create_pool(&mut self, pool_tag: &str, template: crate::GameObject, count: usize) {
        for i in 0..count {
            let name = format!("_pool_{}_{}", pool_tag, i);
            let mut obj = template.clone();
            obj.visible = false;
            obj.tags.push(format!("_pool:{}", pool_tag));
            obj.tags.push("_pool_free".to_string());
            self.store.add(name, obj);
            self.layout.offsets.push((-9999.0, -9999.0));
        }
    }

    /// Acquire one object from pool: shows it, resets position/momentum, returns its name.
    pub fn pool_acquire(&mut self, pool_tag: &str, position: (f32, f32)) -> Option<String> {
        let pool_tag_key = format!("_pool:{}", pool_tag);
        let pool_indices = self.store.tag_to_indices.get(&pool_tag_key)?.clone();
        let free_indices = self.store.tag_to_indices.get("_pool_free")
            .cloned().unwrap_or_default();

        for &idx in &pool_indices {
            if free_indices.contains(&idx) {
                if let Some(obj) = self.store.objects.get_mut(idx) {
                    obj.visible = true;
                    obj.position = position;
                    obj.momentum = (0.0, 0.0);
                    obj.tags.retain(|t| t != "_pool_free");
                }
                if let Some(v) = self.store.tag_to_indices.get_mut("_pool_free") {
                    v.retain(|&i| i != idx);
                }
                if let Some(offset) = self.layout.offsets.get_mut(idx) {
                    *offset = position;
                }
                return self.store.names.get(idx).cloned();
            }
        }
        None
    }

    /// Release an object back to pool: hides it, zeros momentum.
    pub fn pool_release(&mut self, name: &str) {
        if let Some(&idx) = self.store.name_to_index.get(name) {
            if let Some(obj) = self.store.objects.get_mut(idx) {
                obj.visible = false;
                obj.momentum = (0.0, 0.0);
                if !obj.tags.contains(&"_pool_free".to_string()) {
                    obj.tags.push("_pool_free".to_string());
                    self.store.tag_to_indices.entry("_pool_free".into()).or_default().push(idx);
                }
            }
            if let Some(offset) = self.layout.offsets.get_mut(idx) {
                *offset = (-9999.0, -9999.0);
            }
        }
    }

    /// Release all active objects in a pool.
    pub fn pool_release_all(&mut self, pool_tag: &str) {
        let key = format!("_pool:{}", pool_tag);
        let names: Vec<String> = self.get_names_by_tag(&key);
        for name in names {
            self.pool_release(&name);
        }
    }

    /// How many pool objects are available (free).
    pub fn pool_available(&self, pool_tag: &str) -> usize {
        let key = format!("_pool:{}", pool_tag);
        let pool = self.store.tag_to_indices.get(&key).cloned().unwrap_or_default();
        let free = self.store.tag_to_indices.get("_pool_free").cloned().unwrap_or_default();
        pool.iter().filter(|i| free.contains(i)).count()
    }

    /// How many pool objects are currently active (in use).
    pub fn pool_active(&self, pool_tag: &str) -> usize {
        let key = format!("_pool:{}", pool_tag);
        let pool = self.store.tag_to_indices.get(&key).cloned().unwrap_or_default();
        let free = self.store.tag_to_indices.get("_pool_free").cloned().unwrap_or_default();
        pool.iter().filter(|i| !free.contains(i)).count()
    }
}

// ── Image cache ──────────────────────────────────────────────────────────

impl Canvas {
    /// Load an image, caching by path. Subsequent calls return a clone.
    pub fn load_image_cached(&mut self, path: &str) -> Image {
        if let Some(img) = self.image_cache.get(path) {
            return img.clone();
        }
        let img = crate::sprite::load_image(path);
        self.image_cache.insert(path.to_string(), img.clone());
        img
    }

    /// Load a sized image, caching by path+dimensions.
    pub fn load_image_sized_cached(&mut self, path: &str, w: f32, h: f32) -> Image {
        let key = format!("{}:{}x{}", path, w, h);
        if let Some(img) = self.image_cache.get(&key) {
            return img.clone();
        }
        let img = crate::sprite::load_image_sized(path, w, h);
        self.image_cache.insert(key, img.clone());
        img
    }

    /// Clear the image cache.
    pub fn clear_image_cache(&mut self) {
        self.image_cache.clear();
    }
}
