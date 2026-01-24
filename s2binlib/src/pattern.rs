/************************************************************************************
 *  S2BinLib - A static library that helps resolving memory from binary file
 *  and map to absolute memory address, targeting source 2 game engine.
 *  Copyright (C) 2025-2026  samyyc
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 ***********************************************************************************/

use anyhow::Result;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn find_pattern_simd(binary: &[u8], pattern: &[u8], wildcards: &[usize]) -> Result<u64> {
    if pattern.is_empty() || binary.len() < pattern.len() {
        return Err(anyhow::anyhow!("Pattern not found"));
    }

    let mut mask = vec![0xFFu8; pattern.len()];
    for &idx in wildcards {
        if idx < pattern.len() {
            mask[idx] = 0;
        }
    }

    let (first_idx, last_idx) = match get_bounds(pattern, &mask) {
        Some(bounds) => bounds,
        None => return find_scalar(binary, pattern, &mask),
    };

    #[cfg(target_arch = "x86_64")]
    unsafe {
        if is_x86_feature_detected!("avx512bw") {
            return find_avx512(binary, pattern, &mask, first_idx, last_idx);
        } else if is_x86_feature_detected!("avx2") {
            return find_avx2(binary, pattern, &mask, first_idx, last_idx);
        } else if is_x86_feature_detected!("sse2") {
            return find_sse2(binary, pattern, &mask, first_idx, last_idx);
        }
    }

    find_scalar(binary, pattern, &mask)
}

fn get_bounds(pattern: &[u8], mask: &[u8]) -> Option<(usize, usize)> {
    let first = (0..pattern.len()).find(|&i| mask[i] == 0xFF)?;
    let last = (0..pattern.len()).rev().find(|&i| mask[i] == 0xFF)?;
    Some((first, last))
}

#[inline(always)]
fn check_match(data: &[u8], pattern: &[u8], mask: &[u8]) -> bool {
    for i in 0..pattern.len() {
        if mask[i] != 0 && data[i] != pattern[i] {
            return false;
        }
    }
    true
}

fn find_scalar(binary: &[u8], pattern: &[u8], mask: &[u8]) -> Result<u64> {
    if pattern.is_empty() || binary.len() < pattern.len() {
        return Err(anyhow::anyhow!("Pattern not found"));
    }

    let end = binary.len() - pattern.len();
    for i in 0..=end {
        if check_match(&binary[i..], pattern, mask) {
            return Ok(i as u64);
        }
    }
    Err(anyhow::anyhow!("Pattern not found"))
}

#[target_feature(enable = "avx512bw")]
#[cfg(target_arch = "x86_64")]
unsafe fn find_avx512(
    bin: &[u8],
    pat: &[u8],
    mask: &[u8],
    f_idx: usize,
    l_idx: usize,
) -> Result<u64> {
    let f_char = _mm512_set1_epi8(pat[f_idx] as i8);
    let l_char = _mm512_set1_epi8(pat[l_idx] as i8);
    let mut i = 0;
    let end_simd = bin.len().saturating_sub(pat.len()).saturating_sub(63);

    while i <= end_simd {
        let f_chunk = unsafe { _mm512_loadu_si512(bin.as_ptr().add(i + f_idx) as *const _) };
        let l_chunk = unsafe { _mm512_loadu_si512(bin.as_ptr().add(i + l_idx) as *const _) };

        let mut bits =
            _mm512_cmpeq_epi8_mask(f_chunk, f_char) & _mm512_cmpeq_epi8_mask(l_chunk, l_char);

        while bits != 0 {
            let bit_pos = bits.trailing_zeros() as usize;
            if check_match(&bin[i + bit_pos..], pat, mask) {
                return Ok((i + bit_pos) as u64);
            }
            bits &= bits - 1;
        }
        i += 64;
    }
    find_scalar(&bin[i..], pat, mask).map(|off| off + i as u64)
}

#[target_feature(enable = "avx2")]
#[cfg(target_arch = "x86_64")]
unsafe fn find_avx2(
    bin: &[u8],
    pat: &[u8],
    mask: &[u8],
    f_idx: usize,
    l_idx: usize,
) -> Result<u64> {
    let f_char = _mm256_set1_epi8(pat[f_idx] as i8);
    let l_char = _mm256_set1_epi8(pat[l_idx] as i8);
    let mut i = 0;
    let end_simd = bin.len().saturating_sub(pat.len()).saturating_sub(31);

    while i <= end_simd {
        let f_chunk = unsafe { _mm256_loadu_si256(bin.as_ptr().add(i + f_idx) as *const _) };
        let l_chunk = unsafe { _mm256_loadu_si256(bin.as_ptr().add(i + l_idx) as *const _) };

        let match_mask = _mm256_and_si256(
            _mm256_cmpeq_epi8(f_chunk, f_char),
            _mm256_cmpeq_epi8(l_chunk, l_char),
        );

        let mut bits = _mm256_movemask_epi8(match_mask) as u32;
        while bits != 0 {
            let bit_pos = bits.trailing_zeros() as usize;
            if check_match(&bin[i + bit_pos..], pat, mask) {
                return Ok((i + bit_pos) as u64);
            }
            bits &= bits - 1;
        }
        i += 32;
    }
    find_scalar(&bin[i..], pat, mask).map(|off| off + i as u64)
}

#[target_feature(enable = "sse2")]
#[cfg(target_arch = "x86_64")]
unsafe fn find_sse2(
    bin: &[u8],
    pat: &[u8],
    mask: &[u8],
    f_idx: usize,
    l_idx: usize,
) -> Result<u64> {
    let f_char = _mm_set1_epi8(pat[f_idx] as i8);
    let l_char = _mm_set1_epi8(pat[l_idx] as i8);
    let mut i = 0;
    let end_simd = bin.len().saturating_sub(pat.len()).saturating_sub(15);

    while i <= end_simd {
        let f_chunk = unsafe { _mm_loadu_si128(bin.as_ptr().add(i + f_idx) as *const _) };
        let l_chunk = unsafe { _mm_loadu_si128(bin.as_ptr().add(i + l_idx) as *const _) };

        let match_mask = _mm_and_si128(
            _mm_cmpeq_epi8(f_chunk, f_char),
            _mm_cmpeq_epi8(l_chunk, l_char),
        );

        let mut bits = _mm_movemask_epi8(match_mask) as u32;
        while bits != 0 {
            let bit_pos = bits.trailing_zeros() as usize;
            if check_match(&bin[i + bit_pos..], pat, mask) {
                return Ok((i + bit_pos) as u64);
            }
            bits &= bits - 1;
        }
        i += 16;
    }
    find_scalar(&bin[i..], pat, mask).map(|off| off + i as u64)
}
