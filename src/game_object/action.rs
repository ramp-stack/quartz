use super::target::{Target, Location};
use super::condition::Condition;
use super::game_object::GameObject;

#[derive(Clone, Debug)]
pub enum Action {
    ApplyMomentum {
        target: Target,
        value: (f32, f32),
    },
    SetMomentum {
        target: Target,
        value: (f32, f32),
    },
    Spawn {
        object: Box<GameObject>,
        location: Location,
    },
    SetResistance {
        target: Target,
        value: (f32, f32),
    },
    Remove {
        target: Target,
    },
    TransferMomentum {
        from: Target,
        to: Target,
        scale: f32,
    },
    SetAnimation {
        target: Target,
        animation_bytes: &'static [u8],
        fps: f32,
    },
    Teleport {
        target: Target,
        location: Location,
    },
    Show {
        target: Target,
    },
    Hide {
        target: Target,
    },
    Toggle {
        target: Target,
    },
    Conditional {
        condition: Condition,
        if_true: Box<Action>,
        if_false: Option<Box<Action>>,
    },
    Custom {
        name: String,
    },
}