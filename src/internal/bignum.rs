//! Minimal big-integer modular exponentiation — just enough for
//! RSA-2048 PKCS#1 v1.5 signing (`m^d mod n`). Non-negative integers as
//! little-endian `u32` limb vectors, normalised (no trailing zero limbs).
//!
//! From-scratch (no `num-bigint`) per the rust-track strategy lock. Not
//! constant-time — this signs with a bundled TEST key only, never
//! production secrets.

use std::cmp::Ordering;

/// A non-negative big integer (little-endian `u32` limbs, normalised).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big {
    limbs: Vec<u32>,
}

impl Big {
    fn zero() -> Big {
        Big { limbs: Vec::new() }
    }

    fn normalize(mut self) -> Big {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
        self
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Construct from big-endian bytes.
    pub(crate) fn from_be_bytes(bytes: &[u8]) -> Big {
        let mut limbs = Vec::with_capacity(bytes.len() / 4 + 1);
        // Pack 4 bytes per limb, least-significant group first.
        let mut i = bytes.len();
        while i > 0 {
            let start = i.saturating_sub(4);
            let mut limb = 0u32;
            for &b in &bytes[start..i] {
                limb = (limb << 8) | b as u32;
            }
            limbs.push(limb);
            i = start;
        }
        Big { limbs }.normalize()
    }

    /// Serialise as big-endian bytes, left-padded / truncated to `len`.
    pub(crate) fn to_be_bytes(&self, len: usize) -> Vec<u8> {
        let mut out = vec![0u8; len];
        for (li, &limb) in self.limbs.iter().enumerate() {
            for byte in 0..4 {
                let pos = li * 4 + byte;
                if pos < len {
                    out[len - 1 - pos] = ((limb >> (8 * byte)) & 0xff) as u8;
                }
            }
        }
        out
    }

    fn bit_len(&self) -> usize {
        match self.limbs.last() {
            None => 0,
            Some(&top) => (self.limbs.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
        }
    }

    fn cmp(&self, other: &Big) -> Ordering {
        if self.limbs.len() != other.limbs.len() {
            return self.limbs.len().cmp(&other.limbs.len());
        }
        for i in (0..self.limbs.len()).rev() {
            if self.limbs[i] != other.limbs[i] {
                return self.limbs[i].cmp(&other.limbs[i]);
            }
        }
        Ordering::Equal
    }

    /// Shift left by `bits`.
    fn shl(&self, bits: usize) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        let limb_shift = bits / 32;
        let bit_shift = bits % 32;
        let mut out = vec![0u32; self.limbs.len() + limb_shift + 1];
        for (i, &limb) in self.limbs.iter().enumerate() {
            let v = (limb as u64) << bit_shift;
            out[i + limb_shift] |= v as u32;
            out[i + limb_shift + 1] |= (v >> 32) as u32;
        }
        Big { limbs: out }.normalize()
    }

    /// `self -= other`, assuming `self >= other`.
    fn sub_assign(&mut self, other: &Big) {
        let mut borrow = 0i64;
        for i in 0..self.limbs.len() {
            let o = if i < other.limbs.len() { other.limbs[i] as i64 } else { 0 };
            let mut diff = self.limbs[i] as i64 - o - borrow;
            if diff < 0 {
                diff += 1 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            self.limbs[i] = diff as u32;
        }
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    /// Schoolbook multiply.
    fn mul(&self, other: &Big) -> Big {
        if self.is_zero() || other.is_zero() {
            return Big::zero();
        }
        let mut out = vec![0u32; self.limbs.len() + other.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.limbs.iter().enumerate() {
                let cur = out[i + j] as u64 + (a as u64) * (b as u64) + carry;
                out[i + j] = cur as u32;
                carry = cur >> 32;
            }
            out[i + other.limbs.len()] += carry as u32;
        }
        Big { limbs: out }.normalize()
    }

    /// `self mod modulus` via binary long division.
    fn rem(&self, modulus: &Big) -> Big {
        if self.cmp(modulus) == Ordering::Less {
            return self.clone();
        }
        let mut r = self.clone();
        let mbits = modulus.bit_len();
        loop {
            let rbits = r.bit_len();
            if rbits < mbits {
                break;
            }
            let mut shift = rbits - mbits;
            let mut shifted = modulus.shl(shift);
            if shifted.cmp(&r) == Ordering::Greater {
                if shift == 0 {
                    break;
                }
                shift -= 1;
                shifted = modulus.shl(shift);
            }
            r.sub_assign(&shifted);
        }
        r
    }
}

/// Compute `base^exp mod modulus` (square-and-multiply, MSB-first over
/// the big-endian exponent bytes).
pub(crate) fn modpow(base: &Big, exp_be: &[u8], modulus: &Big) -> Big {
    let mut result = Big { limbs: vec![1] };
    let base = base.rem(modulus);
    for &byte in exp_be {
        for bit in (0..8).rev() {
            result = result.mul(&result).rem(modulus);
            if (byte >> bit) & 1 == 1 {
                result = result.mul(&base).rem(modulus);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bytes() {
        let b = Big::from_be_bytes(&[0x01, 0x23, 0x45, 0x67, 0x89]);
        assert_eq!(b.to_be_bytes(5), vec![0x01, 0x23, 0x45, 0x67, 0x89]);
    }

    #[test]
    fn small_modpow() {
        // 4^13 mod 497 = 445 (classic RSA worked example).
        // 497 = 0x01F1, 445 = 0x01BD.
        let base = Big::from_be_bytes(&[4]);
        let modn = Big::from_be_bytes(&[0x01, 0xF1]);
        let r = modpow(&base, &[13], &modn);
        assert_eq!(r.to_be_bytes(2), vec![0x01, 0xBD]);
    }

    #[test]
    fn modpow_identity() {
        // 7^1 mod 100 = 7.
        let r = modpow(&Big::from_be_bytes(&[7]), &[1], &Big::from_be_bytes(&[100]));
        assert_eq!(r.to_be_bytes(1), vec![7]);
    }
}
