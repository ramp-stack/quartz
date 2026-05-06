pub mod types;
pub mod broadphase;
mod contacts;
pub mod solver;
pub mod particles;

// Re-exports for convenience
pub use types::{
    BodyUpdate, CollisionShape, CrystallineCollisionMode, PhysicsBody, PhysicsConfig,
    PhysicsMaterial, PhysicsQuality, PhysicsStepResult,
};
pub use broadphase::{Aabb, AabbPairFinder};
pub use solver::CrystallinePhysics;
pub use solver::SleepState;
pub use particles::types::{CollisionResponse, Emitter, EmitterBuilder, Particle, ParticleShape};
pub use particles::system::{ParticleState, ParticleStepResult, ParticleSystem};
