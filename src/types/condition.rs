use crate::value::{Expr, CompOp};
use super::targeting::Target;

#[derive(Debug, Clone)]
pub enum Condition {
    Always,
    KeyHeld(prism::event::Key),
    KeyNotHeld(prism::event::Key),
    Collision(Target),
    NoCollision(Target),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    IsVisible(Target),
    IsHidden(Target),
    Compare(Expr, CompOp, Expr),
    VarExists(String),
    Grounded(Target),
    Expr(String),
    HasTag(Target, String),

    // -- Crystalline physics conditions ---
    IsSleeping(Target),
    IsMoving(Target),
    SpeedAbove(Target, f32),
    SpeedBelow(Target, f32),
    CrystallineEnabled,
    EmitterActive(String),

    // -- Planet gravity conditions ---
    OnPlanet(Target, Target),
    InGravityField(Target, Target),
    HasDominantPlanet(Target),
    DominantPlanetIs(Target, Target),
    InAnyGravityField(Target),
}

impl Condition {
    pub fn expr(s: impl Into<String>) -> Self { Condition::Expr(s.into()) }

    pub fn expr_checked(s: impl Into<String>) -> Result<Self, String> {
        let src = s.into();
        crate::expr::parse_condition(&src)?;
        Ok(Condition::Expr(src))
    }
}

pub trait ConditionOps {
    fn and(self, other: Condition) -> Condition;
    fn or(self, other: Condition)  -> Condition;
    fn not(self)                   -> Condition;
}

impl ConditionOps for Condition {
    fn and(self, other: Condition) -> Condition {
        Condition::And(Box::new(self), Box::new(other))
    }
    fn or(self, other: Condition) -> Condition {
        Condition::Or(Box::new(self), Box::new(other))
    }
    fn not(self) -> Condition {
        Condition::Not(Box::new(self))
    }
}