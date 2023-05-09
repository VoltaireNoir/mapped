#![doc = include_str!("../README.md")]

pub mod mappers;
pub mod memoize;
pub mod palette;

use image::{DynamicImage, GenericImageView, Rgba};
use mappers::Nearest;
use memoize::Memoized;
use palette::Rgbx;

use std::{
    error::Error,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use rayon::prelude::*;

pub struct Processor<'a, 'b, M>
where
    M: Mapper,
{
    conf: ProcOptions<'a, 'b, M>,
    data: DynamicImage,
    prog: Progress,
}

impl<'a, 'b, M> Processor<'a, 'b, M>
where
    M: Mapper,
{
    pub fn process(&self) -> ProcessedData {
        let img_pixels: Vec<_> = self.data.pixels().map(|(_, _, rgb)| rgb).collect();

        let ProcOptions {
            mapper,
            threads,
            palette,
            ..
        } = &self.conf;

        let raw: Vec<u8> = match threads {
            Threads::Single => img_pixels
                .iter()
                .flat_map(|pixel| mapper.predict(palette, &pixel.0))
                .collect(),
            Threads::Auto => self.dispatch(
                img_pixels
                    .chunks(img_pixels.len() / ThreadCount::calculate().get())
                    .collect(),
            ),
            Threads::Custom(n) => {
                self.dispatch(img_pixels.chunks(img_pixels.len() / n.get()).collect())
            }
            Threads::Rayon => img_pixels
                .par_iter()
                .flat_map(|x| mapper.predict(palette, &x.0))
                .collect(),
            Threads::Extreme => self.dispatch(
                img_pixels
                    .chunks(img_pixels.len() / ThreadCount::extreme().get())
                    .collect(),
            ),
        };

        ProcessedData {
            raw,
            out: self.conf.output.and_then(|p| Some(p.to_path_buf())),
            dimen: self.data.dimensions(),
        }
    }

    pub fn gen_tracker(&mut self) -> Tracker {
        let (x, y) = self.data.dimensions();
        self.prog.init((x * y) as usize)
    }

    fn dispatch(&self, parts: Vec<&[Rgba<u8>]>) -> Vec<u8> {
        let ProcOptions {
            mapper, palette, ..
        } = &self.conf;

        thread::scope(|s| {
            let mut handles: Vec<thread::ScopedJoinHandle<Vec<u8>>> = Vec::new();
            let mut data: Vec<u8> = Vec::new();
            for part in parts {
                let sender = self.prog.get_sender();
                let h = s.spawn(move || {
                    part.iter()
                        .flat_map(|rgb| {
                            let r = mapper.predict(palette, &rgb.0);
                            sender.notify();
                            r
                        })
                        .collect::<Vec<u8>>()
                });
                handles.push(h);
            }
            for h in handles {
                data.append(&mut h.join().unwrap());
            }
            data
        })
    }
}

pub struct ProcessedData {
    raw: Vec<u8>,
    out: Option<PathBuf>,
    dimen: (u32, u32),
}

impl ProcessedData {
    pub fn raw_buffer(&self) -> &[u8] {
        &self.raw
    }

    pub fn save(&self) -> Result<(), Box<dyn Error + 'static>> {
        let output = if let Some(out) = &self.out {
            out.as_path()
        } else {
            "mapped.png".as_ref()
        };
        self.save_to(output)
    }

    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error + 'static>> {
        let (w, h) = self.dimen;
        image::save_buffer(path, &self.raw, w, h, image::ColorType::Rgba8)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcOptions<'a, 'b, M: Mapper = Nearest> {
    mapper: M,
    output: Option<&'a Path>,
    threads: Threads,
    palette: &'b [Rgbx],
}

impl Default for ProcOptions<'_, '_> {
    fn default() -> Self {
        ProcOptions {
            mapper: Nearest,
            output: None,
            threads: Threads::default(),
            palette: &palette::NORD,
        }
    }
}

impl<'a, 'b, M: Mapper> ProcOptions<'a, 'b, M> {
    #[must_use]
    pub fn new(mapper: M) -> Self {
        ProcOptions {
            mapper,
            output: None,
            threads: Threads::default(),
            palette: &palette::NORD,
        }
    }

    #[must_use]
    pub fn mapper<Map: Mapper>(self, mapper: Map) -> ProcOptions<'a, 'b, Map> {
        ProcOptions {
            mapper,
            output: self.output,
            threads: self.threads,
            palette: self.palette,
        }
    }

    #[must_use]
    pub fn copy_with_mapper<Map: Mapper>(&self, mapper: Map) -> ProcOptions<'a, 'b, Map> {
        ProcOptions {
            mapper,
            output: self.output,
            threads: self.threads,
            palette: self.palette,
        }
    }

    #[must_use]
    pub fn output<P: AsRef<Path> + ?Sized>(mut self, out: &'a P) -> Self {
        self.output = Some(out.as_ref());
        self
    }

    #[must_use]
    pub fn threads(mut self, threads: Threads) -> Self {
        self.threads = threads;
        self
    }

    #[must_use]
    pub fn palette(mut self, palette: &'b [Rgbx]) -> Self {
        self.palette = palette;
        self
    }

    pub fn load<F: AsRef<Path>>(
        self,
        file: F,
    ) -> Result<Processor<'a, 'b, M>, Box<dyn Error + 'static>> {
        let data = image::open(file.as_ref())?;

        Ok(Processor {
            conf: self,
            data,
            prog: Progress::default(),
        })
    }

    pub fn load_bytes(
        self,
        buffer: &[u8],
    ) -> Result<Processor<'a, 'b, M>, Box<dyn Error + 'static>> {
        let data = image::load_from_memory(buffer)?;

        Ok(Processor {
            conf: self,
            data,
            prog: Progress::default(),
        })
    }
}

#[derive(Clone, Default)]
struct Progress(SignalSender);

impl Progress {
    fn init(&mut self, size: usize) -> Tracker {
        let (s, r) = mpsc::channel::<Signal>();
        self.0.replace(s);
        Tracker {
            current: 0,
            total: size,
            receiver: r,
        }
    }
    fn get_sender(&self) -> SignalSender {
        self.0.clone()
    }
}

unsafe impl Sync for Progress {}

#[derive(Clone)]
struct SignalSender(Option<Sender<Signal>>);

impl SignalSender {
    fn notify(&self) {
        if let Some(s) = &self.0 {
            s.send(Signal).unwrap();
        }
    }
}

impl Default for SignalSender {
    fn default() -> Self {
        Self(None)
    }
}

impl Deref for SignalSender {
    type Target = Option<Sender<Signal>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SignalSender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Tracker {
    current: usize,
    total: usize,
    receiver: Receiver<Signal>,
}

struct Signal;

impl Tracker {
    pub fn percentage(&mut self) -> f32 {
        self.track();
        (self.current as f32 / self.total as f32) * 100.0
    }
    pub fn current(&mut self) -> usize {
        self.track();
        self.current
    }
    pub const fn total(&self) -> usize {
        self.total
    }
    fn track(&mut self) {
        self.current += self.receiver.try_iter().count();
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum Threads {
    Single,
    #[default]
    Auto,
    Rayon,
    Custom(ThreadCount),
    Extreme,
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadCount(NonZeroUsize);

impl ThreadCount {
    pub fn new(val: NonZeroUsize) -> Self {
        ThreadCount(val)
    }

    pub fn calculate() -> Self {
        if let Ok(c) = std::thread::available_parallelism() {
            ThreadCount::new(c)
        } else {
            ThreadCount::default()
        }
    }

    fn extreme() -> Self {
        NonZeroUsize::new(2usize.pow((Self::calculate().get() / 2) as u32))
            .unwrap()
            .into()
    }

    fn get(&self) -> usize {
        self.0.get()
    }
}

impl From<NonZeroUsize> for ThreadCount {
    fn from(value: NonZeroUsize) -> Self {
        Self(value)
    }
}

impl Default for ThreadCount {
    fn default() -> Self {
        Self(NonZeroUsize::new(2).unwrap())
    }
}

pub trait Mapper: Send + Sync + Clone {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4];
    fn memoized(self) -> Memoized<Self> {
        self.into()
    }
}
