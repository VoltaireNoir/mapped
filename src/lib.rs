pub mod mappers;
pub mod palette;
mod procutils;

use image::{DynamicImage, GenericImageView};
use mappers::Nearest;
use palette::Rgbx;

use std::{
    error::Error,
    ops::{Deref, DerefMut},
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
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
    pub fn process(&self) -> Result<(), Box<dyn Error + 'static>> {
        use procutils::{dispatch_and_join, subdivide};

        let img_pixels: Vec<_> = self.data.pixels().map(|(_, _, rgb)| rgb).collect();

        let ProcOptions {
            mapper,
            threads,
            palette,
            ..
        } = &*self.conf;

        match threads {
            Threads::Single => self.save(
                &img_pixels
                    .iter()
                    .flat_map(|pixel| mapper.predict(self.conf.palette, &pixel.0))
                    .collect::<Vec<u8>>(),
            )?,
            Threads::Auto => self.save(&dispatch_and_join(
                img_pixels.chunks(*ThreadCount::calculate()).collect(),
                palette,
                mapper,
                &self.prog,
            ))?,
            Threads::Custom(n) => self.save(&dispatch_and_join(
                img_pixels.chunks(img_pixels.len() / **n).collect(),
                palette,
                mapper,
                &self.prog,
            ))?,
            Threads::Rayon => self.save(
                &img_pixels
                    .par_iter()
                    .flat_map(|x| mapper.predict(palette, &x.0))
                    .collect::<Vec<u8>>(),
            )?,
            Threads::Extreme => {
                self.save(&dispatch_and_join(
                    subdivide(&img_pixels, *ThreadCount::calculate() as u8),
                    self.conf.palette,
                    &self.conf.mapper,
                    &self.prog,
                ))?;
            }
        }
        Ok(())
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

    fn save(&self, buf: &[u8]) -> Result<(), Box<dyn Error + 'static>> {
        let out = self.conf.output.unwrap_or("mapped.png".as_ref());
        let (w, h) = self.data.dimensions();
        image::save_buffer(out, buf, w, h, image::ColorType::Rgba8)?;
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

    pub fn output(&mut self, out: &'a Path) -> &mut Self {
        self.output = Some(out);
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
}
