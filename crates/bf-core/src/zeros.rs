//! Run-length coding for zero runs, and nothing else.
//!
//! Stripped transactions keep their proofs at full length, as zeros, because
//! the transaction parser insists on canonical proof sizes. Those zeros are
//! most of a shielded transaction's bytes; everything else is ciphertext,
//! keys and commitments, which do not compress. So this codec only collapses
//! long zero runs and copies everything else.
//!
//! Format: repeated `literal length (LEB128) | literal bytes | zero run length
//! (LEB128)` until the input ends.

use crate::varint;

/// Zero runs shorter than this stay inside literals.
const MIN_RUN: usize = 16;

pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() / 2 + 16);
    let mut lit_start = 0;
    let mut i = 0;
    while i < input.len() {
        if input[i] != 0 {
            i += 1;
            continue;
        }
        let run_start = i;
        while i < input.len() && input[i] == 0 {
            i += 1;
        }
        if i - run_start >= MIN_RUN || i == input.len() {
            varint::write(&mut out, (run_start - lit_start) as u64);
            out.extend_from_slice(&input[lit_start..run_start]);
            varint::write(&mut out, (i - run_start) as u64);
            lit_start = i;
        }
    }
    if lit_start < input.len() {
        varint::write(&mut out, (input.len() - lit_start) as u64);
        out.extend_from_slice(&input[lit_start..]);
        varint::write(&mut out, 0);
    }
    out
}

/// Refuses to expand beyond `max_len`, so a hostile row cannot make a client
/// allocate without bound.
pub fn decode(input: &[u8], max_len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < input.len() {
        let lit = varint::read(input, &mut pos)? as usize;
        let end = pos.checked_add(lit)?;
        out.extend_from_slice(input.get(pos..end)?);
        pos = end;
        let zeros = varint::read(input, &mut pos)? as usize;
        if out.len().checked_add(zeros)? > max_len {
            return None;
        }
        out.resize(out.len() + zeros, 0);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(v: &[u8]) {
        let enc = encode(v);
        assert_eq!(decode(&enc, 1 << 20).unwrap(), v, "input {:?}", &v[..v.len().min(40)]);
    }

    #[test]
    fn shapes() {
        round_trip(&[]);
        round_trip(&[0]);
        round_trip(&[7]);
        round_trip(&[0; 15]);
        round_trip(&[0; 5000]);
        round_trip(&[1, 0, 0, 2]);
        let mut tx = vec![5u8; 300];
        tx.extend(std::iter::repeat_n(0, 7264));
        tx.extend([9u8; 150]);
        tx.extend([0u8; 3]);
        round_trip(&tx);
        assert!(encode(&tx).len() < 300 + 150 + 3 + 16);
    }

    #[test]
    fn pseudo_random() {
        let mut x = 0x2545_f491_4f6c_dd1du64;
        for len in [1usize, 17, 100, 4097] {
            let v: Vec<u8> = (0..len)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    if x % 3 == 0 { 0 } else { x as u8 }
                })
                .collect();
            round_trip(&v);
        }
    }

    #[test]
    fn bounded() {
        let enc = encode(&[0; 10_000]);
        assert!(decode(&enc, 9_999).is_none());
        assert!(decode(&[0x05, 1, 2], 100).is_none());
    }
}
