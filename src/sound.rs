use rodio::{Decoder, OutputStreamBuilder, Sink, Source};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub struct SoundOptions {
    pub volume:       f32,
    pub looping:      bool,
    pub pitch:        f32,
    pub delay_secs:   f32,
    pub fade_in_secs: f32,
    pub pan:          f32,
}

impl Default for SoundOptions {
    fn default() -> Self {
        Self {
            volume:       1.0,
            looping:      false,
            pitch:        1.0,
            delay_secs:   0.0,
            fade_in_secs: 0.0,
            pan:          0.0,
        }
    }
}

impl SoundOptions {
    pub fn new() -> Self { Self::default() }
    pub fn volume(mut self, v: f32)   -> Self { self.volume       = v.max(0.0);        self }
    pub fn looping(mut self, l: bool) -> Self { self.looping      = l;                 self }
    pub fn pitch(mut self, p: f32)    -> Self { self.pitch        = p.max(0.01);       self }
    pub fn delay(mut self, s: f32)    -> Self { self.delay_secs   = s.max(0.0);        self }
    pub fn fade_in(mut self, s: f32)  -> Self { self.fade_in_secs = s.max(0.0);        self }
    pub fn pan(mut self, p: f32)      -> Self { self.pan          = p.clamp(-1.0, 1.0); self }
}

#[derive(Clone)]
pub struct SoundHandle {
    sink:    Arc<Mutex<Sink>>,
    stopped: Arc<AtomicBool>,
    done:    Arc<AtomicBool>,
}

impl SoundHandle {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Ok(s) = self.sink.lock() { s.stop(); }
        self.done.store(true, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        if let Ok(s) = self.sink.lock() { s.pause(); }
    }

    pub fn resume(&self) {
        if let Ok(s) = self.sink.lock() { s.play(); }
    }

    pub fn toggle_pause(&self) {
        if let Ok(s) = self.sink.lock() {
            if s.is_paused() { s.play(); } else { s.pause(); }
        }
    }

    pub fn set_volume(&self, volume: f32) {
        if let Ok(s) = self.sink.lock() { s.set_volume(volume.max(0.0)); }
    }

    pub fn set_speed(&self, speed: f32) {
        if let Ok(s) = self.sink.lock() { s.set_speed(speed.max(0.01)); }
    }

    pub fn fade_to(&self, target: f32, duration_secs: f32) {
        let sink = Arc::clone(&self.sink);
        std::thread::spawn(move || {
            let steps    = 60u32;
            let interval = std::time::Duration::from_secs_f32(duration_secs / steps as f32);
            let start    = sink.lock().ok().map(|s| s.volume()).unwrap_or(1.0);
            for i in 1..=steps {
                std::thread::sleep(interval);
                let vol = start + (target - start) * (i as f32 / steps as f32);
                if let Ok(s) = sink.lock() { s.set_volume(vol.max(0.0)); }
            }
        });
    }

    pub fn fade_out(&self, duration_secs: f32) {
        self.stopped.store(true, Ordering::Relaxed);

        let sink = Arc::clone(&self.sink);
        let done = Arc::clone(&self.done);
        std::thread::spawn(move || {
            let steps    = 60u32;
            let interval = std::time::Duration::from_secs_f32(duration_secs / steps as f32);
            let start    = sink.lock().ok().map(|s| s.volume()).unwrap_or(1.0);
            for i in 1..=steps {
                std::thread::sleep(interval);
                let vol = start * (1.0 - i as f32 / steps as f32);
                if let Ok(s) = sink.lock() { s.set_volume(vol.max(0.0)); }
            }
            if let Ok(s) = sink.lock() { s.stop(); }
            done.store(true, Ordering::Relaxed);
        });
    }

    pub fn is_finished(&self) -> bool { self.done.load(Ordering::Relaxed) }
    pub fn is_paused(&self)   -> bool { self.sink.lock().map(|s| s.is_paused()).unwrap_or(false) }
    pub fn volume(&self)      -> f32  { self.sink.lock().map(|s| s.volume()).unwrap_or(0.0) }
    pub fn speed(&self)       -> f32  { self.sink.lock().map(|s| s.speed()).unwrap_or(1.0) }
}

pub(crate) fn spawn_sound(file_path: &str, options: SoundOptions) -> SoundHandle {
    let path     = file_path.to_string();
    let stopped  = Arc::new(AtomicBool::new(false));
    let done     = Arc::new(AtomicBool::new(false));
    let stopped2 = Arc::clone(&stopped);
    let done2    = Arc::clone(&done);

    let (tx, rx) = std::sync::mpsc::channel::<Arc<Mutex<Sink>>>();

    std::thread::spawn(move || {
        if options.delay_secs > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f32(options.delay_secs));
        }

        let Ok(stream) = OutputStreamBuilder::open_default_stream() else { return; };

        let sink = Sink::connect_new(stream.mixer());
        sink.set_volume(if options.fade_in_secs > 0.0 { 0.0 } else { options.volume });
        sink.set_speed(options.pitch);

        let sink_arc = Arc::new(Mutex::new(sink));
        let _ = tx.send(Arc::clone(&sink_arc));

        if options.fade_in_secs > 0.0 {
            let sink_fade = Arc::clone(&sink_arc);
            let target    = options.volume;
            let dur       = options.fade_in_secs;
            std::thread::spawn(move || {
                let steps    = 60u32;
                let interval = std::time::Duration::from_secs_f32(dur / steps as f32);
                for i in 1..=steps {
                    std::thread::sleep(interval);
                    let vol = target * (i as f32 / steps as f32);
                    if let Ok(s) = sink_fade.lock() { s.set_volume(vol); }
                }
            });
        }

        loop {
            if stopped2.load(Ordering::Relaxed) { break; }

            let Ok(file)   = File::open(&path) else { break; };
            let Ok(source) = Decoder::new(std::io::BufReader::new(file)) else { break; };

            let pan    = options.pan;
            let source = if pan != 0.0 {
                source.amplify(((1.0 - pan.abs()) / 2.0_f32).sqrt() * 2.0)
            } else {
                source.amplify(1.0)
            };

            if let Ok(s) = sink_arc.lock() { s.append(source); }

            loop {
                std::thread::sleep(std::time::Duration::from_millis(10));
                if stopped2.load(Ordering::Relaxed) { break; }
                if let Ok(s) = sink_arc.lock() { if s.empty() { break; } }
            }

            if !options.looping || stopped2.load(Ordering::Relaxed) { break; }
        }

        done2.store(true, Ordering::Relaxed);
    });

    let sink = rx.recv_timeout(std::time::Duration::from_millis(300))
        .unwrap_or_else(|_| {
            let stream = OutputStreamBuilder::open_default_stream().unwrap();
            let sink   = Sink::connect_new(stream.mixer());
            sink.pause();
            std::mem::forget(stream);
            Arc::new(Mutex::new(sink))
        });

    SoundHandle { sink, stopped, done }
}