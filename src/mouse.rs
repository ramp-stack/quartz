use crate::{Canvas, MouseButton, ScrollAxis};
use crate::game_object::GameEvent;

pub trait MouseCallback: FnMut(&mut Canvas, MouseButton, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseCallback>;
}
impl<F> MouseCallback for F where F: FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static {
    fn clone_box(&self) -> Box<dyn MouseCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

pub trait MouseMoveCallback: FnMut(&mut Canvas, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseMoveCallback>;
}
impl<F> MouseMoveCallback for F where F: FnMut(&mut Canvas, (f32, f32)) + Clone + 'static {
    fn clone_box(&self) -> Box<dyn MouseMoveCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseMoveCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

pub trait MouseScrollCallback: FnMut(&mut Canvas, (f32, f32)) + 'static {
    fn clone_box(&self) -> Box<dyn MouseScrollCallback>;
}
impl<F> MouseScrollCallback for F where F: FnMut(&mut Canvas, (f32, f32)) + Clone + 'static {
    fn clone_box(&self) -> Box<dyn MouseScrollCallback> { Box::new(self.clone()) }
}
impl Clone for Box<dyn MouseScrollCallback> {
    fn clone(&self) -> Self { self.as_ref().clone_box() }
}

impl Canvas {
    pub fn on_mouse_press(
        &mut self,
        cb: impl FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse_press_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_release(
        &mut self,
        cb: impl FnMut(&mut Canvas, MouseButton, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse_release_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_move(
        &mut self,
        cb: impl FnMut(&mut Canvas, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse_move_callbacks.push(Box::new(cb));
    }

    pub fn on_mouse_scroll(
        &mut self,
        cb: impl FnMut(&mut Canvas, (f32, f32)) + Clone + 'static,
    ) {
        self.mouse_scroll_callbacks.push(Box::new(cb));
    }

    pub fn mouse_position(&self) -> Option<(f32, f32)> {
        self.mouse_position
    }

    pub(crate) fn handle_mouse_event(&mut self, evt: prism::event::MouseEvent) {
        use prism::event::MouseState;

        let screen_pos = match evt.position {
            Some(p) => p,
            None => {
                if !self.hovered_indices.is_empty() {
                    let leaving: Vec<usize> = self.hovered_indices.drain().collect();
                    for idx in leaving {
                        self.trigger_mouse_leave_events(idx);
                    }
                }
                self.mouse_position = None;
                return;
            }
        };

        let vpos = self.screen_to_virtual(screen_pos);

        match evt.state {
            MouseState::Pressed => {
                self.mouse_position = Some(vpos);
                let btn = MouseButton::Left;

                let mut cbs = std::mem::take(&mut self.mouse_press_callbacks);
                for cb in cbs.iter_mut() {
                    cb(self, btn, vpos);
                }
                self.mouse_press_callbacks = cbs;

                self.process_mouse_press_events(vpos, btn);
            }

            MouseState::Released => {
                self.mouse_position = Some(vpos);
                let btn = MouseButton::Left;

                let mut cbs = std::mem::take(&mut self.mouse_release_callbacks);
                for cb in cbs.iter_mut() {
                    cb(self, btn, vpos);
                }
                self.mouse_release_callbacks = cbs;

                self.process_mouse_release_events(vpos, btn);
            }

            MouseState::Moved => {
                self.mouse_position = Some(vpos);

                let mut cbs = std::mem::take(&mut self.mouse_move_callbacks);
                for cb in cbs.iter_mut() {
                    cb(self, vpos);
                }
                self.mouse_move_callbacks = cbs;
                self.process_mouse_move_events(vpos);
                self.update_hover_state(vpos);
            }

            MouseState::Scroll(dx, dy) => {
                let delta = (dx, dy);
                let mut cbs = std::mem::take(&mut self.mouse_scroll_callbacks);
                for cb in cbs.iter_mut() {
                    cb(self, delta);
                }
                self.mouse_scroll_callbacks = cbs;
                self.process_mouse_scroll_events(vpos, dx, dy);
            }
        }
    }

    pub(crate) fn objects_under_cursor(&self, vpos: (f32, f32)) -> Vec<usize> {
        (0..self.objects.len())
            .filter(|&idx| {
                self.objects[idx].visible && self.objects[idx].contains_point(vpos)
            })
            .collect()
    }

    pub(crate) fn update_hover_state(&mut self, vpos: (f32, f32)) {
        use std::collections::HashSet;

        let now_hovered: HashSet<usize> = self.objects_under_cursor(vpos).into_iter().collect();

        let entered: Vec<usize> = now_hovered.difference(&self.hovered_indices).copied().collect();
        for idx in entered {
            self.trigger_mouse_enter_events(idx);
        }

        let left: Vec<usize> = self.hovered_indices.difference(&now_hovered).copied().collect();
        for idx in left {
            self.trigger_mouse_leave_events(idx);
        }

        self.hovered_indices = now_hovered;
    }

    pub(crate) fn process_mouse_press_events(&mut self, vpos: (f32, f32), pressed_btn: MouseButton) {
        let actions: Vec<_> = self
            .objects_under_cursor(vpos)
            .into_iter()
            .flat_map(|idx| {
                self.object_events
                    .get(idx)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| {
                        if let GameEvent::MousePress { action, button, .. } = e {
                            let matches = button.map_or(true, |b| b == pressed_btn);
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

    pub(crate) fn process_mouse_release_events(&mut self, vpos: (f32, f32), released_btn: MouseButton) {
        let actions: Vec<_> = self
            .objects_under_cursor(vpos)
            .into_iter()
            .flat_map(|idx| {
                self.object_events
                    .get(idx)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseRelease { action, button, .. } = e {
                            let matches = button.map_or(true, |b| b == released_btn);
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

    pub(crate) fn process_mouse_move_events(&mut self, vpos: (f32, f32)) {
        let actions: Vec<_> = self
            .objects_under_cursor(vpos)
            .into_iter()
            .flat_map(|idx| {
                self.object_events
                    .get(idx)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseMove { action, .. } = e {
                            Some(action.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_over_events(&mut self, vpos: (f32, f32)) {
        let actions: Vec<_> = self
            .objects_under_cursor(vpos)
            .into_iter()
            .flat_map(|idx| {
                self.object_events
                    .get(idx)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseOver { action, .. } = e {
                            Some(action.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn process_mouse_scroll_events(&mut self, vpos: (f32, f32), dx: f32, dy: f32) {
        let actions: Vec<_> = self
            .objects_under_cursor(vpos)
            .into_iter()
            .flat_map(|idx| {
                self.object_events
                    .get(idx)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| {
                        if let GameEvent::MouseScroll { action, axis, .. } = e {
                            let matches = match axis {
                                None => true,
                                Some(ScrollAxis::Up) => dy < 0.0,
                                Some(ScrollAxis::Down) => dy > 0.0,
                                Some(ScrollAxis::Left) => dx < 0.0,
                                Some(ScrollAxis::Right) => dx > 0.0,
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
        let actions: Vec<_> = self
            .object_events
            .get(idx)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                if let GameEvent::MouseEnter { action, .. } = e {
                    Some(action.clone())
                } else {
                    None
                }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }

    pub(crate) fn trigger_mouse_leave_events(&mut self, idx: usize) {
        let actions: Vec<_> = self
            .object_events
            .get(idx)
            .into_iter()
            .flatten()
            .filter_map(|e| {
                if let GameEvent::MouseLeave { action, .. } = e {
                    Some(action.clone())
                } else {
                    None
                }
            })
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }
}