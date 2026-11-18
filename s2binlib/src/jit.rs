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
use region::{Allocation, Protection};
use std::ptr;

pub struct JitTrampoline {
    ptr: *mut u8,
    size: usize,
    // Unmapped on drop.
    _memory: Allocation,
}

impl JitTrampoline {
    /// The stub lives in its own anonymous mapping rather than on the heap: SELinux denies making
    /// heap memory executable (`execheap`, denied by default policy with `selinuxuser_execheap`
    /// off), while an executable anonymous mapping is the same kind of memory safetyhook
    /// allocates for every inline hook.
    pub fn new(target_address: u64) -> Result<Self> {
        let code = Self::generate(target_address)?;
        let mut memory = region::alloc(code.len(), Protection::READ_WRITE_EXECUTE)?;
        let ptr = memory.as_mut_ptr::<u8>();
        unsafe { ptr::copy_nonoverlapping(code.as_ptr(), ptr, code.len()) };
        Ok(Self {
            ptr,
            size: memory.len(),
            _memory: memory,
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
    fn t_drop_and_reuse() -> Result<()> {
        for _ in 0..64 {
            let t = JitTrampoline::new(f42 as u64)?;
            let f: extern "C" fn() -> i32 = unsafe { t.as_fn_ptr() };
            assert_eq!(f(), 42);
        }
        Ok(())
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
