/// Controls how gravitational force falls off with distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GravityFalloff {
    /// Default. pull ∝ 1/dist. Consistent with all existing games.
    /// Formula: strength × planet_strength × radius / dist
    Linear,

    /// pull ∝ 1/dist². Physically accurate inverse-square law.
    /// Formula: strength × planet_strength × radius² / dist²
    /// At the planet surface (dist == radius) produces identical magnitude to Linear.
    /// Falls off faster at long range; enables realistic orbital mechanics.
    InverseSquare,
}

impl Default for GravityFalloff {
    fn default() -> Self { GravityFalloff::Linear }
}
