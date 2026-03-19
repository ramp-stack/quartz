use std::collections::HashMap;

use crate::game_object::{GameObject, GameEvent, Target};

#[derive(Debug, Default)]
pub struct ObjectStore {
    pub objects: Vec<GameObject>,
    pub names: Vec<String>,
    pub name_to_index: HashMap<String, usize>,
    pub id_to_index: HashMap<String, usize>,
    pub events: Vec<Vec<GameEvent>>,
    pub tag_to_indices: HashMap<String, Vec<usize>>,
}

impl Clone for ObjectStore {
    fn clone(&self) -> Self {
        Self {
            objects: self.objects.clone(),
            names: self.names.clone(),
            name_to_index: self.name_to_index.clone(),
            id_to_index: self.id_to_index.clone(),
            events: self.events.iter().map(|v| v.iter().map(|e| e.clone()).collect()).collect(),
            tag_to_indices: self.tag_to_indices.clone(),
        }
    }
}

impl ObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: String, obj: GameObject) {
        let idx = self.objects.len();

        self.name_to_index.insert(name.clone(), idx);
        self.id_to_index.insert(obj.id.clone(), idx);

        for tag in &obj.tags {
            self.tag_to_indices.entry(tag.clone()).or_default().push(idx);
        }

        self.names.push(name);
        self.objects.push(obj);
        self.events.push(Vec::new());
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let idx = match self.name_to_index.get(name) {
            Some(&i) => i,
            None => return false,
        };

        let removed_obj = self.objects.remove(idx);
        let removed_name = self.names.remove(idx);
        self.events.remove(idx);

        self.name_to_index.remove(&removed_name);
        self.id_to_index.remove(&removed_obj.id);

        for tag in &removed_obj.tags {
            if let Some(indices) = self.tag_to_indices.get_mut(tag) {
                indices.retain(|&i| i != idx);
            }
        }

        self.name_to_index.values_mut().for_each(|i| if *i > idx { *i -= 1 });
        self.id_to_index.values_mut().for_each(|i| if *i > idx { *i -= 1 });
        self.tag_to_indices.values_mut().for_each(|indices| {
            indices.iter_mut().for_each(|i| if *i > idx { *i -= 1 });
        });

        true
    }

    pub fn get_indices(&self, target: &Target) -> Vec<usize> {
        match target {
            Target::ByName(name) => self.name_to_index.get(name).map(|&i| vec![i]).unwrap_or_default(),
            Target::ById(id)     => self.id_to_index.get(id).map(|&i| vec![i]).unwrap_or_default(),
            Target::ByTag(tag)   => self.tag_to_indices.get(tag).cloned().unwrap_or_default(),
        }
    }

    pub fn get_names(&self, target: &Target) -> Vec<String> {
        self.get_indices(target)
            .iter()
            .filter_map(|&i| self.names.get(i).cloned())
            .collect()
    }

    pub fn apply_to_targets<F>(&mut self, target: &Target, mut f: F)
    where
        F: FnMut(&mut GameObject),
    {
        let indices = self.get_indices(target);
        for idx in indices {
            if let Some(obj) = self.objects.get_mut(idx) {
                f(obj);
            }
        }
    }
}