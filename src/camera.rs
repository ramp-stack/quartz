use crate::Target;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: (f32, f32),
    pub world_size: (f32, f32),
    pub(crate) viewport_size: (f32, f32),
    pub(crate) follow_target: Option<Target>,
    pub lerp_speed: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn new(world_size: (f32, f32), viewport_size: (f32, f32)) -> Self {
        Self {
            position: (0.0, 0.0),
            world_size,
            viewport_size,
            follow_target: None,
            lerp_speed: 0.10,
            zoom: 1.0,
        }
    }

    pub fn follow(&mut self, target: Option<Target>) {
        self.follow_target = target;
    }

    pub fn center_on(&mut self, wx: f32, wy: f32) {
        let visible_w = self.viewport_size.0 / self.zoom;
        let visible_h = self.viewport_size.1 / self.zoom;
        self.position.0 = (wx - visible_w * 0.5)
            .clamp(0.0, (self.world_size.0 - visible_w).max(0.0));
        self.position.1 = (wy - visible_h * 0.5)
            .clamp(0.0, (self.world_size.1 - visible_h).max(0.0));
    }

    pub(crate) fn lerp_toward(&mut self, wx: f32, wy: f32) {
        let visible_w = self.viewport_size.0 / self.zoom;
        let visible_h = self.viewport_size.1 / self.zoom;
        let tx = (wx - visible_w * 0.5)
            .clamp(0.0, (self.world_size.0 - visible_w).max(0.0));
        let ty = (wy - visible_h * 0.5)
            .clamp(0.0, (self.world_size.1 - visible_h).max(0.0));
        self.position.0 += (tx - self.position.0) * self.lerp_speed;
        self.position.1 += (ty - self.position.1) * self.lerp_speed;
    }
}