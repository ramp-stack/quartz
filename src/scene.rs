use std::collections::HashMap;
use crate::{Canvas, GameObject, GameEvent, Target};

pub trait SceneCallback: 'static {
    fn call(&mut self, canvas: &mut Canvas);
    fn clone_box(&self) -> Box<dyn SceneCallback>;
}

impl<F> SceneCallback for F
where
    F: FnMut(&mut Canvas) + Clone + 'static,
{
    fn call(&mut self, canvas: &mut Canvas) {
        (self)(canvas)
    }

    fn clone_box(&self) -> Box<dyn SceneCallback> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn SceneCallback> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}

#[derive(Clone)]
pub struct Scene {
    pub name: String,
    objects: Vec<(String, GameObject)>,
    events: Vec<(GameEvent, Target)>,
    on_enter: Option<Box<dyn SceneCallback>>,
    on_exit: Option<Box<dyn SceneCallback>>,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("name", &self.name)
            .field("objects", &self.objects.iter().map(|(n, _)| n).collect::<Vec<_>>())
            .finish()
    }
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            objects: Vec::new(),
            events: Vec::new(),
            on_enter: None,
            on_exit: None,
        }
    }

    pub fn with_object(mut self, name: impl Into<String>, obj: GameObject) -> Self {
        self.objects.push((name.into(), obj));
        self
    }

    pub fn with_event(mut self, event: GameEvent, target: Target) -> Self {
        self.events.push((event, target));
        self
    }

    pub fn on_enter<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut Canvas) + Clone + 'static,
    {
        self.on_enter = Some(Box::new(f));
        self
    }

    pub fn on_exit<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut Canvas) + Clone + 'static,
    {
        self.on_exit = Some(Box::new(f));
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct SceneManager {
    pub(crate) scenes: HashMap<String, Scene>,
    pub(crate) active_scene: Option<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_scene(&mut self, scene: Scene) {
        self.scenes.insert(scene.name.clone(), scene);
    }

    pub fn active_scene(&self) -> Option<&str> {
        self.active_scene.as_deref()
    }

    pub fn has_scene(&self, name: &str) -> bool {
        self.scenes.contains_key(name)
    }
}

impl Canvas {
    pub fn add_scene(&mut self, scene: Scene) {
        self.scene_manager.add_scene(scene);
    }

    pub fn load_scene(&mut self, name: &str) {
        if let Some(current_name) = self.scene_manager.active_scene.clone() {
            let object_names: Vec<String> = self
                .scene_manager
                .scenes
                .get(&current_name)
                .map(|s| s.objects.iter().map(|(n, _)| n.clone()).collect())
                .unwrap_or_default();

            if let Some(scene) = self.scene_manager.scenes.get_mut(&current_name) {
                if let Some(mut cb) = scene.on_exit.take() {
                    cb.call(self);
                    if let Some(s) = self.scene_manager.scenes.get_mut(&current_name) {
                        s.on_exit = Some(cb);
                    }
                }
            }

            for obj_name in object_names {
                self.remove_game_object(&obj_name);
            }
        }

        let (objects, events, mut on_enter_cb) = match self.scene_manager.scenes.get_mut(name) {
            Some(scene) => {
                let objects = scene.objects.clone();
                let events = scene.events.clone();
                let cb = scene.on_enter.take();
                (objects, events, cb)
            }
            None => {
                eprintln!("[SceneManager] Unknown scene: '{name}'");
                return;
            }
        };

        self.scene_manager.active_scene = Some(name.to_string());

        for (obj_name, obj) in objects {
            self.add_game_object(obj_name, obj);
        }

        for (event, target) in events {
            self.add_event(event, target);
        }

        if let Some(mut cb) = on_enter_cb.take() {
            cb.call(self);
            if let Some(s) = self.scene_manager.scenes.get_mut(name) {
                s.on_enter = Some(cb);
            }
        }
    }

    pub fn active_scene(&self) -> Option<&str> {
        self.scene_manager.active_scene()
    }

    pub fn is_scene(&self, name: &str) -> bool {
        self.scene_manager.active_scene.as_deref() == Some(name)
    }
}