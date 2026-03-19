use std::collections::HashMap;
use crate::Canvas;

pub trait EventCallback: FnMut(&mut Canvas) + 'static {
    fn clone_box(&self) -> Box<dyn EventCallback>;
}
impl<F: FnMut(&mut Canvas) + Clone + 'static> EventCallback for F {
    fn clone_box(&self) -> Box<dyn EventCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn EventCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}
impl std::fmt::Debug for dyn EventCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventCallback")
    }
}

#[derive(Default, Debug)]
pub struct CallbackStore {
    pub tick: Vec<Box<dyn EventCallback>>,
    pub custom: HashMap<String, Box<dyn EventCallback>>,
}

impl Clone for CallbackStore {
    fn clone(&self) -> Self {
        Self {
            tick: self.tick.clone(),
            custom: self.custom.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        }
    }
}

impl CallbackStore {
    pub fn new() -> Self { Self::default() }
}