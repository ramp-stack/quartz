use crate::Target;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: (f32, f32),
    pub world_size: (f32, f32),
    pub(crate) viewport_size: (f32, f32),
    pub(crate) follow_target: Option<Target>,
    pub lerp_speed: f32,
}

impl Camera {
    pub fn new(world_size: (f32, f32), viewport_size: (f32, f32)) -> Self {
        Self {
            position: (0.0, 0.0),
            world_size,
            viewport_size,
            follow_target: None,
            lerp_speed: 0.10,
        }
    }

    pub fn follow(&mut self, target: Option<Target>) {
        self.follow_target = target;
    }

    pub fn center_on(&mut self, wx: f32, wy: f32) {
        self.position.0 = (wx - self.viewport_size.0 * 0.5)
            .clamp(0.0, (self.world_size.0 - self.viewport_size.0).max(0.0));
        self.position.1 = (wy - self.viewport_size.1 * 0.5)
            .clamp(0.0, (self.world_size.1 - self.viewport_size.1).max(0.0));
    }

    pub(crate) fn lerp_toward(&mut self, wx: f32, wy: f32) {
        let tx = (wx - self.viewport_size.0 * 0.5)
            .clamp(0.0, (self.world_size.0 - self.viewport_size.0).max(0.0));
        let ty = (wy - self.viewport_size.1 * 0.5)
            .clamp(0.0, (self.world_size.1 - self.viewport_size.1).max(0.0));
        self.position.0 += (tx - self.position.0) * self.lerp_speed;
        self.position.1 += (ty - self.position.1) * self.lerp_speed;
    }
}