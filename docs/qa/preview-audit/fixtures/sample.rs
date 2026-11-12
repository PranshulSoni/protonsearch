//! Preview audit code fixture.
// Unique token: PANDA-CODE-2026

pub fn preview_probe(input: &str) -> usize {
    let mut total = 0usize;
    for ch in input.chars() {
        if ch.is_alphabetic() { total += 1; }
    }
    total
}
