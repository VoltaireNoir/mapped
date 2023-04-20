use super::{palette::Rgbx, Mapper, SignalSender};
use image::Rgba;
use std::thread;

pub(crate) fn dispatch_and_join<M: Mapper>(
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

pub(crate) fn subdivide<T>(pixels: &Vec<T>, times: u8) -> Vec<&[T]> {
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

pub(crate) fn split_and_push<'a, T>(sl: &'a [T], vec: &mut Vec<&'a [T]>) {
    let mid = sl.len() / 2;
    let (left, right) = sl.split_at(mid);
    vec.push(left);
    vec.push(right);
}
