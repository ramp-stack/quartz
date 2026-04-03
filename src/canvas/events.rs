use prism::event::{OnEvent, Event, TickEvent};
use prism::drawable::SizedTree;
use prism::Context;

use super::core::Canvas;
use crate::types::GameEvent;

impl OnEvent for Canvas {
    fn on_event(
        &mut self,
        _ctx:  &mut Context,
        _tree: &SizedTree,
        event: Box<dyn Event>,
    ) -> Vec<Box<dyn Event>> {
        if let Some(kb_evt) = event.downcast_ref::<prism::event::KeyboardEvent>() {
            self.handle_keyboard_event(kb_evt);
        }

        if let Some(mouse_evt) = event.downcast_ref::<prism::event::MouseEvent>() {
            self.handle_mouse_event(mouse_evt.clone());
        }

        if let Some(_tick) = event.downcast_ref::<TickEvent>() {
            if self.paused { return vec![event]; }
            const DELTA_TIME: f32 = 0.016;

            let mut tick_cbs = std::mem::take(&mut self.callbacks.tick);
            tick_cbs.iter_mut().for_each(|cb| cb(self));
            self.callbacks.tick = tick_cbs;

            self.process_held_key_events();
            self.process_all_tick_events();

            if let Some(pos) = self.mouse.position {
                let vpos = self.screen_to_virtual(pos);
                self.process_mouse_over_events(vpos);
            }

            // Fire named custom events registered on objects.
            let custom_names: Vec<String> = self.store.events.iter()
                .flatten()
                .filter_map(|e| {
                    if GameEvent::is_custom(e) {
                        e.custom_name().map(str::to_string)
                    } else {
                        None
                    }
                })
                .collect();

            for name in custom_names {
                if let Some(mut handler) = self.callbacks.custom.remove(&name) {
                    handler(self);
                    self.callbacks.custom.insert(name, handler);
                }
            }

            self.process_hot_reloads(DELTA_TIME);
            self.update_objects(DELTA_TIME);
            self.handle_collisions();

            let canvas_size = self.layout.canvas_size.get();
            let boundary_indices: Vec<usize> = self.store.objects.iter()
                .enumerate()
                .filter(|(_, obj)| obj.visible && obj.check_boundary_collision(canvas_size))
                .map(|(i, _)| i)
                .collect();
            for idx in boundary_indices {
                self.trigger_boundary_collision_events(idx);
            }
        }

        vec![event]
    }
}

impl Canvas {
    pub fn canvas_size(&self) -> (f32, f32) {
        self.layout.canvas_size.get()
    }

    pub(crate) fn screen_to_virtual(&self, screen_pos: (f32, f32)) -> (f32, f32) {
        let scale = self.layout.scale.get();
        let (pad_x, pad_y) = self.layout.safe_area_offset.get();
        if scale == 0.0 { return screen_pos; }
        ((screen_pos.0 - pad_x) / scale, (screen_pos.1 - pad_y) / scale)
    }

    pub(crate) fn process_all_tick_events(&mut self) {
        let actions: Vec<_> = self.store.events.iter()
            .flatten()
            .filter(|e| GameEvent::is_tick(e))
            .map(|e| e.action().clone())
            .collect();
        actions.into_iter().for_each(|a| self.run(a));
    }
}