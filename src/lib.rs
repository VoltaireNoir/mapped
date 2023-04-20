pub mod mappers;
pub mod palette;

use image::{DynamicImage, GenericImageView, Rgba};
use mappers::Nearest;
use palette::Rgbx;

use std::{
    error::Error,
    ops::{Deref, DerefMut},
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use rayon::prelude::*;

pub struct Processor<'a, 'b, 'c, M>
where
    M: Mapper,
{
    conf: &'c ProcOptions<'a, 'b, M>,
    data: DynamicImage,
    prog: SignalSender,
}

impl<'a, 'b, 'c, M> Processor<'a, 'b, 'c, M>
where
    M: Mapper,
{
    pub fn process(&self) -> Result<(), Box<dyn Error + 'static>> {
        //TODO: write the rest of the function
        let img_pixels: Vec<_> = self.data.pixels().map(|(_, _, rgb)| rgb).collect();
        let mapper = &self.conf.mapper;
        match self.conf.threads {
            Threads::Single => self.save(
                &img_pixels
                    .iter()
                    .flat_map(|pixel| mapper.predict(self.conf.palette, &pixel.0))
                    .collect::<Vec<u8>>(),
            )?,
            Threads::Auto => todo!(),
            Threads::Custom(_) => todo!(),
            Threads::Rayon => self.save(
                &img_pixels
                    .par_iter()
                    .flat_map(|x| mapper.predict(self.conf.palette, &x.0))
                    .collect::<Vec<u8>>(),
            )?,
            Threads::Extreme => {
                self.save(&dispatch_and_join2(
                    subdivide(&img_pixels, *ThreadCount::calculate()),
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
pub struct ProcOptions<'a, 'b, M: Mapper> {
    mapper: M,
    output: Option<&'a Path>,
    threads: Threads,
    palette: &'b [Rgbx],
}

impl Default for ProcOptions<'_, '_, Nearest> {
    fn default() -> Self {
        ProcOptions {
            mapper: Nearest,
            output: None,
            threads: Threads::default(),
            palette: &crate::palette::NORD,
        }
    }
}

impl<'a, 'b, M: Mapper> ProcOptions<'a, 'b, M> {
    pub fn new(mapper: M) -> Self {
        ProcOptions {
            mapper,
            output: None,
            threads: Threads::default(),
            palette: &crate::palette::NORD,
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
    ) -> Result<Processor<'a, 'b, '_, M>, Box<dyn Error + 'static>> {
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
pub struct ThreadCount(u8);

impl ThreadCount {
    pub fn new(val: u8) -> Result<Self, u8> {
        if val % 2 == 0 && val != 0 {
            Ok(Self(val))
        } else {
            Err(val)
        }
    }

    fn calculate() -> Self {
        if let Ok(c) = std::thread::available_parallelism() {
            let c = usize::from(c);
            if c >= 4 {
                ThreadCount::new((c / 2) as u8).unwrap()
            } else {
                ThreadCount::default()
            }
        } else {
            ThreadCount::default()
        }
    }
}

impl Deref for ThreadCount {
    type Target = u8;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for ThreadCount {
    fn default() -> Self {
        Self(1)
    }
}

fn dispatch_and_join2<M: Mapper>(
    parts: Vec<&[Rgba<u8>]>,
    palette: &[Rgbx],
    mapper: &M,
    progress: &SignalSender,
) -> Vec<u8> {
    thread::scope(|s| {
        let mut handles: Vec<thread::ScopedJoinHandle<Vec<u8>>> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        for part in parts {
            let sender = progress.clone();
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

fn subdivide<T>(pixels: &Vec<T>, times: u8) -> Vec<&[T]> {
    let mut parts: Vec<&[T]> = Vec::new();
    parts.push(pixels);
    for _ in 0..times {
        let len = parts.len();
        for _ in 0..len {
            split_and_push(parts.remove(0), &mut parts)
        }
    }
    parts
}

fn split_and_push<'a, T>(sl: &'a [T], vec: &mut Vec<&'a [T]>) {
    let mid = sl.len() / 2;
    let (left, right) = sl.split_at(mid);
    vec.push(left);
    vec.push(right);
}

pub trait Mapper: Send + Sync {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4];
}
