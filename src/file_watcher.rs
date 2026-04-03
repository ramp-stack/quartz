use crate::Canvas;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::{Cell, Ref, RefCell, RefMut};

pub struct Shared<T> {
    value:   Rc<RefCell<T>>,
    changed: Rc<Cell<bool>>,
}

impl<T> Shared<T> {
    pub fn new(val: T) -> Self {
        Self {
            value:   Rc::new(RefCell::new(val)),
            changed: Rc::new(Cell::new(false)),
        }
    }

    pub fn get(&self) -> Ref<'_, T> {
        self.value.borrow()
    }

    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.value.borrow_mut()
    }

    pub fn update<F: FnOnce(&mut T)>(&self, f: F) {
        f(&mut self.value.borrow_mut());
    }

    pub fn set(&self, val: T) {
        *self.value.borrow_mut() = val;
        self.changed.set(true);
    }

    pub fn changed(&self) -> bool {
        self.changed.replace(false)
    }

    pub(crate) fn value_rc(&self) -> &Rc<RefCell<T>> {
        &self.value
    }

    pub(crate) fn changed_rc(&self) -> &Rc<Cell<bool>> {
        &self.changed
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self {
            value:   Rc::clone(&self.value),
            changed: Rc::clone(&self.changed),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shared({:?})", self.value.borrow())
    }
}


#[derive(Debug, Clone, Default)]
pub struct SourceSettings {
    strings: HashMap<String, String>,
    floats:  HashMap<String, f32>,
    usizes:  HashMap<String, usize>,
}

impl SourceSettings {
    pub fn parse(src: &str) -> Self {
        let mut out = Self::default();
        for raw in src.lines() {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") { continue; }
            let Some((key, rest)) = split_key(line) else { continue };
            let rest = rest.trim();

            if rest.starts_with('"') {
                if let Some(end) = rest[1..].find('"') {
                    out.strings.insert(key.to_string(), rest[1..1 + end].to_string());
                    continue;
                }
            }

            let num = rest.trim_end_matches(',').trim();
            if !num.contains('.') {
                if let Ok(v) = num.parse::<usize>() {
                    out.usizes.insert(key.to_string(), v);
                    continue;
                }
            }
            if let Ok(v) = num.parse::<f32>() {
                out.floats.insert(key.to_string(), v);
            }
        }
        out
    }

    pub fn str(&self, key: &str)   -> Option<String> { self.strings.get(key).cloned() }
    pub fn f32(&self, key: &str)   -> Option<f32>    { self.floats.get(key).copied() }
    pub fn usize(&self, key: &str) -> Option<usize>  { self.usizes.get(key).copied() }
}

fn split_key(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let key   = line[..colon].trim();
    if key.is_empty() || key.contains(' ') || key.contains('"') { return None; }
    Some((key, &line[colon + 1..]))
}

pub trait FromSource: Sized {
    fn from_source(p: &SourceSettings) -> Self;
}

pub(crate) trait FileWatchCallback: 'static {
    fn call(&mut self, canvas: &mut Canvas, bytes: &[u8]);
    fn clone_box(&self) -> Box<dyn FileWatchCallback>;
}

impl<F> FileWatchCallback for F
where
    F: FnMut(&mut Canvas, &[u8]) + Clone + 'static,
{
    fn call(&mut self, canvas: &mut Canvas, bytes: &[u8]) {
        (self)(canvas, bytes)
    }
    fn clone_box(&self) -> Box<dyn FileWatchCallback> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn FileWatchCallback> {
    fn clone(&self) -> Self {
        self.as_ref().clone_box()
    }
}

#[derive(Clone)]
pub(crate) struct FileWatcher {
    pub path:    String,
    pub mtime:   Option<std::time::SystemTime>,
    pub handler: Box<dyn FileWatchCallback>,
}