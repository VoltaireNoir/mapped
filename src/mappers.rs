use super::{
    palette::{ColorClass, Rgbx},
    Mapper,
};
use ahash::AHashMap;

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
        let grp = KNN::classify(pixel, self.k, &super::palette::SYN_DATA_SET, true, false);
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
