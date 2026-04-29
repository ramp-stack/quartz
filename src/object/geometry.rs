use crate::types::Anchor;
use super::GameObject;

impl GameObject {
    pub fn check_boundary_collision(&self, canvas_size: (f32, f32)) -> bool {
        if self.rotation == 0.0 {
            return self.position.0 <= 0.0
                || self.position.0 + self.size.0 >= canvas_size.0
                || self.position.1 <= 0.0
                || self.position.1 + self.size.1 >= canvas_size.1;
        }
        // Use rotated AABB for rotating objects so the visual extent triggers events.
        let corners = self.corners_world();
        let min_x = corners.iter().map(|c| c.0).fold(f32::MAX, |a, b| a.min(b));
        let max_x = corners.iter().map(|c| c.0).fold(f32::MIN, |a, b| a.max(b));
        let min_y = corners.iter().map(|c| c.1).fold(f32::MAX, |a, b| a.min(b));
        let max_y = corners.iter().map(|c| c.1).fold(f32::MIN, |a, b| a.max(b));
        min_x <= 0.0 || max_x >= canvas_size.0 || min_y <= 0.0 || max_y >= canvas_size.1
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

    /// World-space position of the rotation pivot.
    /// With the default pivot (0.5, 0.5) this equals the geometric centre.
    pub fn pivot_world(&self) -> (f32, f32) {
        (
            self.position.0 + self.size.0 * self.pivot.0,
            self.position.1 + self.size.1 * self.pivot.1,
        )
    }

    /// Transforms a point in local pivot-relative space into world space.
    pub fn local_to_world(&self, local: (f32, f32)) -> (f32, f32) {
        let (pw_x, pw_y) = self.pivot_world();
        if self.rotation == 0.0 {
            return (pw_x + local.0, pw_y + local.1);
        }
        let theta = self.rotation.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        (
            pw_x + local.0 * cos_t - local.1 * sin_t,
            pw_y + local.0 * sin_t + local.1 * cos_t,
        )
    }

    /// World-space geometric centre after rotation.
    /// With pivot (0.5, 0.5) this equals position + size * 0.5 (current behaviour).
    pub fn rotated_center(&self) -> (f32, f32) {
        self.local_to_world((
            self.size.0 * (0.5 - self.pivot.0),
            self.size.1 * (0.5 - self.pivot.1),
        ))
    }

    /// All four world-space corner positions of the rotated rectangle.
    /// Order: top-left, top-right, bottom-left, bottom-right (in local frame).
    pub fn corners_world(&self) -> [(f32, f32); 4] {
        let (px, py) = self.pivot;
        let local = [
            (-self.size.0 * px,          -self.size.1 * py),
            ( self.size.0 * (1.0 - px),  -self.size.1 * py),
            (-self.size.0 * px,           self.size.1 * (1.0 - py)),
            ( self.size.0 * (1.0 - px),   self.size.1 * (1.0 - py)),
        ];
        std::array::from_fn(|i| self.local_to_world(local[i]))
    }
}