//! SIMD-accelerated operations for quantized NNUE inference.
//!
//! Provides vectorized implementations for:
//! - i16 vector add/sub (accumulator updates)
//! - SCReLU dot product (output layer computation)
//!
//! Automatically selects the best available instruction set:
//! - x86_64: AVX2 (runtime detected), scalar fallback
//! - aarch64: NEON (always available on AArch64)
//! - Other: scalar fallback

use super::QA;

// ---------------------------------------------------------------------------
// Public dispatch functions
// ---------------------------------------------------------------------------

/// Elementwise `dst[i] += src[i]` for i16 slices.
#[inline]
pub fn vec_add_i16(dst: &mut [i16], src: &[i16]) {
    debug_assert_eq!(dst.len(), src.len());

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { avx2::vec_add(dst, src) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { neon::vec_add(dst, src) };
    }

    #[allow(unreachable_code)]
    scalar::vec_add(dst, src);
}

/// Elementwise `dst[i] -= src[i]` for i16 slices.
#[inline]
#[allow(dead_code)] // prepared for future incremental accumulator updates
pub fn vec_sub_i16(dst: &mut [i16], src: &[i16]) {
    debug_assert_eq!(dst.len(), src.len());

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { avx2::vec_sub(dst, src) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { neon::vec_sub(dst, src) };
    }

    #[allow(unreachable_code)]
    scalar::vec_sub(dst, src);
}

/// SCReLU dot product: `Σ clamp(acc[i], 0, QA)² × weights[i]`.
///
/// The result is in quantized units at scale QA² × QB. The caller divides by
/// `QA² × QB` (and applies `EVAL_SCALE`) to get centipawns.
#[inline]
pub fn screlu_dot(acc: &[i16], weights: &[i16]) -> i32 {
    debug_assert_eq!(acc.len(), weights.len());

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { avx2::screlu_dot(acc, weights) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { neon::screlu_dot(acc, weights) };
    }

    #[allow(unreachable_code)]
    scalar::screlu_dot(acc, weights)
}

// ---------------------------------------------------------------------------
// Scalar fallback
// ---------------------------------------------------------------------------

mod scalar {
    use super::QA;

    pub fn vec_add(dst: &mut [i16], src: &[i16]) {
        for (d, &s) in dst.iter_mut().zip(src) {
            *d = d.wrapping_add(s);
        }
    }

    #[allow(dead_code)]
    pub fn vec_sub(dst: &mut [i16], src: &[i16]) {
        for (d, &s) in dst.iter_mut().zip(src) {
            *d = d.wrapping_sub(s);
        }
    }

    pub fn screlu_dot(acc: &[i16], weights: &[i16]) -> i32 {
        let mut sum: i32 = 0;
        for (&a, &w) in acc.iter().zip(weights) {
            let clamped = a.clamp(0, QA) as i32;
            sum += clamped * clamped * w as i32;
        }
        sum
    }
}

// ---------------------------------------------------------------------------
// x86_64 — AVX2 (256-bit: 16 × i16 per register)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;

    use super::QA;

    #[target_feature(enable = "avx2")]
    pub unsafe fn vec_add(dst: &mut [i16], src: &[i16]) {
        let len = dst.len();
        let mut i = 0;
        while i + 16 <= len {
            let a = _mm256_loadu_si256(dst.as_ptr().add(i).cast());
            let b = _mm256_loadu_si256(src.as_ptr().add(i).cast());
            _mm256_storeu_si256(dst.as_mut_ptr().add(i).cast(), _mm256_add_epi16(a, b));
            i += 16;
        }
        while i < len {
            *dst.get_unchecked_mut(i) = dst.get_unchecked(i).wrapping_add(*src.get_unchecked(i));
            i += 1;
        }
    }

    #[allow(dead_code)]
    #[target_feature(enable = "avx2")]
    pub unsafe fn vec_sub(dst: &mut [i16], src: &[i16]) {
        let len = dst.len();
        let mut i = 0;
        while i + 16 <= len {
            let a = _mm256_loadu_si256(dst.as_ptr().add(i).cast());
            let b = _mm256_loadu_si256(src.as_ptr().add(i).cast());
            _mm256_storeu_si256(dst.as_mut_ptr().add(i).cast(), _mm256_sub_epi16(a, b));
            i += 16;
        }
        while i < len {
            *dst.get_unchecked_mut(i) = dst.get_unchecked(i).wrapping_sub(*src.get_unchecked(i));
            i += 1;
        }
    }

    /// SCReLU dot product using i32 widening (safe for arbitrary weight magnitudes).
    ///
    /// For each group of 16 i16 values, we widen clamped accumulators and weights
    /// to i32, compute `clamped² × weight`, and accumulate.
    #[target_feature(enable = "avx2")]
    pub unsafe fn screlu_dot(acc: &[i16], weights: &[i16]) -> i32 {
        let len = acc.len();
        let mut sum = _mm256_setzero_si256();
        let zero = _mm256_setzero_si256();
        let qa = _mm256_set1_epi16(QA);

        let mut i = 0;
        while i + 16 <= len {
            let a = _mm256_loadu_si256(acc.as_ptr().add(i).cast());
            let w = _mm256_loadu_si256(weights.as_ptr().add(i).cast());

            let clamped = _mm256_min_epi16(_mm256_max_epi16(a, zero), qa);

            // Low 8 values: sign/zero-extend i16 → i32
            let clamp_lo = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(clamped));
            let w_lo = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(w));
            let sq_lo = _mm256_mullo_epi32(clamp_lo, clamp_lo);
            sum = _mm256_add_epi32(sum, _mm256_mullo_epi32(sq_lo, w_lo));

            // High 8 values
            let clamp_hi = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(clamped, 1));
            let w_hi = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(w, 1));
            let sq_hi = _mm256_mullo_epi32(clamp_hi, clamp_hi);
            sum = _mm256_add_epi32(sum, _mm256_mullo_epi32(sq_hi, w_hi));

            i += 16;
        }

        let mut result = hsum_epi32(sum);

        while i < len {
            let c = (*acc.get_unchecked(i)).clamp(0, QA) as i32;
            result += c * c * *weights.get_unchecked(i) as i32;
            i += 1;
        }
        result
    }

    #[target_feature(enable = "avx2")]
    unsafe fn hsum_epi32(v: __m256i) -> i32 {
        let hi128 = _mm256_extracti128_si256(v, 1);
        let lo128 = _mm256_castsi256_si128(v);
        let sum128 = _mm_add_epi32(lo128, hi128);
        let hi64 = _mm_unpackhi_epi64(sum128, sum128);
        let sum64 = _mm_add_epi32(sum128, hi64);
        let hi32 = _mm_shuffle_epi32(sum64, 0b_00_00_00_01);
        let sum32 = _mm_add_epi32(sum64, hi32);
        _mm_cvtsi128_si32(sum32)
    }
}

// ---------------------------------------------------------------------------
// aarch64 — NEON (128-bit: 8 × i16 per register)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "aarch64")]
mod neon {
    use std::arch::aarch64::*;

    use super::QA;

    pub unsafe fn vec_add(dst: &mut [i16], src: &[i16]) {
        let len = dst.len();
        let mut i = 0;
        while i + 8 <= len {
            unsafe {
                let a = vld1q_s16(dst.as_ptr().add(i));
                let b = vld1q_s16(src.as_ptr().add(i));
                vst1q_s16(dst.as_mut_ptr().add(i), vaddq_s16(a, b));
            }
            i += 8;
        }
        while i < len {
            unsafe {
                *dst.get_unchecked_mut(i) =
                    dst.get_unchecked(i).wrapping_add(*src.get_unchecked(i));
            }
            i += 1;
        }
    }

    #[allow(dead_code)]
    pub unsafe fn vec_sub(dst: &mut [i16], src: &[i16]) {
        let len = dst.len();
        let mut i = 0;
        while i + 8 <= len {
            unsafe {
                let a = vld1q_s16(dst.as_ptr().add(i));
                let b = vld1q_s16(src.as_ptr().add(i));
                vst1q_s16(dst.as_mut_ptr().add(i), vsubq_s16(a, b));
            }
            i += 8;
        }
        while i < len {
            unsafe {
                *dst.get_unchecked_mut(i) =
                    dst.get_unchecked(i).wrapping_sub(*src.get_unchecked(i));
            }
            i += 1;
        }
    }

    /// SCReLU dot product using i32 widening via NEON.
    ///
    /// Uses `vmovl_s16` to widen i16 → i32, then `vmulq_s32` + `vmlaq_s32`
    /// for overflow-safe `clamped² × weight` computation.
    pub unsafe fn screlu_dot(acc: &[i16], weights: &[i16]) -> i32 {
        unsafe {
            let len = acc.len();
            let mut sum = vdupq_n_s32(0);
            let zero = vdupq_n_s16(0);
            let qa = vdupq_n_s16(QA);

            let mut i = 0;
            while i + 8 <= len {
                let a = vld1q_s16(acc.as_ptr().add(i));
                let w = vld1q_s16(weights.as_ptr().add(i));
                let clamped = vminq_s16(vmaxq_s16(a, zero), qa);

                let clamp_lo = vmovl_s16(vget_low_s16(clamped));
                let w_lo = vmovl_s16(vget_low_s16(w));
                let sq_lo = vmulq_s32(clamp_lo, clamp_lo);
                sum = vmlaq_s32(sum, sq_lo, w_lo);

                let clamp_hi = vmovl_s16(vget_high_s16(clamped));
                let w_hi = vmovl_s16(vget_high_s16(w));
                let sq_hi = vmulq_s32(clamp_hi, clamp_hi);
                sum = vmlaq_s32(sum, sq_hi, w_hi);

                i += 8;
            }

            let mut result = vaddvq_s32(sum);

            while i < len {
                let c = (*acc.get_unchecked(i)).clamp(0, QA) as i32;
                result += c * c * *weights.get_unchecked(i) as i32;
                i += 1;
            }
            result
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_add_sub() {
        let mut dst = vec![1i16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
        let src = vec![
            10i16, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170,
        ];

        vec_add_i16(&mut dst, &src);
        assert_eq!(
            dst,
            vec![
                11, 22, 33, 44, 55, 66, 77, 88, 99, 110, 121, 132, 143, 154, 165, 176, 187
            ]
        );

        vec_sub_i16(&mut dst, &src);
        assert_eq!(
            dst,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
        );
    }

    #[test]
    fn test_screlu_dot_basic() {
        let acc = vec![0i16; 8];
        let weights = vec![1i16; 8];
        assert_eq!(screlu_dot(&acc, &weights), 0);
    }

    #[test]
    fn test_screlu_dot_clamping() {
        let qa = QA;
        let acc = vec![-10i16, 0, 100, qa, qa + 50, qa, 0, 50];
        let weights = vec![1i16; 8];

        let expected: i32 = acc
            .iter()
            .zip(&weights)
            .map(|(&a, &w)| {
                let c = a.clamp(0, qa) as i32;
                c * c * w as i32
            })
            .sum();

        assert_eq!(screlu_dot(&acc, &weights), expected);
    }

    #[test]
    fn test_screlu_dot_negative_weights() {
        let acc = vec![QA; 16];
        let weights = vec![-10i16; 16];
        let expected = 16 * (QA as i32) * (QA as i32) * (-10i32);
        assert_eq!(screlu_dot(&acc, &weights), expected);
    }

    #[test]
    fn test_screlu_dot_matches_scalar() {
        // Realistic ranges: acc ∈ [-380, 385], weights ∈ [-100, 100] (QB=64 scale)
        let acc: Vec<i16> = (0..256).map(|i| (i * 3 - 380) as i16).collect();
        let weights: Vec<i16> = (0..256).map(|i| (i % 201 - 100) as i16).collect();

        let expected = scalar::screlu_dot(&acc, &weights);
        assert_eq!(screlu_dot(&acc, &weights), expected);
    }
}
