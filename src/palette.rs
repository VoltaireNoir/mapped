use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct Rgbx(u8, u8, u8, ColorClass);

impl Rgbx {
    pub fn new(red: u8, green: u8, blue: u8, class: ColorClass) -> Rgbx {
        Rgbx(red, green, blue, class)
    }

    fn step_towards_val(orig: u8, target: u8, step: u8) -> u8 {
        match orig.cmp(&target) {
            Ordering::Equal => orig,
            Ordering::Greater => {
                let x = orig.saturating_sub(step);
                if x < target {
                    target
                } else {
                    x
                }
            }
            Ordering::Less => {
                let x = orig.saturating_add(step);
                if x > target {
                    target
                } else {
                    x
                }
            }
        }
    }
    // The closer to 0 the value is, the closer the given color is to the target value
    pub fn diff_rating(&self, rgb_val: &[u8; 4]) -> i16 {
        (self.0 as i16 - rgb_val[0] as i16)
            + (self.1 as i16 - rgb_val[1] as i16)
            + (self.2 as i16 - rgb_val[2] as i16) / 3
    }

    pub fn manhattan_dist(&self, rgb_val: &[u8; 4]) -> u16 {
        (self.0 as u16).abs_diff(rgb_val[0] as u16)
            + (self.1 as u16).abs_diff(rgb_val[1] as u16)
            + (self.2 as u16).abs_diff(rgb_val[2] as u16)
    }

    pub fn euclidian_dist(&self, rgb_val: &[u8; 4]) -> f32 {
        ((self.0.abs_diff(rgb_val[0]) as f32).powi(2)
            + (self.1.abs_diff(rgb_val[1]) as f32).powi(2)
            + (self.2.abs_diff(rgb_val[2]) as f32).powi(2))
        .sqrt()
    }

    pub fn rgba_array(&self) -> [u8; 4] {
        let (r, g, b): (u8, u8, u8) = (self.0, self.1, self.2);
        [r, g, b, 255]
    }

    pub fn rgb_float_array(&self) -> [f32; 3] {
        let max: f32 = 255.;
        [
            self.0 as f32 / max,
            self.1 as f32 / max,
            self.2 as f32 / max,
        ]
    }

    pub fn group(&self) -> ColorClass {
        self.3
    }

    pub fn step_towards(&self, other: &Rgbx, step: u8) -> Rgbx {
        let r = Self::step_towards_val(self.0, other.0, step);
        let g = Self::step_towards_val(self.1, other.1, step);
        let b = Self::step_towards_val(self.2, other.2, step);
        Rgbx(r, g, b, other.group())
    }

    pub fn gradient(&self, end_point: &Rgbx, distance: u8) -> Vec<Rgbx> {
        let mut vals: Vec<Rgbx> = Vec::new();
        loop {
            if vals.is_empty() {
                vals.push(*self);
                continue;
            }
            let next = vals.last().unwrap().step_towards(end_point, distance);
            vals.push(next);
            if next == *end_point {
                break;
            }
        }
        vals
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorClass {
    Blues,
    Whites,
    Greys,
    Red,
    Purple,
    Green,
    Yellow,
    Orange,
}

impl ColorClass {
    pub fn weight(&self) -> usize {
        match self {
            Self::Blues => 0,
            Self::Whites => 0,
            Self::Greys => 0,
            Self::Red => 0,
            Self::Purple => 0,
            Self::Green => 0,
            Self::Yellow => 0,
            Self::Orange => 0,
        }
    }
}

use ColorClass::{Blues, Green, Greys, Orange, Purple, Red, Whites, Yellow};

pub fn generate_data() {
    // Start and end points for all classes
    use std::fmt::Write as FmtWrite;

    let red1: (Rgbx, Rgbx) = (Rgbx(153, 0, 0, Red), Rgbx(255, 0, 0, Red));
    let red2: (Rgbx, Rgbx) = (Rgbx(255, 0, 0, Red), Rgbx(255, 153, 153, Red));
    let red3: (Rgbx, Rgbx) = (Rgbx(255, 0, 127, Red), Rgbx(153, 0, 76, Red));
    let red4: (Rgbx, Rgbx) = (Rgbx(255, 0, 127, Red), Rgbx(255, 153, 204, Red));
    let blue1: (Rgbx, Rgbx) = (Rgbx(0, 0, 255, Blues), Rgbx(0, 0, 153, Blues));
    let blue2: (Rgbx, Rgbx) = (Rgbx(0, 0, 255, Blues), Rgbx(153, 153, 255, Blues));
    let blue3: (Rgbx, Rgbx) = (Rgbx(0, 128, 255, Blues), Rgbx(0, 76, 153, Blues));
    let blue4: (Rgbx, Rgbx) = (Rgbx(0, 128, 255, Blues), Rgbx(153, 204, 255, Blues));
    let blue5: (Rgbx, Rgbx) = (Rgbx(0, 255, 255, Blues), Rgbx(0, 153, 153, Blues));
    let blue6: (Rgbx, Rgbx) = (Rgbx(0, 255, 255, Blues), Rgbx(153, 255, 255, Blues));
    let purple1: (Rgbx, Rgbx) = (Rgbx(255, 0, 255, Purple), Rgbx(153, 0, 153, Purple));
    let purple2: (Rgbx, Rgbx) = (Rgbx(255, 0, 255, Purple), Rgbx(255, 153, 255, Purple));
    let green1: (Rgbx, Rgbx) = (Rgbx(0, 255, 0, Green), Rgbx(0, 153, 0, Green));
    let green2: (Rgbx, Rgbx) = (Rgbx(0, 255, 0, Green), Rgbx(153, 255, 153, Green));
    let green3: (Rgbx, Rgbx) = (Rgbx(128, 255, 0, Green), Rgbx(76, 153, 0, Green));
    let green4: (Rgbx, Rgbx) = (Rgbx(128, 255, 0, Green), Rgbx(204, 255, 153, Green));
    let green5: (Rgbx, Rgbx) = (Rgbx(0, 255, 128, Green), Rgbx(0, 153, 76, Green));
    let green6: (Rgbx, Rgbx) = (Rgbx(0, 255, 128, Green), Rgbx(153, 255, 204, Green));
    let yellow1: (Rgbx, Rgbx) = (Rgbx(255, 255, 0, Yellow), Rgbx(153, 153, 0, Yellow));
    let yellow2: (Rgbx, Rgbx) = (Rgbx(255, 255, 0, Yellow), Rgbx(255, 255, 153, Yellow));
    let orange1: (Rgbx, Rgbx) = (Rgbx(255, 128, 0, Orange), Rgbx(153, 76, 0, Orange));
    let orange2: (Rgbx, Rgbx) = (Rgbx(255, 128, 0, Orange), Rgbx(255, 204, 153, Orange));
    let whites: (Rgbx, Rgbx) = (Rgbx(255, 255, 255, Whites), Rgbx(192, 192, 192, Whites));
    let whites2: (Rgbx, Rgbx) = (Rgbx(255, 255, 255, Whites), Rgbx(204, 229, 255, Whites));
    let whites3: (Rgbx, Rgbx) = (Rgbx(255, 255, 255, Whites), Rgbx(229, 255, 204, Whites));
    let whites4: (Rgbx, Rgbx) = (Rgbx(255, 255, 255, Whites), Rgbx(255, 204, 204, Whites));
    let blacks: (Rgbx, Rgbx) = (Rgbx(0, 0, 0, Greys), Rgbx(128, 128, 128, Greys));
    let gradients: Vec<Rgbx> = [
        red1, red2, red3, red4, blue1, blue2, blue3, blue4, blue5, blue6, purple1, purple2, green1,
        green2, green3, green4, green5, green6, yellow1, yellow2, orange1, orange2, whites,
        whites2, whites3, whites4, blacks,
    ]
    .into_iter()
    .flat_map(|(start, end)| start.gradient(&end, 5))
    .collect();
    let mut data = String::new();
    write!(&mut data, "{:?}", gradients).expect("Failed to write to string");
    std::fs::write("src/generated_data", data).expect("Failed to write data to file");
}

pub const NORD: [Rgbx; 16] = [
    Rgbx(216, 222, 233, Whites),
    Rgbx(229, 233, 240, Whites),
    Rgbx(236, 239, 244, Whites),
    Rgbx(143, 188, 187, Blues),
    Rgbx(136, 192, 208, Blues),
    Rgbx(129, 161, 193, Blues),
    Rgbx(94, 129, 172, Blues),
    Rgbx(191, 97, 106, Red),
    Rgbx(208, 135, 112, Orange),
    Rgbx(235, 203, 139, Yellow),
    Rgbx(163, 190, 140, Green),
    Rgbx(180, 142, 173, Purple),
    Rgbx(46, 52, 64, Greys),
    Rgbx(59, 66, 82, Greys),
    Rgbx(67, 76, 94, Greys),
    Rgbx(76, 86, 106, Greys),
];

pub const SYN_DATA_SET: [Rgbx; 671] = include!("generated_data");

pub const DATA_SET: [Rgbx; 112] = [
    Rgbx(255, 255, 255, Whites),
    Rgbx(224, 224, 224, Whites),
    Rgbx(192, 192, 192, Whites),
    Rgbx(236, 239, 244, Whites),
    Rgbx(216, 222, 233, Whites),
    Rgbx(229, 233, 240, Whites),
    Rgbx(0, 0, 0, Greys),
    Rgbx(8, 9, 4, Greys),
    Rgbx(21, 20, 13, Greys),
    Rgbx(32, 32, 32, Greys),
    Rgbx(64, 64, 64, Greys),
    Rgbx(96, 96, 96, Greys),
    Rgbx(76, 86, 106, Greys),
    Rgbx(46, 52, 64, Greys),
    Rgbx(59, 66, 82, Greys),
    Rgbx(67, 76, 94, Greys),
    Rgbx(19, 22, 16, Greys),
    Rgbx(17, 3, 0, Greys),
    Rgbx(255, 0, 0, Red),
    Rgbx(255, 51, 51, Red),
    Rgbx(255, 102, 102, Red),
    Rgbx(255, 102, 102, Red),
    Rgbx(255, 153, 153, Red),
    Rgbx(204, 0, 0, Red),
    Rgbx(153, 0, 0, Red),
    Rgbx(102, 0, 0, Red),
    Rgbx(191, 97, 106, Red),
    Rgbx(128, 0, 0, Red),
    Rgbx(220, 20, 60, Red),
    Rgbx(178, 34, 34, Red),
    Rgbx(99, 17, 48, Red),
    Rgbx(73, 19, 51, Red),
    Rgbx(49, 16, 48, Red),
    Rgbx(255, 128, 0, Orange),
    Rgbx(255, 153, 51, Orange),
    Rgbx(255, 178, 102, Orange),
    Rgbx(204, 102, 0, Orange),
    Rgbx(153, 76, 0, Orange),
    Rgbx(102, 51, 0, Orange),
    Rgbx(208, 135, 112, Orange),
    Rgbx(232, 134, 61, Orange),
    Rgbx(224, 95, 11, Orange),
    Rgbx(255, 255, 0, Yellow),
    Rgbx(255, 255, 51, Yellow),
    Rgbx(255, 255, 102, Yellow),
    Rgbx(255, 255, 153, Yellow),
    Rgbx(255, 255, 204, Yellow),
    Rgbx(255, 254, 114, Yellow),
    Rgbx(204, 204, 0, Yellow),
    Rgbx(153, 153, 0, Yellow),
    Rgbx(102, 102, 0, Yellow),
    Rgbx(51, 51, 0, Yellow),
    Rgbx(235, 203, 139, Yellow),
    Rgbx(255, 255, 204, Yellow),
    Rgbx(255, 204, 153, Yellow),
    Rgbx(0, 255, 0, Green),
    Rgbx(51, 255, 51, Green),
    Rgbx(102, 255, 102, Green),
    Rgbx(153, 255, 153, Green),
    Rgbx(204, 255, 204, Green),
    Rgbx(0, 204, 0, Green),
    Rgbx(0, 153, 0, Green),
    Rgbx(0, 102, 0, Green),
    Rgbx(128, 255, 0, Green),
    Rgbx(153, 255, 51, Green),
    Rgbx(178, 255, 102, Green),
    Rgbx(204, 255, 153, Green),
    Rgbx(229, 255, 204, Green),
    Rgbx(102, 204, 0, Green),
    Rgbx(76, 153, 0, Green),
    Rgbx(0, 255, 128, Green),
    Rgbx(51, 255, 153, Green),
    Rgbx(102, 255, 178, Green),
    Rgbx(0, 204, 102, Green),
    Rgbx(0, 153, 76, Green),
    Rgbx(255, 0, 255, Purple),
    Rgbx(127, 0, 255, Purple),
    Rgbx(153, 51, 255, Purple),
    Rgbx(178, 102, 255, Purple),
    Rgbx(204, 153, 255, Purple),
    Rgbx(102, 0, 204, Purple),
    Rgbx(76, 0, 153, Purple),
    Rgbx(255, 51, 255, Purple),
    Rgbx(255, 102, 255, Purple),
    Rgbx(255, 153, 255, Purple),
    Rgbx(204, 0, 204, Purple),
    Rgbx(153, 0, 153, Purple),
    Rgbx(255, 0, 127, Purple),
    Rgbx(255, 51, 153, Purple),
    Rgbx(204, 0, 102, Purple),
    Rgbx(180, 142, 173, Purple),
    Rgbx(0, 0, 255, Blues),
    Rgbx(51, 51, 255, Blues),
    Rgbx(102, 102, 255, Blues),
    Rgbx(153, 153, 255, Blues),
    Rgbx(204, 204, 255, Blues),
    Rgbx(0, 0, 204, Blues),
    Rgbx(0, 0, 153, Blues),
    Rgbx(0, 0, 102, Blues),
    Rgbx(0, 128, 255, Blues),
    Rgbx(0, 153, 153, Blues),
    Rgbx(0, 204, 204, Blues),
    Rgbx(51, 153, 255, Blues),
    Rgbx(102, 178, 255, Blues),
    Rgbx(153, 204, 255, Blues),
    Rgbx(204, 229, 255, Blues),
    Rgbx(0, 102, 204, Blues),
    Rgbx(0, 76, 153, Blues),
    Rgbx(0, 255, 255, Blues),
    Rgbx(51, 255, 255, Blues),
    Rgbx(102, 255, 255, Blues),
    Rgbx(153, 255, 255, Blues),
];

#[cfg(test)]
mod test {
    use super::*;
    use crate::KNN;
    const BASIC_COLORS: [Rgbx; 14] = [
        Rgbx(255, 0, 0, Red),
        Rgbx(255, 0, 127, Red),
        Rgbx(255, 128, 0, Orange),
        Rgbx(255, 255, 0, Yellow),
        Rgbx(128, 255, 0, Green),
        Rgbx(0, 255, 0, Green),
        Rgbx(0, 255, 128, Green),
        Rgbx(0, 255, 255, Blues),
        Rgbx(0, 128, 255, Blues),
        Rgbx(0, 0, 255, Blues),
        Rgbx(255, 0, 255, Purple),
        Rgbx(128, 128, 128, Greys),
        Rgbx(0, 0, 0, Greys),
        Rgbx(255, 255, 255, Whites),
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
            let grp = KNN::classify(&color.rgba_array(), k, data_set, true, false);
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
