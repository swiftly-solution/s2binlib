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
use object::{Object, ObjectSection};

use crate::is_executable;

#[cfg(target_os = "windows")]
mod win {
    use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, Module32NextW, TH32CS_SNAPMODULE,
        TH32CS_SNAPMODULE32,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;

    pub(super) fn module_from_pointer(ptr: u64) -> Option<(String, u64)> {
        unsafe {
            let snap = match CreateToolhelp32Snapshot(
                TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
                GetCurrentProcessId(),
            ) {
                Ok(h) => h,
                Err(_) => return None,
            };
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut entry = MODULEENTRY32W::default();
            entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;
            let mut hit = None;
            if Module32FirstW(snap, &mut entry).is_ok() {
                loop {
                    let base = entry.modBaseAddr as usize as u64;
                    let end = base + entry.modBaseSize as u64;
                    if ptr >= base && ptr < end {
                        let len = entry
                            .szModule
                            .iter()
                            .position(|c| *c == 0)
                            .unwrap_or(entry.szModule.len());
                        let name = String::from_utf16_lossy(&entry.szModule[..len]);
                        hit = Some((name, base));
                        break;
                    }
                    if Module32NextW(snap, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snap);
            hit
        }
    }
}

#[cfg(target_os = "linux")]
mod lin {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    pub(super) fn module_from_pointer(ptr: u64) -> Option<(String, u64)> {
        let file = File::open("/proc/self/maps").ok()?;
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            let mut parts = line.split_whitespace();
            let range = match parts.next() {
                Some(r) => r,
                None => continue,
            };
            let mut bounds = range.split('-');
            let start = match bounds.next().and_then(|v| u64::from_str_radix(v, 16).ok()) {
                Some(v) => v,
                None => continue,
            };
            let end = match bounds.next().and_then(|v| u64::from_str_radix(v, 16).ok()) {
                Some(v) => v,
                None => continue,
            };
            if ptr < start || ptr >= end {
                continue;
            }
            let name = parts.nth(4).unwrap_or("").to_string();
            return Some((name, start));
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct MemorySectionDescriptor {
    pub index: usize,
    pub name: Option<String>,
    pub address: u64,
    pub offset: usize,
    pub size: usize,
    pub executable: bool,
}

pub fn module_sections_from_slice<'a>(
    data: &'a [u8],
    image_base: u64,
) -> Result<Vec<MemorySectionDescriptor>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let file = object::File::parse(data)?;
    let mut sections = Vec::new();

    for section in file.sections() {
        let size = usize::try_from(section.size()).unwrap_or(0);
        if size == 0 {
            continue;
        }

        let address = section.address();
        let relative = address.checked_sub(image_base).unwrap_or(address);
        let offset = match usize::try_from(relative) {
            Ok(offset) if offset < data.len() => offset,
            _ => continue,
        };

        let available = data.len().saturating_sub(offset);
        if available == 0 {
            continue;
        }

        let slice_size = size.min(available);
        if slice_size == 0 {
            continue;
        }

        sections.push(MemorySectionDescriptor {
            index: section.index().0 as usize,
            name: section.name().ok().map(|s| s.to_string()),
            address,
            offset,
            size: slice_size,
            executable: is_executable(section.flags()),
        });
    }

    if sections.is_empty() {
        sections.push(MemorySectionDescriptor {
            index: 0,
            name: None,
            address: image_base,
            offset: 0,
            size: data.len(),
            executable: true,
        });
    } else {
        sections.sort_by_key(|section| (section.address, section.index));
    }

    Ok(sections)
}

#[cfg(target_os = "windows")]
pub fn module_from_pointer(ptr: u64) -> Option<(String, u64)> {
    win::module_from_pointer(ptr)
}

#[cfg(target_os = "linux")]
pub fn module_from_pointer(ptr: u64) -> Option<(String, u64)> {
    lin::module_from_pointer(ptr)
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn module_from_pointer(_ptr: u64) -> Option<(String, u64)> {
    None
}

pub fn get_module_base_from_pointer(ptr: u64) -> u64 {
    module_from_pointer(ptr).map(|(_, base)| base).unwrap_or(0)
}

#[allow(dead_code)]
pub fn set_mem_access(ptr: u64, size: usize) -> Result<()> {
    unsafe {
        let addr = ptr as *const u8;
        region::protect(addr, size, region::Protection::READ_WRITE_EXECUTE)
            .map_err(|e| anyhow::anyhow!("Failed to change memory protection: {}", e))
    }
}
