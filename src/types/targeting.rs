#[derive(Debug, Clone)]
pub enum Target {
    ByName(String),
    ById(String),
    ByTag(String),
}

impl Target {
    pub fn name(s: impl Into<String>) -> Self { Target::ByName(s.into()) }
    pub fn id(s: impl Into<String>)   -> Self { Target::ById(s.into()) }
    pub fn tag(s: impl Into<String>)  -> Self { Target::ByTag(s.into()) }
}

#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone)]
pub enum Location {
    Position((f32, f32)),
    Between(Box<Target>, Box<Target>),
    AtTarget(Box<Target>),
    Relative {
        target: Box<Target>,
        offset: (f32, f32),
    },
    OnTarget {
        target: Box<Target>,
        anchor: Anchor,
        offset: (f32, f32),
    },
}

impl Location {
    pub fn at(x: f32, y: f32) -> Self {
        Location::Position((x, y))
    }

    pub fn at_target(target: Target) -> Self {
        Location::AtTarget(Box::new(target))
    }

    pub fn between(t1: Target, t2: Target) -> Self {
        Location::Between(Box::new(t1), Box::new(t2))
    }

    pub fn relative_to(target: Target, offset: (f32, f32)) -> Self {
        Location::Relative {
            target: Box::new(target),
            offset,
        }
    }

    pub fn on_target(target: Target, anchor: Anchor, offset: (f32, f32)) -> Self {
        Location::OnTarget {
            target: Box::new(target),
            anchor,
            offset,
        }
    }
}