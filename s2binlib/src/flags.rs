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

use object::SectionFlags;

pub fn is_readable(flags: SectionFlags) -> bool {
    match flags {
        SectionFlags::Coff { characteristics } => (characteristics & 0x40000000) != 0,
        SectionFlags::Elf { sh_flags } => (sh_flags & 0x2) != 0,
        SectionFlags::MachO { flags: _ } => true,
        SectionFlags::Xcoff { s_flags: _ } => true,
        SectionFlags::None => false,
        _ => false,
    }
}

pub fn is_writable(flags: SectionFlags) -> bool {
    match flags {
        SectionFlags::Coff { characteristics } => (characteristics & 0x80000000) != 0,
        SectionFlags::Elf { sh_flags } => (sh_flags & 0x1) != 0,
        SectionFlags::MachO { flags: _ } => false,
        SectionFlags::Xcoff { s_flags: _ } => false,
        SectionFlags::None => false,
        _ => false,
    }
}

pub fn is_executable(flags: SectionFlags) -> bool {
    match flags {
        SectionFlags::Coff { characteristics } => (characteristics & 0x20000000) != 0,
        SectionFlags::Elf { sh_flags } => (sh_flags & 0x4) != 0,
        SectionFlags::MachO { flags: _ } => false,
        SectionFlags::Xcoff { s_flags: _ } => false,
        SectionFlags::None => false,
        _ => false,
    }
}
