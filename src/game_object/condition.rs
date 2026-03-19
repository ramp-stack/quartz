use super::target::Target;

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
}