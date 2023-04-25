use crate::palette;

use super::{
    palette::{ColorClass, Rgbx},
    Mapper,
};
use ahash::AHashMap;

#[derive(Debug, Clone)]
pub struct Nearest;

impl Mapper for Nearest {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        palette
            .iter()
            .min_by_key(|pal| pal.manhattan_dist(pixel))
            .unwrap()
            .rgba_array()
    }
}

#[derive(Debug, Clone)]
pub struct NearestDoublePass;

impl Mapper for NearestDoublePass {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        let basic = palette::find_closest(&palette::BASECOLORS, pixel);
        palette
            .iter()
            .min_by_key(|pc| pc.manhattan_dist(&basic))
            .unwrap()
            .rgba_array()
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Knn {
    k: usize,
}

impl Default for Knn {
    fn default() -> Self {
        Knn { k: 12 }
    }
}

impl Knn {
    pub fn with(k: usize) -> Self {
        Knn { k }
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

impl Mapper for Knn {
    fn predict(&self, palette: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        let grp = Knn::classify(pixel, self.k, &super::palette::SYN_DATA_SET, true, false);
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

#[derive(Debug, Clone)]
pub struct ManualMap;

impl Mapper for ManualMap {
    fn predict(&self, _: &[Rgbx], pixel: &[u8; 4]) -> [u8; 4] {
        match *pixel {
            [100..=255, 0, 0, _] => palette::NORD[8].rgba_array(),
            [185..=255, 0..=68, 0..=68, _] => palette::NORD[8].rgba_array(),
            _ => *pixel,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::palette::ColorClass::*;
    use crate::palette::*;
    use crate::rgbx;

    const BASIC_COLORS: [Rgbx; 14] = [
        rgbx!(255, 0, 0, r),
        rgbx!(255, 0, 127, r),
        rgbx!(255, 128, 0, o),
        rgbx!(255, 255, 0, y),
        rgbx!(128, 255, 0, g),
        rgbx!(0, 255, 0, g),
        rgbx!(0, 255, 128, g),
        rgbx!(0, 255, 255, b),
        rgbx!(0, 128, 255, b),
        rgbx!(0, 0, 255, b),
        rgbx!(255, 0, 255, p),
        rgbx!(128, 128, 128, g),
        rgbx!(0, 0, 0, g),
        rgbx!(255, 255, 255, w),
    ];

    #[test]
    fn basic_color_accuracy() {
        let acc = prediction_accuracy(&BASIC_COLORS, &SYN_DATA_SET, 30, true);
        println!("Basic color prediction accuracy: {}%", acc);
        assert!(acc > 95.0)
    }

    fn prediction_accuracy(sample: &[Rgbx], data_set: &[Rgbx], k: usize, print: bool) -> f32 {
        let mut matches = 0;
        for color in sample {
            let grp = Knn::classify(&color.rgba_array(), k, data_set, true, false);
            matches += if grp == color.3 {
                1
            } else {
                if print {
                    println!("Failed to predict: {:?}, prediction: {:?}", color, grp);
                }
                0
            };
        }
        (matches as f32 / sample.len() as f32) * 100.0
    }

    #[test]
    fn rgbx_equality() {
        let x = Rgbx(255, 255, 255, ColorClass::Whites);
        let y = x;
        assert_eq!(x, y)
    }

    #[test]
    fn rgbx_inequality() {
        let x = Rgbx(255, 255, 255, ColorClass::Whites);
        let y = Rgbx(255, 200, 0, ColorClass::Orange);
        assert_ne!(x, y)
    }

    #[test]
    fn gradient() {
        let start = Rgbx(255, 204, 204, Blues);
        let end = Rgbx(102, 0, 0, Blues);
        let _g = start.gradient(&end, 10);
    }
}
