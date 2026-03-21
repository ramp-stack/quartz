#[derive(Debug, Clone)]
pub struct Entropy {
    seed: u64,
}
 
impl Entropy {
    pub fn new() -> Self {
        Self::from_time()
    }
 
    pub fn from_seed(seed: u64) -> Self {
        Self { seed }
    }
 
    pub fn from_time() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ (d.as_secs().wrapping_mul(6364136223846793005)))
            .unwrap_or(12345678901234567);
        Self { seed }
    }
 
    fn tick(&mut self) -> f32 {
        self.seed = self.seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.seed >> 33) as f32) / (u32::MAX as f32)
    }
 
    pub fn seed(&mut self, seed: u64) {
        self.seed = seed;
    }
 
    pub fn reseed(&mut self) {
        *self = Self::from_time();
    }
 
    pub fn next(&mut self) -> f32 {
        self.tick()
    }
 
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.tick() * (max - min)
    }
 
    pub fn int(&mut self, min: i32, max: i32) -> i32 {
        let range = (max - min + 1).max(1) as f32;
        min + (self.tick() * range).floor() as i32
    }
 
    pub fn chance(&mut self, probability: f32) -> bool {
        self.tick() < probability
    }
 
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() { return None; }
        let idx = self.int(0, items.len() as i32 - 1) as usize;
        Some(&items[idx])
    }
 
    pub fn position_in(&mut self, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
        (self.range(x, x + w), self.range(y, y + h))
    }
}
 
impl Default for Entropy {
    fn default() -> Self {
        Self::new()
    }
}
 