pub mod pallete;

use ahash::AHashMap;
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use pallete::{ColorClass, Rgbx};
use std::{
    error::Error,
    marker::PhantomData,
    ops::Deref,
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Instant,
};

use rayon::prelude::*;

pub struct Processor<'a, 'b, F, T = Unloaded>
where
    F: AsRef<str>,
{
    file: F,
    data: Option<DynamicImage>,
    mapper: Box<dyn Mapper>,
    output: Option<&'a Path>,
    palette: &'b [Rgbx],
    threads: Threads,
    marker: PhantomData<T>,
    progress: Option<Sender<Signal>>,
}

fn notify(sender: Option<&Sender<Signal>>) {
    if let Some(s) = sender {
        s.send(Signal).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct Loaded;

#[derive(Debug, Clone)]
pub struct Unloaded;

impl<'a, 'b, F, T> Processor<'a, 'b, F, T>
where
    F: AsRef<str>,
{
    pub fn strategy(mut self, mapper: Box<dyn Mapper>) -> Self {
        self.mapper = mapper;
        self
    }

    pub fn output(mut self, output: &'a Path) -> Self {
        self.output.replace(output);
        self
    }

    pub fn palette(mut self, palette: &'b [Rgbx]) -> Self {
        self.palette = palette;
        self
    }

    pub fn threads(mut self, threads: Threads) -> Self {
        self.threads = threads;
        self
    }
}

impl<'a, 'b, F> Processor<'a, 'b, F, Unloaded>
where
    F: AsRef<str>,
{
    pub fn new(file: F) -> Processor<'a, 'b, F> {
        Processor {
            file,
            data: None,
            mapper: Box::new(Nearest),
            output: None,
            palette: &pallete::NORD,
            threads: Threads::Auto,
            progress: None,
            marker: PhantomData,
        }
    }

    pub fn load(self) -> Result<Processor<'a, 'b, F, Loaded>, Box<dyn Error + 'static>> {
        let img = image::open(self.file.as_ref())?;

        Ok(Processor {
            file: self.file,
            data: Some(img),
            mapper: self.mapper,
            output: self.output,
            palette: self.palette,
            threads: self.threads,
            marker: PhantomData,
            progress: self.progress,
        })
    }

    pub fn set_file(&mut self, file: F) {
        self.file = file;
    }
}

impl<F> Processor<'_, '_, F, Loaded>
where
    F: AsRef<str>,
{
    pub fn process(&self) -> Result<(), Box<dyn Error + 'static>> {
        //TODO: write the rest of the function
        let img_pixels: Vec<_> = self.image().pixels().map(|(_, _, rgb)| rgb).collect();
        match self.threads {
            Threads::Single => (),
            Threads::Auto => (),
            Threads::Custom(_) => (),
            Threads::Rayon => self.save(
                &img_pixels
                    .par_iter()
                    .flat_map(|x| self.mapper.predict(self.palette, &x.0))
                    .collect::<Vec<u8>>(),
            )?,
            Threads::Extreme => {
                self.save(&dispatch_and_join2(
                    subdivide(&img_pixels, *ThreadCount::calculate()),
                    self.palette,
                    self.mapper.as_ref(),
                    self.progress.as_ref(),
                ))?;
            }
        }
        Ok(())
    }

    pub fn gen_tracker(&mut self) -> Tracker {
        let (s, r) = mpsc::channel::<Signal>();
        let (x, y) = self.image().dimensions();

        self.progress.replace(s);

        Tracker {
            current: 0,
            total: (x * y) as usize,
            receiver: r,
        }
    }

    fn image(&self) -> &DynamicImage {
        self.data.as_ref().unwrap()
    }

    fn save(&self, buf: &[u8]) -> Result<(), Box<dyn Error + 'static>> {
        let out = self.output.unwrap_or("mapped.png".as_ref());
        let (w, h) = self.image().dimensions();
        image::save_buffer(out, buf, w, h, image::ColorType::Rgba8)?;
        Ok(())
    }
}

pub struct Tracker {
    current: usize,
    total: usize,
    receiver: Receiver<Signal>,
}

struct Signal;

impl From<Signal> for usize {
    fn from(_: Signal) -> Self {
        1
    }
}

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
        self.current += self.receiver.try_iter().map(usize::from).sum::<usize>();
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

fn load_file(file: &str) -> DynamicImage {
    image::open(file).expect("Failed to read file.")
}

fn save_file(raw_data: Vec<u8>, (w, h): (u32, u32), output: Option<&str>, input: &str) {
    let newimg = RgbaImage::from_vec(w, h, raw_data).expect("failed to create new image");
    if let Some(output) = output {
        newimg.save(output).expect("Failed to save file");
    } else {
        let (name, _) = input.split_once('.').unwrap();
        let output = format!("{}_nordified.png", name);
        newimg.save(output).expect("Failed to save file");
    };
}

pub fn nordify(file: impl AsRef<str>, output: Option<&str>, palette: &[Rgbx], mapper: &dyn Mapper) {
    let img = load_file(file.as_ref());
    let img_pixels: Vec<_> = img.pixels().map(|(_, _, rgb)| rgb).collect();

    let n_parts: u8 = {
        let c = num_cpus::get();
        if c >= 4 {
            (c / 2) as u8
        } else {
            1
        }
    };

    save_file(
        dispatch_and_join(subdivide(&img_pixels, n_parts), palette, mapper),
        img.dimensions(),
        output,
        file.as_ref(),
    );
}

fn dispatch_and_join(parts: Vec<&[Rgba<u8>]>, palette: &[Rgbx], mapper: &dyn Mapper) -> Vec<u8> {
    thread::scope(|s| {
        let mut handles: Vec<thread::ScopedJoinHandle<Vec<u8>>> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        for part in parts {
            let h = s.spawn(|| {
                part.iter()
                    .flat_map(|rgb| mapper.predict(palette, &rgb.0))
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

fn dispatch_and_join2(
    parts: Vec<&[Rgba<u8>]>,
    palette: &[Rgbx],
    mapper: &dyn Mapper,
    progress: Option<&Sender<Signal>>,
) -> Vec<u8> {
    thread::scope(|s| {
        let mut handles: Vec<thread::ScopedJoinHandle<Vec<u8>>> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        for part in parts {
            let sender = progress.cloned();
            let h = s.spawn(move || {
                part.iter()
                    .flat_map(|rgb| {
                        let r = mapper.predict(palette, &rgb.0);
                        notify(sender.as_ref());
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

pub struct Nearest;

impl Mapper for Nearest {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        let pick = palette
            .iter()
            .enumerate()
            .map(|(i, pal)| (i, pal.euclidian_dist(pixel)))
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .unwrap();

        palette[pick.0].rgba_array()
    }
}

pub struct Creative;

impl Mapper for Creative {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        let distances = palette
            .iter()
            .enumerate()
            .map(|(i, target)| (i, target.diff_rating(pixel)));
        let pos = distances.clone().filter(|x| x.1 >= 0).min_by_key(|x| x.1);
        let neg = distances.filter(|x| x.1 <= 0).max_by_key(|x| x.1);

        match (pos, neg) {
            (None, None) => *pixel,
            (Some(pos), Some(neg)) => {
                let posneg = -neg.1;
                if posneg < pos.1 {
                    palette[neg.0].rgba_array()
                } else {
                    palette[pos.0].rgba_array()
                }
            }
            (Some(pos), None) => palette[pos.0].rgba_array(),
            (None, Some(neg)) => palette[neg.0].rgba_array(),
        }
    }
}

pub struct KNN {
    k: usize,
}

impl Default for KNN {
    fn default() -> Self {
        KNN { k: 12 }
    }
}

impl KNN {
    pub fn with(k: usize) -> Self {
        KNN { k }
    }
    fn classify(
        c: &[u8; 4],
        k: usize,
        dataset: &[Rgbx],
        random: bool,
        weighted: bool,
    ) -> ColorClass {
        let mut ratings: Vec<_> = dataset
            .iter()
            .map(|pal| (pal.euclidian_dist(c), pal.group()))
            .collect();
        ratings.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut vote_map = AHashMap::with_capacity(k);

        for (_, g) in ratings[..=k].iter() {
            vote_map
                .entry(g)
                .and_modify(|entry| *entry += 1)
                .or_insert(0);
        }
        let (grp, count) = if weighted {
            vote_map
                .iter()
                .map(|(k, v)| (k, v + k.weight()))
                .max_by_key(|x| x.1)
                .unwrap()
        } else {
            vote_map
                .iter()
                .map(|(k, v)| (k, *v))
                .max_by_key(|x| x.1)
                .unwrap()
        };

        if random {
            let mut candidates: Vec<ColorClass> = Vec::new();
            for (g, v) in vote_map.iter() {
                if v == &count && g != grp {
                    candidates.push(**g)
                }
            }
            let l = candidates.len();
            if l == 0 {
                **grp
            } else {
                candidates[fastrand::usize(..l)]
            }
        } else {
            **grp
        }
    }
}

impl Mapper for KNN {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        let grp = KNN::classify(pixel, self.k, &pallete::SYN_DATA_SET, true, false);
        let (i, _, _) = palette
            .iter()
            .enumerate()
            .map(|(i, pal)| (i, pal.euclidian_dist(pixel), pal.group()))
            .filter(|x| x.2 == grp)
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .unwrap();

        palette[i].rgba_array()
    }
}

// The closer the diff to 0 is, the more linearly distributed are the valued
// Linearly distributed values tend to fall into blacks and greys
// Higher diff indicates varied distribution, and may indicate that the color is closer to red green or blue
// TODO: Apply diff if the distance of two candidates falls within a certain threshhold
fn diff(color: Vec<u8>) -> i16 {
    let (x, y, z) = (color[0] as i16, color[1] as i16, color[2] as i16);
    (x - y) + (x - z)
}
