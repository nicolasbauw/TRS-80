pub fn serialize(input: &[u8]) -> Vec<u8> {
    let mut bits = Vec::new();
    for byte in input.iter() {
        for bit in 0..=7 {
            bits.push((byte >> bit) & 1)
        }
    }
    bits
}