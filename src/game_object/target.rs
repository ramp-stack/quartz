#[derive(Debug, Clone)]
pub enum Target {
    ByName(String),
    ById(String),
    ByTag(String),
}

impl Target {
    pub fn name(s: impl Into<String>) -> Self { Target::ByName(s.into()) }
    pub fn id(s: impl Into<String>) -> Self { Target::ById(s.into()) }
    pub fn tag(s: impl Into<String>) -> Self { Target::ByTag(s.into()) }
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
}