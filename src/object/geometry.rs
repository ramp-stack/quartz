use crate::types::Anchor;
use super::GameObject;

impl GameObject {
    pub fn check_boundary_collision(&self, canvas_size: (f32, f32)) -> bool {
        self.position.0 <= 0.0
            || self.position.0 + self.size.0 >= canvas_size.0
            || self.position.1 <= 0.0
            || self.position.1 + self.size.1 >= canvas_size.1
    }

    pub fn get_anchor_position(&self, anchor: Anchor) -> (f32, f32) {
        (
            self.position.0 + self.size.0 * anchor.x,
            self.position.1 + self.size.1 * anchor.y,
        )
    }

    pub fn contains_point(&self, point: (f32, f32)) -> bool {
        point.0 >= self.position.0
            && point.0 <= self.position.0 + self.size.0
            && point.1 >= self.position.1
            && point.1 <= self.position.1 + self.size.1
    }

    pub fn apply_rotation_momentum(&mut self) {
        if self.rotation_momentum == 0.0 { return; }
        self.rotation += self.rotation_momentum;
        self.rotation_momentum *= self.rotation_resistance;
        if self.rotation_momentum.abs() < 0.01 { self.rotation_momentum = 0.0; }
        if self.is_platform { self.sync_rotation_normal(); }
    }

    pub fn sync_rotation_normal(&mut self) {
        let theta = self.rotation.to_radians();
        self.surface_normal = (theta.sin(), -theta.cos());
    }

    pub fn slope_surface_y(&self, world_x: f32) -> f32 {
        match self.slope {
            None => self.position.1,
            Some((left_offset, right_offset)) => {
                if self.size.0 == 0.0 { return self.position.1; }
                let t = ((world_x - self.position.0) / self.size.0).clamp(0.0, 1.0);
                self.position.1 + left_offset + (right_offset - left_offset) * t
            }
        }
    }

    pub fn rotation_from_slope(&self) -> f32 {
        match self.slope {
            None => 0.0,
            Some((left_offset, right_offset)) => {
                (right_offset - left_offset).atan2(self.size.0).to_degrees()
            }
        }
    }

    pub fn surface_normal_at(&self, _world_x: f32) -> (f32, f32) {
        match self.slope {
            None => self.surface_normal,
            Some((left_offset, right_offset)) => {
                let w = self.size.0;
                if w < 0.01 { return (0.0, -1.0); }
                let rise = right_offset - left_offset;
                let len  = (rise * rise + w * w).sqrt();
                (rise / len, -w / len)
            }
        }
    }

    pub fn slope_aabb(&self) -> (f32, f32, f32, f32) {
        match self.slope {
            None => (self.position.0, self.position.1, self.size.0, self.size.1),
            Some((left_off, right_off)) => {
                let left_y  = self.position.1 + left_off;
                let right_y = self.position.1 + right_off;
                let top     = left_y.min(right_y);
                let bottom  = left_y.max(right_y) + self.size.1;
                (self.position.0, top, self.size.0, bottom - top)
            }
        }
    }

    pub fn clip(mut self) -> Self {
        self.clipped = true;
        self
    }

    pub fn set_clip(&mut self, clip: bool) {
        self.clipped = clip;
    }

    pub fn set_clip_origin(&mut self, origin: Option<(f32, f32)>) {
        self.clip_origin = origin;
    }
}