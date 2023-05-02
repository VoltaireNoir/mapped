pub(crate) fn subdivide<T>(pixels: &Vec<T>, times: usize) -> Vec<&[T]> {
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
