use super::{palette::Rgbx, Mapper};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Memoized<M: Mapper> {
    mapper: M,
    mem: Arc<DashMap<[u8; 4], [u8; 4], ahash::RandomState>>,
}

impl<M: Mapper> Memoized<M> {
    pub fn new(mapper: M) -> Self {
        Memoized {
            mapper,
            mem: Arc::new(DashMap::with_capacity_and_hasher(
                1000,
                ahash::RandomState::default(),
            )),
        }
    }
}

impl<M: Mapper> Mapper for Memoized<M> {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        if let Some(v) = self.mem.get(pixel) {
            *v
        } else {
            let pred = self.mapper.predict(palette, pixel);
            self.mem.insert(*pixel, pred);
            pred
        }
    }
}

impl<M: Mapper> From<M> for Memoized<M> {
    fn from(value: M) -> Self {
        Memoized::new(value)
    }
}
