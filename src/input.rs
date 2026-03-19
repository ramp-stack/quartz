use std::collections::HashSet;
use prism::event::{Key, KeyboardEvent, KeyboardState};

use crate::Canvas;
use crate::game_object::GameEvent;

pub trait Callback: FnMut(&mut Canvas, &Key) + 'static {
    fn clone_box(&self) -> Box<dyn Callback>;
}
impl<F: FnMut(&mut Canvas, &Key) + Clone + 'static> Callback for F {
    fn clone_box(&self) -> Box<dyn Callback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn Callback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}
impl std::fmt::Debug for dyn Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback")
    }
}

#[derive(Default, Debug)]
pub struct InputState {
    pub held_keys: HashSet<Key>,
    pub press_callbacks: Vec<Box<dyn Callback>>,
    pub release_callbacks: Vec<Box<dyn Callback>>,
}

impl Clone for InputState {
    fn clone(&self) -> Self {
        Self {
            held_keys: self.held_keys.clone(),
            press_callbacks: self.press_callbacks.clone(),
            release_callbacks: self.release_callbacks.clone(),
        }
    }
}

impl InputState {
    pub fn new() -> Self { Self::default() }
}

impl Canvas {
    pub fn on_key_press(&mut self, cb: impl FnMut(&mut Canvas, &Key) + Clone + 'static) {
        self.input.press_callbacks.push(Box::new(cb));
    }

    pub fn on_key_release(&mut self, cb: impl FnMut(&mut Canvas, &Key) + Clone + 'static) {
        self.input.release_callbacks.push(Box::new(cb));
    }

    pub fn is_key_held(&self, key: &Key) -> bool {
        self.input.held_keys.contains(key)
    }

    pub(crate) fn handle_keyboard_event(&mut self, evt: &KeyboardEvent) {
        let KeyboardEvent { state, key } = evt;
        match state {
            KeyboardState::Pressed if self.input.held_keys.insert(key.clone()) => {
                println!("key {key:?}");

                let key_clone = key.clone();
                let mut cbs = std::mem::take(&mut self.input.press_callbacks);
                for cb in cbs.iter_mut() { cb(self, &key_clone); }
                self.input.press_callbacks = cbs;

                self.process_key_events(key, GameEvent::is_key_press);
            }
            KeyboardState::Released => {
                self.input.held_keys.remove(key);

                let key_clone = key.clone();
                let mut cbs = std::mem::take(&mut self.input.release_callbacks);
                for cb in cbs.iter_mut() { cb(self, &key_clone); }
                self.input.release_callbacks = cbs;

                self.process_key_events(key, GameEvent::is_key_release);
            }
            _ => {}
        }
    }

    pub(crate) fn process_key_events<F>(&mut self, key: &Key, predicate: F)
    where
        F: Fn(&GameEvent) -> bool,
    {
        let actions: Vec<_> = self.store.events.iter()
            .flatten()
            .filter(|e| predicate(e) && e.key() == Some(key))
            .map(|e| e.action().clone())
            .collect();

        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_held_key_events(&mut self) {
        let held = self.input.held_keys.clone();
        let actions: Vec<_> = self.store.events.iter()
            .flatten()
            .filter(|e| GameEvent::is_key_hold(e) && e.key().map_or(false, |k| held.contains(k)))
            .map(|e| e.action().clone())
            .collect();

        actions.into_iter().for_each(|a| self.run(a));
    }
}