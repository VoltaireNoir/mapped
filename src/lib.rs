pub mod mappers;
pub mod memoize;
pub mod palette;
mod procutils;

use image::{DynamicImage, GenericImageView, Rgba};
use mappers::Nearest;
use memoize::Memoized;
use palette::Rgbx;

use std::{
    error::Error,
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
    conf: &'a ProcOptions<'a, 'b, M>,
    data: DynamicImage,
    prog: SignalSender,
}

impl<'a, 'b, M> Processor<'a, 'b, M>
where
    M: Mapper,
{
    pub fn process(&self) -> ProcessedData {
        use procutils::subdivide;

        let img_pixels: Vec<_> = self.data.pixels().map(|(_, _, rgb)| rgb).collect();

        let ProcOptions {
            mapper,
            threads,
            palette,
            ..
        } = self.conf;

        let raw: Vec<u8> = match threads {
            Threads::Single => img_pixels
                .iter()
                .flat_map(|pixel| mapper.predict(palette, &pixel.0))
                .collect(),
            Threads::Auto => self.dispatch(
                img_pixels
                    .chunks(img_pixels.len() / *ThreadCount::calculate())
                    .collect(),
            ),
            Threads::Custom(n) => {
                self.dispatch(img_pixels.chunks(img_pixels.len() / **n).collect())
            }
            Threads::Rayon => img_pixels
                .par_iter()
                .flat_map(|x| mapper.predict(palette, &x.0))
                .collect(),
            Threads::Extreme => self.dispatch(subdivide(&img_pixels, *ThreadCount::calculate())),
        };

        ProcessedData {
            raw,
            out: self
                .conf
                .output
                .and_then(|p| Some(p.to_path_buf()))
                .or(None),
            dimen: self.data.dimensions(),
        }
    }

    pub fn gen_tracker(&mut self) -> Tracker {
        let (s, r) = mpsc::channel::<Signal>();
        let (x, y) = self.data.dimensions();

        self.prog.replace(s);

        Tracker {
            current: 0,
            total: (x * y) as usize,
            receiver: r,
        }
    }

    fn dispatch(&self, parts: Vec<&[Rgba<u8>]>) -> Vec<u8> {
        let ProcOptions {
            mapper, palette, ..
        } = self.conf;

        thread::scope(|s| {
            let mut handles: Vec<thread::ScopedJoinHandle<Vec<u8>>> = Vec::new();
            let mut data: Vec<u8> = Vec::new();
            for part in parts {
                let sender = self.prog.clone();
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
    pub fn new(mapper: M) -> Self {
        ProcOptions {
            mapper,
            output: None,
            threads: Threads::default(),
            palette: &palette::NORD,
        }
    }

    pub fn swap_mapper<Map: Mapper>(self, mapper: Map) -> ProcOptions<'a, 'b, Map> {
        ProcOptions {
            mapper,
            output: self.output,
            threads: self.threads,
            palette: self.palette,
        }
    }

    pub fn copy_with_mapper<Map: Mapper>(&self, mapper: Map) -> ProcOptions<'a, 'b, Map> {
        ProcOptions {
            mapper,
            output: self.output,
            threads: self.threads,
            palette: self.palette,
        }
    }

    pub fn output<P: AsRef<Path> + ?Sized>(&mut self, out: &'a P) -> &mut Self {
        self.output = Some(out.as_ref());
        self
    }

    pub fn threads(&mut self, threads: Threads) -> &mut Self {
        self.threads = threads;
        self
    }

    pub fn palette(&mut self, palete: &'b [Rgbx]) -> &mut Self {
        self.palette = palete;
        self
    }

    pub fn load<F: AsRef<Path>>(
        &'_ self,
        file: F,
    ) -> Result<Processor<'_, 'b, M>, Box<dyn Error + 'static>> {
        let data = image::open(file.as_ref())?;

        Ok(Processor {
            conf: self,
            data,
            prog: SignalSender::new(),
        })
    }
}

#[derive(Clone)]
struct SignalSender(Option<Sender<Signal>>);

impl SignalSender {
    fn new() -> Self {
        SignalSender(None)
    }

    fn notify(&self) {
        if let Some(s) = &self.0 {
            s.send(Signal).unwrap();
        }
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
pub struct ThreadCount(usize);

impl ThreadCount {
    pub fn new(val: usize) -> Self {
        ThreadCount(val)
    }

    fn calculate() -> Self {
        if let Ok(c) = std::thread::available_parallelism() {
            let c = usize::from(c);
            if c >= 4 {
                ThreadCount::new(c / 2)
            } else {
                ThreadCount::default()
            }
        } else {
            ThreadCount::default()
        }
    }
}

impl Deref for ThreadCount {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for ThreadCount {
    fn default() -> Self {
        Self(1)
    }
}

pub trait Mapper: Send + Sync + Clone {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4];
    fn memoized(self) -> Memoized<Self> {
        self.into()
    }
}
