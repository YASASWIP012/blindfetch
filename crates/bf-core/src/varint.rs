//! Unsigned LEB128, the smallest encoding that keeps the index a few bytes
//! per transaction.

pub fn write(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Reads one value and advances `pos`. Returns `None` on truncated input or a
/// value wider than 64 bits.
pub fn read(buf: &[u8], pos: &mut usize) -> Option<u64> {
    let mut v: u64 = 0;
    for shift in (0..64).step_by(7) {
        let byte = *buf.get(*pos)?;
        *pos += 1;
        v |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        for v in [0, 1, 127, 128, 300, 16_383, 16_384, u32::MAX as u64, u64::MAX] {
            let mut buf = Vec::new();
            super::write(&mut buf, v);
            let mut pos = 0;
            assert_eq!(super::read(&buf, &mut pos), Some(v));
            assert_eq!(pos, buf.len());
        }
    }

    #[test]
    fn truncated() {
        let mut pos = 0;
        assert_eq!(super::read(&[0x80], &mut pos), None);
    }
}
