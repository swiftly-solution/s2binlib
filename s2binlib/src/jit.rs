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
use iced_x86::{Code, Encoder, Instruction, Register};
use region::{Protection, protect_with_handle};
use std::{alloc, ptr};

pub struct JitTrampoline {
    ptr: *mut u8,
    size: usize,
    _handle: region::ProtectGuard,
}

impl JitTrampoline {
    pub fn new(target_address: u64) -> Result<Self> {
        let code = Self::generate(target_address)?;
        let size = code.len().max(region::page::size());
        let layout =
            unsafe { alloc::Layout::from_size_align_unchecked(size, region::page::size()) };
        let ptr = unsafe { alloc::alloc(layout) };
        if ptr.is_null() {
            anyhow::bail!("alloc failed");
        }
        unsafe { ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len()) };
        let handle = unsafe { protect_with_handle(ptr, size, Protection::READ_WRITE_EXECUTE)? };
        Ok(Self {
            ptr,
            size,
            _handle: handle,
        })
    }

    fn generate(target: u64) -> Result<Vec<u8>> {
        let mut enc = Encoder::new(64);
        for _ in 0..20 {
            enc.encode(&Instruction::with(Code::Nopw), 0)?;
        }
        enc.encode(
            &Instruction::with2(Code::Mov_r64_imm64, Register::RAX, target)?,
            0,
        )?;
        enc.encode(&Instruction::with1(Code::Jmp_rm64, Register::RAX)?, 0)?;
        Ok(enc.take_buffer().to_vec())
    }

    pub fn address(&self) -> u64 {
        self.ptr as u64
    }

    pub unsafe fn as_fn_ptr<F>(&self) -> F {
        unsafe { std::mem::transmute_copy(&self.ptr) }
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for JitTrampoline {
    fn drop(&mut self) {
        unsafe {
            let layout = alloc::Layout::from_size_align_unchecked(self.size, region::page::size());
            alloc::dealloc(self.ptr, layout);
        }
    }
}
unsafe impl Send for JitTrampoline {}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn f42() -> i32 {
        42
    }
    extern "C" fn add(a: i32, b: i32) -> i32 {
        a + b
    }
    extern "C" fn f12345() -> i32 {
        12345
    }

    #[test]
    fn t_basic() -> Result<()> {
        let t = JitTrampoline::new(f42 as u64)?;
        let f: extern "C" fn() -> i32 = unsafe { t.as_fn_ptr() };
        assert_eq!(f(), 42);
        Ok(())
    }

    #[test]
    fn t_args() -> Result<()> {
        let t = JitTrampoline::new(add as u64)?;
        let f: extern "C" fn(i32, i32) -> i32 = unsafe { t.as_fn_ptr() };
        assert_eq!(f(10, 32), 42);
        Ok(())
    }

    #[test]
    fn t_no_args() -> Result<()> {
        let t = JitTrampoline::new(f12345 as u64)?;
        let f: extern "C" fn() -> i32 = unsafe { t.as_fn_ptr() };
        assert_eq!(f(), 12345);
        Ok(())
    }

    #[test]
    fn t_addr() -> Result<()> {
        let t = JitTrampoline::new(f42 as u64)?;
        assert_ne!(t.address(), 0);
        assert_ne!(t.address(), f42 as u64);
        Ok(())
    }
}
