use super::core::Canvas;
use prism::canvas::Image;

impl Canvas {
    pub fn get_names_by_tag(&self, tag: &str) -> Vec<String> {
        self.store.tag_to_indices.get(tag)
            .map(|indices| {
                indices.iter().filter_map(|&i| self.store.names.get(i).cloned()).collect()
            })
            .unwrap_or_default()
    }

    pub fn count_by_tag(&self, tag: &str) -> usize {
        self.store.tag_to_indices.get(tag).map_or(0, |v| v.len())
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.store.tag_to_indices.get(tag).map_or(false, |v| !v.is_empty())
    }
}

impl Canvas {
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

    pub fn pool_release_all(&mut self, pool_tag: &str) {
        let key = format!("_pool:{}", pool_tag);
        let names: Vec<String> = self.get_names_by_tag(&key);
        for name in names {
            self.pool_release(&name);
        }
    }

    pub fn pool_available(&self, pool_tag: &str) -> usize {
        let key = format!("_pool:{}", pool_tag);
        let pool = self.store.tag_to_indices.get(&key).cloned().unwrap_or_default();
        let free = self.store.tag_to_indices.get("_pool_free").cloned().unwrap_or_default();
        pool.iter().filter(|i| free.contains(i)).count()
    }

    pub fn pool_active(&self, pool_tag: &str) -> usize {
        let key = format!("_pool:{}", pool_tag);
        let pool = self.store.tag_to_indices.get(&key).cloned().unwrap_or_default();
        let free = self.store.tag_to_indices.get("_pool_free").cloned().unwrap_or_default();
        pool.iter().filter(|i| !free.contains(i)).count()
    }
}

impl Canvas {
    pub fn load_image_cached(&mut self, key: &str, bytes: &[u8]) -> Image {
        self.image_cache.get_or_create(key, || crate::sprite::load_image(bytes))
    }

    pub fn load_image_sized_cached(&mut self, key: &str, bytes: &[u8], w: f32, h: f32) -> Image {
        let cache_key = format!("{}:{}x{}", key, w, h);
        self.image_cache.get_or_create(cache_key, || crate::sprite::load_image_sized(bytes, w, h))
    }

    pub fn get_or_create_image(&mut self, key: impl Into<String>, f: impl FnOnce() -> Image) -> Image {
        self.image_cache.get_or_create(key, f)
    }

    pub fn clear_image_cache(&mut self) {
        self.image_cache.clear();
    }
}

pub fn orbit_speed(gravity_strength: f32, planet_radius: f32, orbit_dist: f32) -> f32 {
    if orbit_dist <= 0.0 { return 0.0; }
    (gravity_strength * planet_radius / orbit_dist).sqrt()
}

pub fn escape_speed(gravity_strength: f32, planet_radius: f32, dist: f32) -> f32 {
    orbit_speed(gravity_strength, planet_radius, dist) * std::f32::consts::SQRT_2
}