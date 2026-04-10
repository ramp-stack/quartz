use std::collections::{HashSet, HashMap};
use prism::event::{Key, KeyboardEvent, KeyboardState};
use crate::{Canvas, MouseButton, ScrollAxis, GameEvent};

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
    pub held_keys:         HashSet<Key>,
    pub press_callbacks:   Vec<Box<dyn Callback>>,
    pub release_callbacks: Vec<Box<dyn Callback>>,
}

impl Clone for InputState {
    fn clone(&self) -> Self {
        Self {
            held_keys:         self.held_keys.clone(),
            press_callbacks:   self.press_callbacks.clone(),
            release_callbacks: self.release_callbacks.clone(),
        }
    }
}

impl InputState {
    pub fn new() -> Self { Self::default() }
}

pub trait MouseCallback: FnMut(&mut Canvas, MouseButton, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseCallback>;
}
impl<F: FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static> MouseCallback for F {
    fn clone_box(&self) -> Box<dyn MouseCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

pub trait MouseMoveCallback: FnMut(&mut Canvas, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseMoveCallback>;
}
impl<F: FnMut(&mut Canvas, (f32, f32)) + Clone + 'static> MouseMoveCallback for F {
    fn clone_box(&self) -> Box<dyn MouseMoveCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseMoveCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

pub trait MouseScrollCallback: FnMut(&mut Canvas, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseScrollCallback>;
}
impl<F: FnMut(&mut Canvas, (f32, f32)) + Clone + 'static> MouseScrollCallback for F {
    fn clone_box(&self) -> Box<dyn MouseScrollCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseScrollCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

#[derive(Default)]
pub struct MouseState {
    pub position:          Option<(f32, f32)>,
    pub hovered_indices:   HashSet<usize>,
    pub press_callbacks:   Vec<Box<dyn MouseCallback>>,
    pub release_callbacks: Vec<Box<dyn MouseCallback>>,
    pub move_callbacks:    Vec<Box<dyn MouseMoveCallback>>,
    pub scroll_callbacks:  Vec<Box<dyn MouseScrollCallback>>,
}

impl Clone for MouseState {
    fn clone(&self) -> Self {
        Self {
            position:          self.position,
            hovered_indices:   self.hovered_indices.clone(),
            press_callbacks:   self.press_callbacks.clone(),
            release_callbacks: self.release_callbacks.clone(),
            move_callbacks:    self.move_callbacks.clone(),
            scroll_callbacks:  self.scroll_callbacks.clone(),
        }
    }
}

impl std::fmt::Debug for MouseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MouseState")
            .field("position", &self.position)
            .field("hovered_count", &self.hovered_indices.len())
            .finish()
    }
}

impl MouseState {
    pub fn new() -> Self { Self::default() }
}

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
    pub tick:   Vec<Box<dyn EventCallback>>,
    pub custom: HashMap<String, Box<dyn EventCallback>>,
}

impl Clone for CallbackStore {
    fn clone(&self) -> Self {
        Self {
            tick:   self.tick.clone(),
            custom: self.custom.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        }
    }
}

impl CallbackStore {
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
        let KeyboardEvent { state, key, .. } = evt;
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

    pub fn on_mouse_press(
        &mut self,
        cb: impl FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse.press_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_release(
        &mut self,
        cb: impl FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse.release_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_move(
        &mut self,
        cb: impl FnMut(&mut Canvas, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse.move_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_scroll(
        &mut self,
        cb: impl FnMut(&mut Canvas, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse.scroll_callbacks.push(Box::new(cb));
    }

    pub fn mouse_position(&self) -> Option<(f32, f32)> {
        self.mouse.position
    }

    pub(crate) fn handle_mouse_event(&mut self, evt: prism::event::MouseEvent) {
        use prism::event::MouseState as PrismMouseState;

        let screen_pos = match evt.position {
            Some(p) => p,
            None => {
                let leaving: Vec<usize> = self.mouse.hovered_indices.drain().collect();
                for idx in leaving {
                    self.trigger_mouse_leave_events(idx);
                }
                self.mouse.position = None;
                return;
            }
        };

        let vpos = self.screen_to_virtual(screen_pos);

        match evt.state {
            PrismMouseState::Pressed => {
                self.mouse.position = Some(vpos);
                let btn = MouseButton::Left;

                let mut cbs = std::mem::take(&mut self.mouse.press_callbacks);
                for cb in cbs.iter_mut() { cb(self, btn, vpos); }
                self.mouse.press_callbacks = cbs;

                self.process_mouse_press_events(vpos, btn);
            }
            PrismMouseState::Released => {
                self.mouse.position = Some(vpos);
                let btn = MouseButton::Left;

                let mut cbs = std::mem::take(&mut self.mouse.release_callbacks);
                for cb in cbs.iter_mut() { cb(self, btn, vpos); }
                self.mouse.release_callbacks = cbs;

                self.process_mouse_release_events(vpos, btn);
            }
            PrismMouseState::Moved => {
                self.mouse.position = Some(vpos);

                let mut cbs = std::mem::take(&mut self.mouse.move_callbacks);
                for cb in cbs.iter_mut() { cb(self, vpos); }
                self.mouse.move_callbacks = cbs;

                self.process_mouse_move_events(vpos);
                self.update_hover_state(vpos);
            }
            PrismMouseState::Scroll(dx, dy) => {
                let mut cbs = std::mem::take(&mut self.mouse.scroll_callbacks);
                for cb in cbs.iter_mut() { cb(self, (dx, dy)); }
                self.mouse.scroll_callbacks = cbs;

                self.process_mouse_scroll_events(vpos, dx, dy);
            }
        }
    }

    pub(crate) fn objects_under_cursor(&self, vpos: (f32, f32)) -> Vec<usize> {
        (0..self.store.objects.len())
            .filter(|&idx| {
                self.store.objects[idx].visible
                    && self.store.objects[idx].contains_point(vpos)
            })
            .collect()
    }

    pub(crate) fn update_hover_state(&mut self, vpos: (f32, f32)) {
        let now_hovered: HashSet<usize> =
            self.objects_under_cursor(vpos).into_iter().collect();

        let entered: Vec<usize> = now_hovered
            .difference(&self.mouse.hovered_indices)
            .copied()
            .collect();
        for idx in entered {
            self.trigger_mouse_enter_events(idx);
        }

        let left: Vec<usize> = self.mouse.hovered_indices
            .difference(&now_hovered)
            .copied()
            .collect();
        for idx in left {
            self.trigger_mouse_leave_events(idx);
        }

        self.mouse.hovered_indices = now_hovered;
    }

    pub(crate) fn process_mouse_press_events(&mut self, vpos: (f32, f32), pressed_btn: MouseButton) {
        let actions: Vec<_> = self.objects_under_cursor(vpos).into_iter()
            .flat_map(|idx| {
                self.store.events.get(idx).into_iter().flatten()
                    .filter_map(|e| {
                        if let GameEvent::MousePress { action, button, .. } = e {
                            if button.map_or(true, |b| b == pressed_btn) {
                                Some(action.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_release_events(&mut self, vpos: (f32, f32), released_btn: MouseButton) {
        let actions: Vec<_> = self.objects_under_cursor(vpos).into_iter()
            .flat_map(|idx| {
                self.store.events.get(idx).into_iter().flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseRelease { action, button, .. } = e {
                            if button.map_or(true, |b| b == released_btn) {
                                Some(action.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_move_events(&mut self, vpos: (f32, f32)) {
        let actions: Vec<_> = self.objects_under_cursor(vpos).into_iter()
            .flat_map(|idx| {
                self.store.events.get(idx).into_iter().flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseMove { action, .. } = e { Some(action.clone()) } else { None }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_over_events(&mut self, vpos: (f32, f32)) {
        let actions: Vec<_> = self.objects_under_cursor(vpos).into_iter()
            .flat_map(|idx| {
                self.store.events.get(idx).into_iter().flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseOver { action, .. } = e { Some(action.clone()) } else { None }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_scroll_events(&mut self, vpos: (f32, f32), dx: f32, dy: f32) {
        let actions: Vec<_> = self.objects_under_cursor(vpos).into_iter()
            .flat_map(|idx| {
                self.store.events.get(idx).into_iter().flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseScroll { action, axis, .. } = e {
                            let matches = match axis {
                                None                     => true,
                                Some(ScrollAxis::Up)     => dy < 0.0,
                                Some(ScrollAxis::Down)   => dy > 0.0,
                                Some(ScrollAxis::Left)   => dx < 0.0,
                                Some(ScrollAxis::Right)  => dx > 0.0,
                            };
                            if matches { Some(action.clone()) } else { None }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn trigger_mouse_enter_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| {
                if let GameEvent::MouseEnter { action, .. } = e { Some(action.clone()) } else { None }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn trigger_mouse_leave_events(&mut self, idx: usize) {
        let actions: Vec<_> = self.store.events.get(idx).into_iter().flatten()
            .filter_map(|e| {
                if let GameEvent::MouseLeave { action, .. } = e { Some(action.clone()) } else { None }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }
}