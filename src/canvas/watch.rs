use super::core::Canvas;
use crate::file_watcher;

impl Canvas {
    pub fn watch_file<F>(&mut self, path: impl Into<String>, handler: F)
    where
        F: FnMut(&mut Canvas, &[u8]) + Clone + 'static,
    {
        let path  = path.into();
        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        self.file_watchers.push(file_watcher::FileWatcher {
            path,
            mtime,
            handler: Box::new(handler),
        });
    }

    pub fn watch_source<T>(
        &mut self,
        path:   impl Into<String>,
        target: file_watcher::Shared<T>,
    )
    where
        T: file_watcher::FromSource + Clone + 'static,
    {
        self.watch_file(path, move |_cv, bytes| {
            let Ok(src) = std::str::from_utf8(bytes) else { return };
            let new_val = T::from_source(&file_watcher::SourceSettings::parse(src));
            target.set(new_val);
        });
    }
}