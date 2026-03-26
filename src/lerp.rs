#[derive(Debug, Clone, Copy)]
pub struct Lerp {
    pub value:  f32,
    pub target: f32,
    speed:      f32,
    min:        f32,
    max:        f32,
}

impl Lerp {
    pub fn new(speed: f32) -> Self {
        Self { value: 0.0, target: 0.0, speed, min: f32::NEG_INFINITY, max: f32::INFINITY }
    }

    pub fn bounded(speed: f32, min: f32, max: f32) -> Self {
        Self { value: 0.0, target: 0.0, speed, min, max }
    }

    pub fn tick(&mut self) -> bool {
        if (self.value - self.target).abs() < 0.5 {
            self.value = self.target;
            return false;
        }
        self.value += (self.target - self.value) * self.speed;
        true
    }

    pub fn set(&mut self, target: f32) {
        self.target = target.clamp(self.min, self.max);
    }

    pub fn nudge(&mut self, delta: f32) {
        self.set(self.target + delta);
    }

    pub fn set_bounds(&mut self, min: f32, max: f32) {
        self.min    = min;
        self.max    = max;
        self.target = self.target.clamp(min, max);
    }

    pub fn snap(&mut self) {
        self.value = self.target;
    }

    pub fn snap_to(&mut self, v: f32) {
        let v       = v.clamp(self.min, self.max);
        self.value  = v;
        self.target = v;
    }
}