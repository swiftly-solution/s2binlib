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

use anyhow::{Result, bail};
use hashbrown::HashMap;
use iced_x86::{Code, Decoder, DecoderOptions, Instruction, OpKind, Register};
use object::{Object, ObjectSection, ObjectSymbol, SectionKind, read::pe::ImageOptionalHeader};
use std::{
    cell::Cell,
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{
    VTableInfo, find_pattern_simd, is_executable,
    jit::JitTrampoline,
    memory::{get_module_base_from_pointer, module_from_pointer, set_mem_access},
};

#[cfg(target_os = "windows")]
use std::ffi::CString;
#[cfg(target_os = "windows")]
use std::os::raw::c_void;

#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn GetModuleHandleA(lpModuleName: *const u8) -> *mut c_void;
}

pub struct S2BinLib<'a> {
    pub(crate) game_path: PathBuf,
    pub(crate) game_type: String,
    pub(crate) os: String,
    pub(crate) binaries: HashMap<String, Vec<u8>>,
    pub(crate) manual_base_addresses: HashMap<String, u64>,
    /// Cached cross-references: binary_name -> (target_rva -> Vec<xref_rva>)
    pub(crate) xrefs_cache: HashMap<String, HashMap<u64, Vec<u64>>>,
    /// Trampolines: (mem_address -> JitTrampoline)
    pub(crate) trampolines: HashMap<u64, JitTrampoline>,
    pub(crate) custom_binary_paths_windows: HashMap<String, String>,
    pub(crate) custom_binary_paths_linux: HashMap<String, String>,
    pub(crate) vtables: HashMap<String, Vec<VTableInfo>>,
    pub(crate) name_to_vtables: HashMap<String, &'a VTableInfo>,
    /// Cached ASCII strings: binary_name -> (string -> string_rva)
    pub(crate) strings_cache: HashMap<String, HashMap<String, u64>>,
    pub(crate) calls_targets_cache: HashMap<String, Vec<u64>>,
}

fn read_int32(data: &[u8], offset: u64) -> u32 {
    let mut rvalue = 0;
    for i in 0..4 {
        rvalue |= (data[offset as usize + i as usize] as u32) << (i * 8);
    }
    rvalue
}

fn read_int64(data: &[u8], offset: u64) -> i64 {
    let mut rvalue = 0i64;
    for i in 0..8 {
        rvalue |= (data[offset as usize + i as usize] as i64) << (i * 8);
    }
    rvalue
}

fn write_strings_to_json<P: AsRef<Path>>(
    strings: &HashMap<String, u64>,
    output_path: P,
) -> Result<()> {
    let ordered_strings: BTreeMap<&str, u64> = strings
        .iter()
        .map(|(string, rva)| (string.as_str(), *rva))
        .collect();

    let mut output = BufWriter::new(File::create(output_path)?);
    serde_json::to_writer_pretty(&mut output, &ordered_strings)?;
    output.flush()?;

    Ok(())
}

impl<'a> S2BinLib<'a> {
    fn get_os_name(&self) -> String {
        match self.os.as_str() {
            "windows" => "win64".to_string(),
            _ => "linuxsteamrt64".to_string(),
        }
    }

    fn get_os_lib_name(&self, lib_name: &str) -> String {
        match self.os.as_str() {
            "windows" => format!("{}.dll", lib_name),
            _ => format!("lib{}.so", lib_name),
        }
    }

    pub fn get_os(&self) -> String {
        self.os.clone()
    }

    pub fn get_module_base_address(&self, lib_name: &str) -> Result<u64> {
        if let Some(&base_address) = self.manual_base_addresses.get(lib_name) {
            return Ok(base_address);
        }

        let module_name = self.get_os_lib_name(lib_name);
        match self.os.as_str() {
            "windows" => self.get_module_base_address_windows(&module_name),
            _ => self.get_module_base_address_linux(&module_name),
        }
    }

    #[cfg(target_os = "windows")]
    fn get_module_base_address_windows(&self, module_name: &str) -> Result<u64> {
        let c_module_name = CString::new(module_name)?;
        unsafe {
            let handle = GetModuleHandleA(c_module_name.as_ptr() as *const u8);
            if handle.is_null() {
                return Err(anyhow::anyhow!(
                    "Module '{}' not found or not loaded",
                    module_name
                ));
            }
            Ok(handle as u64)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_module_base_address_windows(&self, _module_name: &str) -> Result<u64> {
        Err(anyhow::anyhow!(
            "Windows module loading not supported on this platform"
        ))
    }

    #[cfg(target_os = "linux")]
    fn get_module_base_address_linux(&self, module_name: &str) -> Result<u64> {
        let maps_file = fs::File::open("/proc/self/maps")?;
        let reader = BufReader::new(maps_file);

        for line in reader.lines() {
            let line = line?;
            if line.contains(module_name) {
                // Parse the line format: "address_start-address_end perms offset dev inode pathname"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(addr_range) = parts.first() {
                    if let Some(start_addr) = addr_range.split('-').next() {
                        return u64::from_str_radix(start_addr, 16)
                            .map_err(|e| anyhow::anyhow!("Failed to parse address: {}", e));
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Module '{}' not found in process memory",
            module_name
        ))
    }

    #[cfg(not(target_os = "linux"))]
    fn get_module_base_address_linux(&self, _module_name: &str) -> Result<u64> {
        Err(anyhow::anyhow!(
            "Linux module loading not supported on this platform"
        ))
    }

    fn decorate_rtti_type_descriptor_name(&self, name: &str) -> String {
        match self.os.as_str() {
            "windows" => format!(".?AV{}@@", name),
            _ => format!("{}{}", name.len(), name),
        }
    }

    fn decorate_rtti_type_descriptor_name_nested_2(&self, class1: &str, class2: &str) -> String {
        match self.os.as_str() {
            "windows" => format!(".?AV{}@{}@@", class2, class1),
            _ => format!("N{}{}{}{}E", class1.len(), class1, class2.len(), class2),
        }
    }

    pub fn set_custom_binary_path(
        &mut self,
        binary_name: &str,
        path: &str,
        os: &str,
    ) -> Result<()> {
        if os.to_lowercase() == "windows" {
            self.custom_binary_paths_windows
                .insert(binary_name.to_string(), path.to_string());
            Ok(())
        } else if os.to_lowercase() == "linux" {
            self.custom_binary_paths_linux
                .insert(binary_name.to_string(), path.to_string());
            Ok(())
        } else {
            anyhow::bail!("Unsupported OS: {}", os);
        }
    }

    pub fn new(game_path: &str, game_type: &str, os: &str) -> Self {
        Self {
            game_path: PathBuf::from(game_path),
            game_type: game_type.to_string(),
            os: os.to_string(),
            binaries: HashMap::new(),
            manual_base_addresses: HashMap::new(),
            xrefs_cache: HashMap::new(),
            trampolines: HashMap::new(),
            custom_binary_paths_windows: HashMap::new(),
            custom_binary_paths_linux: HashMap::new(),
            vtables: HashMap::new(),
            name_to_vtables: HashMap::new(),
            strings_cache: HashMap::new(),
            calls_targets_cache: HashMap::new(),
        }
    }

    pub fn set_module_base_from_pointer(&mut self, lib_name: &str, pointer: u64) {
        self.manual_base_addresses
            .insert(lib_name.to_string(), get_module_base_from_pointer(pointer));
    }

    pub fn clear_module_base_address(&mut self, lib_name: &str) {
        self.manual_base_addresses.remove(lib_name);
    }

    pub fn module_from_pointer(&self, ptr: u64) -> Result<u64> {
        let module = module_from_pointer(ptr);

        if module.is_none() {
            bail!("Failed to get module from pointer.");
        }

        let (module_name, module_base) = module.unwrap();
        // respect custom module base
        for (manual_module_name, manual_module_base) in &self.manual_base_addresses {
            let lib_name = self.get_os_lib_name(&manual_module_name);
            if module_name.contains(&lib_name) {
                return Ok(*manual_module_base);
            }
        }
        Ok(module_base)
    }

    pub fn get_binary_path(&self, binary_name: &str) -> String {
        if self.os.to_lowercase() == "windows" {
            if let Some(path) = self.custom_binary_paths_windows.get(binary_name) {
                return path.clone();
            }
        } else if self.os.to_lowercase() == "linux" {
            if let Some(path) = self.custom_binary_paths_linux.get(binary_name) {
                return path.clone();
            }
        }
        match binary_name {
            "server" | "client" | "matchmaking" | "host" => self
                .game_path
                .join(self.game_type.clone())
                .join("bin")
                .join(self.get_os_name())
                .join(self.get_os_lib_name(binary_name))
                .to_string_lossy()
                .to_string(),
            _ => self
                .game_path
                .join("bin")
                .join(self.get_os_name())
                .join(self.get_os_lib_name(binary_name))
                .to_string_lossy()
                .to_string(),
        }
    }

    pub fn is_binary_loaded(&self, binary_name: &str) -> bool {
        self.binaries.contains_key(binary_name)
    }

    pub fn load_binary(&mut self, binary_name: &str) {
        let binary_path = self.get_binary_path(binary_name);
        let binary_data = fs::read(binary_path.clone());
        if let Ok(binary_data) = binary_data {
            self.binaries.insert(binary_name.to_string(), binary_data);
        } else {
            println!("[Warning] Binary not found: {}", binary_path.clone());
        }
    }

    pub fn get_binary(&self, binary_name: &str) -> Result<&[u8]> {
        self.binaries
            .get(binary_name)
            .map(|v| v.as_slice())
            .ok_or_else(|| anyhow::anyhow!("Binary not found."))
    }

    fn file_offset_to_rva(&self, binary_name: &str, file_offset: u64) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;

        for section in object.sections() {
            if let Some(file_range) = section.file_range() {
                let section_file_start = file_range.0;
                let section_file_end = file_range.0 + file_range.1;

                if file_offset >= section_file_start && file_offset < section_file_end {
                    let section_rva = section.address();
                    let offset_in_section = file_offset - section_file_start;
                    return Ok(section_rva + offset_in_section);
                }
            }
        }
        Err(anyhow::anyhow!("File offset not found in any section."))
    }

    fn rva_to_file_offset(&self, binary_name: &str, rva: u64) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;

        for section in object.sections() {
            let section_rva = section.address();
            let section_size = section.size();
            let section_rva_end = section_rva + section_size;

            if rva >= section_rva && rva < section_rva_end {
                if let Some(file_range) = section.file_range() {
                    let section_file_start = file_range.0;
                    let offset_in_section = rva - section_rva;
                    return Ok(section_file_start + offset_in_section);
                }
            }
        }
        Err(anyhow::anyhow!("rva not found in any section."))
    }

    fn is_file_offset_executable(&self, binary_name: &str, file_offset: u64) -> Result<bool> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;
        for section in object.sections() {
            if let Some(file_range) = section.file_range() {
                let section_file_start = file_range.0;
                let section_file_end = file_range.0 + file_range.1;
                if file_offset >= section_file_start && file_offset < section_file_end {
                    return Ok(is_executable(section.flags()));
                }
            }
        }
        Err(anyhow::anyhow!("Address not found in any section."))
    }

    fn get_section_range(&self, binary_name: &str, section_name: &str) -> Result<(u64, u64)> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;
        let section = object
            .section_by_name(section_name)
            .ok_or_else(|| anyhow::anyhow!("Section not found."))?;
        Ok((
            section.file_range().unwrap().0,
            section.file_range().unwrap().1 + section.file_range().unwrap().0,
        ))
    }

    fn find_all_pattern_string_in_section(
        &self,
        binary_name: &str,
        section_name: &str,
        string: &str,
    ) -> Result<Vec<u64>> {
        let binary_data = self.get_binary(binary_name)?;
        let (start, end) = self.get_section_range(binary_name, section_name)?;
        let bytes = string.as_bytes();
        let mut result = Vec::new();
        let mut offset = start;
        loop {
            if offset >= end {
                break;
            }
            let search = find_pattern_simd(
                &binary_data[offset as usize..end as usize],
                bytes,
                &vec![],
            );
            let Ok(mut found) = search else { break };
            if found == 0 {
                break;
            }
            found += offset;
            result.push(found);
            offset = found + 1;
        }
        Ok(result)
    }

    fn find_pattern_int32_in_section(
        &self,
        binary_name: &str,
        section_name: &str,
        pattern: u32,
    ) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let pattern_wildcard = vec![];

        let (start, end) = self.get_section_range(binary_name, section_name)?;
        let mut result = find_pattern_simd(
            &binary_data[start as usize..end as usize],
            &pattern.to_le_bytes(),
            &pattern_wildcard,
        )?;
        if result != 0 {
            result += start;
        }
        Ok(result)
    }

    fn find_pattern_bytes_in_section(
        &self,
        binary_name: &str,
        section_name: &str,
        pattern: &[u8],
    ) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let (start, end) = self.get_section_range(binary_name, section_name)?;
        let pattern_wildcard = vec![];
        let mut result = find_pattern_simd(
            &binary_data[start as usize..end as usize],
            pattern,
            &pattern_wildcard,
        )?;
        if result != 0 {
            result += start;
        }
        Ok(result)
    }

    fn find_pattern_rva(&self, binary_name: &str, pattern_string: &str) -> Result<u64> {
        let result = Cell::new(0);
        self.pattern_scan_all_rva(binary_name, pattern_string, |_, address| {
            result.set(address);
            true
        })?;
        Ok(result.get())
    }

    fn get_image_base(&self, binary_name: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;

        match object {
            object::File::Pe64(pe) => {
                let image_base = pe.nt_headers().optional_header.image_base();
                Ok(image_base)
            }
            object::File::Pe32(pe) => {
                let image_base = pe.nt_headers().optional_header.image_base() as u64;
                Ok(image_base)
            }
            object::File::Elf64(_) | object::File::Elf32(_) => Ok(0),
            _ => Err(anyhow::anyhow!("Unsupported file format")),
        }
    }

    fn read_string(&self, binary_name: &str, file_offset: u64) -> Result<String> {
        let binary_data = self.get_binary(binary_name)?;
        let mut bytes = vec![];
        let mut file_offset = file_offset;
        while binary_data[file_offset as usize] != 0 {
            bytes.push(binary_data[file_offset as usize]);
            file_offset += 1;
        }
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn get_binary_name_by_ptr(&self, ptr: u64) -> Result<String> {
        for (binary_name, binary_data) in self.binaries.iter() {
            let base_address = self.get_module_base_address(binary_name)?;
            if ptr >= base_address && ptr < base_address + binary_data.len() as u64 {
                return Ok(binary_name.clone());
            }
        }
        Err(anyhow::anyhow!("Binary not found."))
    }

    fn find_vtable_rva_windows(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;

        let candidates =
            self.find_all_pattern_string_in_section(binary_name, ".data", vtable_name)?;
        let mut last_err: Option<anyhow::Error> = None;
        for type_descriptor_name in candidates {
            let attempt = (|| {
                let rtti_type_descriptor = self.file_offset_to_rva(binary_name, type_descriptor_name)?
                    - 0x10
                    - self.get_image_base(binary_name)?;

                let rtti_type_descriptor_ptr_pattern = rtti_type_descriptor.to_le_bytes().to_vec();

                let (_start, end) = self.get_section_range(binary_name, ".rdata")?;

                let mut reference = self.find_pattern_int32_in_section(
                    binary_name,
                    ".rdata",
                    rtti_type_descriptor as u32,
                )?;
                if reference == 0 {
                    bail!("Vtable not found.");
                }
                loop {
                    if read_int32(&binary_data, reference - 0xC) == 1
                        && read_int32(&binary_data, reference - 0x8) == 0
                    {
                        let reference_offset =
                            self.file_offset_to_rva(binary_name, reference - 0xC)?;
                        let rtti_complete_object_locator = self.find_pattern_int32_in_section(
                            binary_name,
                            ".rdata",
                            reference_offset as u32,
                        )?;
                        if rtti_complete_object_locator != 0 {
                            return Ok(self.file_offset_to_rva(
                                binary_name,
                                rtti_complete_object_locator + 8,
                            )?);
                        }
                    }
                    let last_reference = reference + 1;
                    let result = find_pattern_simd(
                        &binary_data[last_reference as usize..end as usize],
                        &rtti_type_descriptor_ptr_pattern[0..4],
                        &vec![],
                    );
                    let Ok(next) = result else { break };
                    if next == 0 {
                        break;
                    }
                    reference = next + last_reference as u64;
                }

                bail!("Vtable not found.")
            })();
            match attempt {
                Ok(r) => return Ok(r),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Vtable not found.")))
    }

    fn find_vtable_rva_linux(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;

        let candidates =
            self.find_all_pattern_string_in_section(binary_name, ".rodata", vtable_name)?;
        let mut last_err: Option<anyhow::Error> = None;

        for type_info_name in candidates {
            let attempt: std::result::Result<u64, anyhow::Error> = (|| {
                let type_info_name_str = self.read_string(binary_name, type_info_name)?;
                if type_info_name_str != vtable_name {
                    bail!("Vtable not found.");
                }

                let type_info_name_rva = self.file_offset_to_rva(binary_name, type_info_name)?;
                let type_info_name_ptr_pattern = type_info_name_rva.to_le_bytes();

                let reference_type_name = self.find_pattern_bytes_in_section(
                    binary_name,
                    ".data.rel.ro",
                    &type_info_name_ptr_pattern[0..4],
                )?;
                if reference_type_name == 0 {
                    bail!("Vtable not found.");
                }

                let type_info = reference_type_name - 0x8;
                let type_info_rva = self.file_offset_to_rva(binary_name, type_info)?;
                let type_info_ptr_pattern = type_info_rva.to_le_bytes();

                for section_name in &[".data.rel.ro", ".data.rel.ro.local"] {
                    if let Ok((start, end)) = self.get_section_range(binary_name, section_name) {
                        let mut search_offset = start;
                        loop {
                            let result = find_pattern_simd(
                                &binary_data[search_offset as usize..end as usize],
                                &type_info_ptr_pattern,
                                &vec![],
                            );

                            let Ok(mut reference) = result else { break };
                            if reference == 0 {
                                break;
                            }

                            reference += search_offset;

                            if reference >= 0x8 {
                                let offset_to_this = read_int64(binary_data, reference - 0x8);
                                if offset_to_this == 0 {
                                    return Ok(self.file_offset_to_rva(
                                        binary_name,
                                        reference + 0x8,
                                    )?);
                                }
                            }

                            search_offset = reference + 8;
                            if search_offset >= end {
                                break;
                            }
                        }
                    }
                }

                bail!("Vtable not found.")
            })();

            match attempt {
                Ok(r) => return Ok(r),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Vtable not found.")))
    }

    fn is_valid_rva(&self, binary_name: &str, rva: u64) -> Result<bool> {
        let Ok(file_offset) = self.rva_to_file_offset(binary_name, rva) else {
            return Ok(false);
        };

        Ok(file_offset > 0 && file_offset < self.get_binary(binary_name)?.len() as u64)
    }

    fn is_valid_executable_rva(&self, binary_name: &str, rva: u64) -> Result<bool> {
        if !self.is_valid_rva(binary_name, rva)? {
            return Ok(false);
        }

        let file_offset = self.rva_to_file_offset(binary_name, rva)?;
        self.is_file_offset_executable(binary_name, file_offset)
    }

    fn mem_address_to_rva(&self, binary_name: &str, address: u64) -> Result<u64> {
        let base_address = self.get_module_base_address(binary_name)?;
        let image_base = self.get_image_base(binary_name)?;
        Ok(address - base_address + image_base)
    }

    pub fn rva_to_mem_address(&self, binary_name: &str, address: u64) -> Result<u64> {
        let base_address = self.get_module_base_address(binary_name)?;
        let image_base = self.get_image_base(binary_name)?;
        Ok(address - image_base + base_address)
    }

    pub fn find_vtable_rva(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        self.find_vtable_mangled_rva(
            binary_name,
            &self.decorate_rtti_type_descriptor_name(vtable_name),
        )
    }

    pub fn find_vtable_nested_2_rva(
        &self,
        binary_name: &str,
        class1_name: &str,
        class2_name: &str,
    ) -> Result<u64> {
        self.find_vtable_mangled_rva(
            binary_name,
            &self.decorate_rtti_type_descriptor_name_nested_2(class1_name, class2_name),
        )
    }

    pub fn find_vtable_mangled_rva(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        match self.os.as_str() {
            "windows" => self.find_vtable_rva_windows(binary_name, vtable_name),
            _ => self.find_vtable_rva_linux(binary_name, vtable_name),
        }
    }

    pub fn get_vtable_vfunc_count(&self, binary_name: &str, vtable_name: &str) -> Result<usize> {
        let vtable_rva = self.find_vtable_rva(binary_name, vtable_name)?;
        self.get_vtable_vfunc_count_by_rva(binary_name, vtable_rva)
    }

    pub fn get_vtable_vfunc_count_by_rva(
        &self,
        binary_name: &str,
        vtable_rva: u64,
    ) -> Result<usize> {
        let mut offset = 0;

        loop {
            let vfunc_rva = self.read_by_rva(binary_name, vtable_rva + offset, 8)?;
            let vfunc_rva = u64::from_le_bytes(vfunc_rva.try_into().unwrap());

            if vfunc_rva == 0 || !self.is_valid_executable_rva(binary_name, vfunc_rva)? {
                break;
            }

            // check if its a valid function
            offset += 8;
        }
        Ok(offset as usize / 8)
    }

    pub fn find_vtable(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        let result = self.find_vtable_rva(binary_name, vtable_name)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn find_vtable_mangled(&self, binary_name: &str, vtable_name: &str) -> Result<u64> {
        let result = self.find_vtable_mangled_rva(binary_name, vtable_name)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn find_vtable_nested_2(
        &self,
        binary_name: &str,
        class1_name: &str,
        class2_name: &str,
    ) -> Result<u64> {
        let result = self.find_vtable_nested_2_rva(binary_name, class1_name, class2_name)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn pattern_scan_rva(&self, binary_name: &str, pattern_string: &str) -> Result<u64> {
        self.find_pattern_rva(binary_name, pattern_string)
    }

    pub fn pattern_scan(&self, binary_name: &str, pattern_string: &str) -> Result<u64> {
        let result = self.find_pattern_rva(binary_name, pattern_string)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn pattern_scan_all_rva(
        &self,
        binary_name: &str,
        pattern_string: &str,
        callback: impl Fn(usize, u64) -> bool,
    ) -> Result<()> {
        let binary_data = self.get_binary(binary_name)?;
        let pattern = pattern_string
            .split(" ")
            .map(|x| {
                if x == "?" || x == "??" {
                    0u8
                } else {
                    u8::from_str_radix(x, 16).unwrap()
                }
            })
            .collect::<Vec<u8>>();
        let pattern_wildcard = pattern_string
            .split(" ")
            .enumerate()
            .filter(|(_, x)| *x == "?" || *x == "??")
            .map(|(index, _)| index)
            .collect::<Vec<usize>>();

        let mut offset = 0;
        let mut match_index = 0;
        while offset + pattern.len() < binary_data.len() {
            let result = find_pattern_simd(&binary_data[offset..], &pattern, &pattern_wildcard)?;

            if result == 0 {
                return Ok(());
            }

            offset += result as usize;

            if callback(
                match_index,
                self.file_offset_to_rva(binary_name, offset as u64)?,
            ) {
                return Ok(());
            }

            match_index += 1;
            offset += 1;
        }

        Err(anyhow::anyhow!("Pattern not found."))
    }

    pub fn pattern_scan_all(
        &self,
        binary_name: &str,
        pattern_string: &str,
        callback: impl Fn(usize, u64) -> bool,
    ) -> Result<()> {
        // pre check error
        let _ = self.get_module_base_address(binary_name)?;
        let _ = self.get_image_base(binary_name)?;

        self.pattern_scan_all_rva(binary_name, pattern_string, |index, x| {
            callback(index, self.rva_to_mem_address(binary_name, x).unwrap())
        })
    }

    pub fn find_export_rva(&self, binary_name: &str, export_name: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;

        for export in object.exports()? {
            if String::from_utf8_lossy(export.name()) == export_name {
                return Ok(export.address() as u64);
            }
        }
        Err(anyhow::anyhow!("Export not found."))
    }

    pub fn find_export(&self, binary_name: &str, export_name: &str) -> Result<u64> {
        let result = self.find_export_rva(binary_name, export_name)?;
        Ok(self.mem_address_to_rva(binary_name, result)?)
    }

    pub fn find_symbol_rva(&self, binary_name: &str, symbol_name: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;

        let symbol = object
            .dynamic_symbols()
            .into_iter()
            .find(|s| s.name() == Ok(symbol_name))
            .ok_or_else(|| anyhow::anyhow!("Symbol not found."))?;
        Ok(symbol.address() as u64)
    }

    pub fn find_symbol(&self, binary_name: &str, symbol_name: &str) -> Result<u64> {
        let result = self.find_symbol_rva(binary_name, symbol_name)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn read_by_file_offset(
        &self,
        binary_name: &str,
        file_offset: u64,
        size: usize,
    ) -> Result<&[u8]> {
        let binary_data: &[u8] = self.get_binary(binary_name)?;
        Ok(&binary_data[file_offset as usize..file_offset as usize + size])
    }

    pub fn read_by_rva(&self, binary_name: &str, address: u64, size: usize) -> Result<&[u8]> {
        let file_offset = self.rva_to_file_offset(binary_name, address)?;
        self.read_by_file_offset(binary_name, file_offset, size)
    }

    pub fn read_by_mem_address(
        &self,
        binary_name: &str,
        address: u64,
        size: usize,
    ) -> Result<&[u8]> {
        let rva = self.mem_address_to_rva(binary_name, address)?;
        self.read_by_rva(binary_name, rva, size)
    }

    pub fn find_vfunc_by_vtbname_rva(
        &self,
        binary_name: &str,
        vtb_name: &str,
        vfunc_index: usize,
    ) -> Result<u64> {
        let vtb = self.find_vtable_rva(binary_name, vtb_name)?;

        let vfuncptr = self.read_by_rva(binary_name, vtb + vfunc_index as u64 * 8, 8)?;
        Ok(u64::from_le_bytes(vfuncptr.try_into().unwrap()))
    }

    pub fn find_vfunc_by_vtbname(
        &self,
        binary_name: &str,
        vtb_name: &str,
        vfunc_index: usize,
    ) -> Result<u64> {
        let vtb = self.find_vfunc_by_vtbname_rva(binary_name, vtb_name, vfunc_index)?;
        Ok(self.rva_to_mem_address(binary_name, vtb)?)
    }

    pub fn find_vfunc_by_vtbptr_rva(&self, vtb_ptr: u64, vfunc_index: usize) -> Result<u64> {
        let binary_name = self.get_binary_name_by_ptr(vtb_ptr)?;
        let vtb_rva = self.mem_address_to_rva(&binary_name, vtb_ptr)?;
        let vfuncptr = self.read_by_rva(&binary_name, vtb_rva + vfunc_index as u64 * 8, 8)?;
        Ok(u64::from_le_bytes(vfuncptr.try_into().unwrap()))
    }

    pub fn find_vfunc_by_vtbptr(&self, vtb_ptr: u64, vfunc_index: usize) -> Result<u64> {
        let binary_name = self.get_binary_name_by_ptr(vtb_ptr)?;
        let vtb_rva = self.mem_address_to_rva(&binary_name, vtb_ptr)?;
        let vfuncptr = self.read_by_rva(&binary_name, vtb_rva + vfunc_index as u64 * 8, 8)?;
        let vfunc_rva = u64::from_le_bytes(vfuncptr.try_into().unwrap());
        Ok(self.rva_to_mem_address(&binary_name, vfunc_rva)?)
    }

    pub fn find_string_rva(&self, binary_name: &str, string: &str) -> Result<u64> {
        let binary_data = self.get_binary(binary_name)?;
        let string_bytes = string.as_bytes();
        let result = find_pattern_simd(binary_data, string_bytes, &vec![])?;
        Ok(self.file_offset_to_rva(binary_name, result)?)
    }

    pub fn find_string(&self, binary_name: &str, string: &str) -> Result<u64> {
        let result = self.find_string_rva(binary_name, string)?;
        Ok(self.rva_to_mem_address(binary_name, result)?)
    }

    pub fn dump_xrefs(&mut self, binary_name: &str) -> Result<()> {
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;
        let image_base = self.get_image_base(binary_name)?;

        // Temporary storage for xrefs
        let mut xrefs_map: HashMap<u64, Vec<u64>> = HashMap::new();

        // Call commands
        let mut calls_targets_rva: Vec<u64> = Vec::new();

        // Determine bitness for decoder
        let bitness = match object {
            object::File::Pe64(_) | object::File::Elf64(_) => 64,
            object::File::Pe32(_) | object::File::Elf32(_) => 32,
            _ => return Err(anyhow::anyhow!("Unsupported file format")),
        };

        // Iterate through all sections
        for section in object.sections() {
            // Skip non-executable sections
            if !is_executable(section.flags()) {
                continue;
            }

            // Get section data
            let section_data = match section.data() {
                Ok(data) => data,
                Err(_) => continue,
            };

            // Get section virtual address
            let section_rva = section.address();

            // Create decoder
            let mut decoder =
                Decoder::with_ip(bitness, section_data, section_rva, DecoderOptions::NONE);

            let mut instruction = Instruction::default();

            // Decode all instructions in the section
            while decoder.can_decode() {
                decoder.decode_out(&mut instruction);

                // Skip invalid instructions
                if instruction.is_invalid() {
                    continue;
                }

                let instr_rva = instruction.ip();

                let is_call = instruction.is_call_far() || instruction.is_call_near();

                // Analyze instruction operands for memory references
                for i in 0..instruction.op_count() {
                    let op_kind = instruction.op_kind(i);

                    match op_kind {
                        // Direct memory operand (e.g., mov rax, [0x140001000])
                        OpKind::Memory => {
                            if instruction.is_ip_rel_memory_operand() {
                                // RIP-relative addressing
                                let target_rva = instruction.ip_rel_memory_address();
                                xrefs_map
                                    .entry(target_rva)
                                    .or_insert_with(Vec::new)
                                    .push(instr_rva);
                                if is_call {
                                    calls_targets_rva.push(target_rva);
                                }
                            } else if instruction.memory_base() == Register::None
                                && instruction.memory_index() == Register::None
                            {
                                // Absolute addressing
                                let displacement = instruction.memory_displacement64();
                                if displacement != 0 {
                                    xrefs_map
                                        .entry(displacement)
                                        .or_insert_with(Vec::new)
                                        .push(instr_rva);
                                    if is_call {
                                        calls_targets_rva.push(displacement);
                                    }
                                }
                            }
                        }

                        // Near branch (call, jmp, jcc)
                        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                            let target_rva = instruction.near_branch_target();
                            xrefs_map
                                .entry(target_rva)
                                .or_insert_with(Vec::new)
                                .push(instr_rva);
                            if is_call {
                                calls_targets_rva.push(target_rva);
                            }
                        }

                        // Immediate rvalues that might be addresses
                        OpKind::Immediate32 | OpKind::Immediate64 => {
                            let immediate = if bitness == 64 {
                                instruction.immediate(i)
                            } else {
                                instruction.immediate(i) as u32 as u64
                            };

                            // Only consider rvalues that look like valid virtual addresses
                            // For PE files, check if it's near the image base
                            // For ELF files, check if it's in a reasonable range
                            let is_likely_address = if bitness == 64 {
                                immediate >= image_base && immediate < image_base + 0x10000000
                            } else {
                                immediate >= image_base && immediate < image_base + 0x1000000
                            };

                            if is_likely_address {
                                xrefs_map
                                    .entry(immediate)
                                    .or_insert_with(Vec::new)
                                    .push(instr_rva);
                                if is_call {
                                    calls_targets_rva.push(immediate);
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
        }

        // Store the collected xrefs in the cache
        self.xrefs_cache.insert(binary_name.to_string(), xrefs_map);

        calls_targets_rva.sort();
        self.calls_targets_cache
            .insert(binary_name.to_string(), calls_targets_rva);

        Ok(())
    }

    pub fn dump_strings(&mut self, binary_name: &str) -> Result<()> {
        const MIN_LENGTH: u64 = 4;
        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;
        let mut strings_map: HashMap<String, u64> = HashMap::new();

        let mut section_ranges = vec![];
        for section in object.sections() {
            match section.kind() {
                // Skip code and uninitialized data, scan everything else
                SectionKind::Text
                | SectionKind::UninitializedData
                | SectionKind::UninitializedTls => continue,
                _ => {}
            }
            if let Some((start, size)) = section.file_range() {
                let end = (start + size).min(binary_data.len() as u64);
                if start < end {
                    section_ranges.push((start, end));
                }
            }
        }

        for section_range in section_ranges {
            let start = section_range.0;
            let end = section_range.1;
            let mut index = start;
            while index < end {
                let byte = binary_data[index as usize];
                if !byte.is_ascii() || (byte < 0x20 || byte > 0x7E) {
                    index += 1;
                    continue;
                }

                let start = index;
                while index < end {
                    let b = binary_data[index as usize];
                    if !b.is_ascii() || (b < 0x20 || b > 0x7E) {
                        break;
                    }
                    index += 1;
                }

                let length = index - start;
                if length >= MIN_LENGTH {
                    let slice = &binary_data[start as usize..index as usize];
                    if let Ok(rva) = self.file_offset_to_rva(binary_name, start as u64) {
                        let string = String::from_utf8_lossy(slice).to_string();
                        strings_map.insert(string, rva);
                    }
                }
            }
        }

        self.strings_cache
            .insert(binary_name.to_string(), strings_map);

        Ok(())
    }

    /// Dump all printable ASCII strings and their RVAs to a JSON file.
    ///
    /// The binary is scanned before every write so the output always reflects
    /// the currently loaded binary. JSON object keys are sorted to make dumps
    /// deterministic across runs.
    pub fn dump_strings_to_json<P: AsRef<Path>>(
        &mut self,
        binary_name: &str,
        output_path: P,
    ) -> Result<()> {
        self.dump_strings(binary_name)?;

        let strings = self
            .get_strings(binary_name)
            .ok_or_else(|| anyhow::anyhow!("Strings were not cached."))?;
        write_strings_to_json(strings, output_path)
    }

    pub fn get_strings(&self, binary_name: &str) -> Option<&HashMap<String, u64>> {
        self.strings_cache.get(binary_name)
    }

    pub fn find_xrefs_cached(&self, binary_name: &str, target_rva: u64) -> Option<&Vec<u64>> {
        self.xrefs_cache
            .get(binary_name)
            .and_then(|map| map.get(&target_rva))
    }

    pub fn unload_binary(&mut self, binary_name: &str) {
        self.binaries.remove(binary_name);
    }

    pub fn unload_all_binaries(&mut self) {
        self.binaries.clear();
    }

    pub fn install_trampoline(&mut self, mem_address: u64) -> Result<u64> {
        if let Some(trampoline) = self.trampolines.get(&mem_address) {
            return Ok(trampoline.address());
        }

        let original_func_ptr = unsafe { std::ptr::read(mem_address as *const u64) };

        let trampoline = JitTrampoline::new(original_func_ptr)?;

        set_mem_access(mem_address, 8)?;

        unsafe {
            std::ptr::write(mem_address as *mut u64, trampoline.address());
        }

        let address = trampoline.address();
        self.trampolines.insert(mem_address, trampoline);

        Ok(address)
    }

    pub fn follow_xref_mem_to_mem(&self, mem_address: u64) -> Result<u64> {
        const MAX_INSTR_LEN: usize = 15;
        let mut instruction_bytes = [0u8; MAX_INSTR_LEN];

        unsafe {
            std::ptr::copy_nonoverlapping(
                mem_address as *const u8,
                instruction_bytes.as_mut_ptr(),
                MAX_INSTR_LEN,
            );
        }

        let mut decoder =
            Decoder::with_ip(64, &instruction_bytes, mem_address, DecoderOptions::NONE);

        let mut instruction = Instruction::default();
        decoder.decode_out(&mut instruction);

        if instruction.is_invalid() {
            return Err(anyhow::anyhow!(
                "Invalid instruction at address 0x{:X}",
                mem_address
            ));
        }

        for i in 0..instruction.op_count() {
            let op_kind = instruction.op_kind(i);

            match op_kind {
                OpKind::Memory => {
                    if instruction.is_ip_rel_memory_operand() {
                        let target_address = instruction.ip_rel_memory_address();
                        return Ok(target_address);
                    } else if instruction.memory_base() == Register::None
                        && instruction.memory_index() == Register::None
                    {
                        let displacement = instruction.memory_displacement64();
                        if displacement != 0 {
                            return Ok(displacement);
                        }
                    }
                }

                OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                    let target_address = instruction.near_branch_target();
                    return Ok(target_address);
                }

                _ => {}
            }
        }

        Err(anyhow::anyhow!(
            "No valid xref found in instruction at address 0x{:X}",
            mem_address
        ))
    }

    pub fn follow_xref_rva_to_mem(&self, binary_name: &str, rva: u64) -> Result<u64> {
        let mem_address = self.rva_to_mem_address(binary_name, rva)?;
        self.follow_xref_mem_to_mem(mem_address)
    }

    pub fn follow_xref_rva_to_rva(&self, binary_name: &str, rva: u64) -> Result<u64> {
        let file_offset = self.rva_to_file_offset(binary_name, rva)?;
        let binary_data = self.get_binary(binary_name)?;

        const MAX_INSTR_LEN: usize = 15;

        if file_offset as usize + MAX_INSTR_LEN > binary_data.len() {
            return Err(anyhow::anyhow!(
                "Instruction at rva 0x{:X} extends beyond binary data",
                rva
            ));
        }

        let instruction_bytes =
            &binary_data[file_offset as usize..file_offset as usize + MAX_INSTR_LEN];

        let mut decoder = Decoder::with_ip(64, instruction_bytes, rva, DecoderOptions::NONE);

        let mut instruction = Instruction::default();
        decoder.decode_out(&mut instruction);

        if instruction.is_invalid() {
            return Err(anyhow::anyhow!("Invalid instruction at rva 0x{:X}", rva));
        }

        for i in 0..instruction.op_count() {
            let op_kind = instruction.op_kind(i);

            match op_kind {
                OpKind::Memory => {
                    if instruction.is_ip_rel_memory_operand() {
                        let target_address = instruction.ip_rel_memory_address();
                        return Ok(target_address);
                    } else if instruction.memory_base() == Register::None
                        && instruction.memory_index() == Register::None
                    {
                        let displacement = instruction.memory_displacement64();
                        if displacement != 0 {
                            return Ok(displacement);
                        }
                    }
                }

                OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
                    let target_address = instruction.near_branch_target();
                    return Ok(target_address);
                }

                _ => {}
            }
        }

        Err(anyhow::anyhow!(
            "No valid xref found in instruction at rva 0x{:X}",
            rva
        ))
    }

    pub fn find_networkvar_vtable_statechanged_rva(&self, vtable_rva: u64) -> Result<u64> {
        let vfunc_count = self.get_vtable_vfunc_count_by_rva("server", vtable_rva)?;

        for i in 0..vfunc_count {
            let vfunc_rva = self.read_by_rva("server", vtable_rva + i as u64 * 8, 8)?;
            let vfunc_rva = u64::from_le_bytes(vfunc_rva.try_into().unwrap());

            let bytes = self.read_by_rva("server", vfunc_rva, 32)?;

            let mut iced = Decoder::with_ip(64, &bytes, 0, DecoderOptions::NONE);

            while iced.can_decode() {
                let inst = iced.decode();
                if inst.code() == Code::Cmp_rm32_imm8 {
                    let mem_displ = inst.memory_displacement32();
                    let immediate = inst.immediate8();
                    // 56 is NetworkStateChangedData->m_nPathIndex
                    if mem_displ == 56 && immediate == 255 {
                        return Ok(i as u64);
                    }
                }
            }
        }
        Err(anyhow::anyhow!("NetworkVar_StateChanged not found"))
    }

    pub fn find_networkvar_vtable_statechanged(&self, vtable_mem_address: u64) -> Result<u64> {
        let vtable_rva = self.mem_address_to_rva("server", vtable_mem_address)?;
        self.find_networkvar_vtable_statechanged_rva(vtable_rva)
    }

    pub fn is_nullsub_rva(&self, binary_name: &str, func_rva: u64) -> Result<bool> {
        let bytes = self.read_by_rva(binary_name, func_rva, 3)?;
        if bytes[0] == 0xC2 {
            return Ok(true);
        }

        if bytes[0] == 0xC3 {
            return Ok(true);
        }

        if bytes[0] == 0xB0 && bytes[1] == 0x01 && bytes[2] == 0xC3 {
            return Ok(true);
        }

        Ok(false)
    }

    pub fn is_nullsub(&self, binary_name: &str, func_mem_address: u64) -> Result<bool> {
        let func_rva = self.mem_address_to_rva("server", func_mem_address)?;
        self.is_nullsub_rva(binary_name, func_rva)
    }

    fn build_ida_signature(signature: &[(u8, bool)]) -> String {
        let mut out = String::new();
        for (i, (byte, wildcard)) in signature.iter().enumerate() {
            if *wildcard {
                out.push('?');
            } else {
                out.push_str(&format!("{:02X}", byte));
            }
            if i + 1 != signature.len() {
                out.push(' ');
            }
        }
        out
    }

    fn trim_signature(signature: &mut Vec<(u8, bool)>) {
        while signature.last().map_or(false, |(_, w)| *w) {
            signature.pop();
        }
    }

    fn is_signature_unique(&self, binary_name: &str, signature: &[(u8, bool)]) -> Result<bool> {
        let count = Cell::new(0usize);
        let sig_str = Self::build_ida_signature(signature);
        let res = self.pattern_scan_all_rva(binary_name, &sig_str, |_, _| {
            let next = count.get() + 1;
            count.set(next);
            next > 1
        });
        Ok(count.get() == 1)
    }

    pub fn make_sig_rva(&self, binary_name: &str, func_rva: u64) -> Result<String> {
        const MAX_SIGNATURE_LENGTH: usize = 1000;
        const WILDCARD_OPERANDS: bool = true;
        const DONT_WILDCARD_ZERO_IMM: bool = true;

        let binary_data = self.get_binary(binary_name)?;
        let object = object::File::parse(binary_data)?;
        let bitness = match object {
            object::File::Pe64(_) | object::File::Elf64(_) => 64,
            object::File::Pe32(_) | object::File::Elf32(_) => 32,
            _ => bail!("Unsupported file format"),
        };

        let start_offset = self.rva_to_file_offset(binary_name, func_rva)? as usize;
        if start_offset >= binary_data.len() {
            bail!("Invalid rva");
        }

        let mut decoder =
            Decoder::with_ip(bitness, &binary_data[start_offset..], func_rva, DecoderOptions::NONE);
        let mut instruction = Instruction::default();

        let mut signature: Vec<(u8, bool)> = Vec::new();
        let mut consumed = start_offset;
        let mut total_len = 0usize;

        while decoder.can_decode() {
            decoder.decode_out(&mut instruction);
            if instruction.is_invalid() {
                if signature.is_empty() {
                    bail!("Failed to decode instruction");
                }
                break;
            }

            let inst_len = instruction.len() as usize;
            if inst_len == 0 || consumed + inst_len > binary_data.len() {
                bail!("Instruction out of range");
            }

            total_len += inst_len;
            if total_len > MAX_SIGNATURE_LENGTH {
                bail!("Signature exceeded maximum length");
            }

            let const_offsets = decoder.get_constant_offsets(&instruction);
            let disp_off = const_offsets.displacement_offset() as usize;
            let disp_size = const_offsets.displacement_size() as usize;
            let imm1_off = const_offsets.immediate_offset() as usize;
            let imm1_size = const_offsets.immediate_size() as usize;
            let imm2_off = const_offsets.immediate_offset2() as usize;
            let imm2_size = const_offsets.immediate_size2() as usize;

            let mut wildcard_mask = vec![false; inst_len];

            if WILDCARD_OPERANDS {
                if disp_size > 0 && disp_off + disp_size <= inst_len {
                    for i in disp_off..disp_off + disp_size {
                        wildcard_mask[i] = true;
                    }
                }

                let imm_ranges = [
                    (imm1_off, imm1_size),
                    (imm2_off, imm2_size),
                ];

                for (off, size) in imm_ranges {
                    if size == 0 || off + size > inst_len {
                        continue;
                    }
                    let bytes = &binary_data[consumed + off..consumed + off + size];
                    let is_zero = bytes.iter().all(|b| *b == 0);
                    let should_wildcard = !(DONT_WILDCARD_ZERO_IMM && is_zero);
                    if should_wildcard {
                        for i in off..off + size {
                            wildcard_mask[i] = true;
                        }
                    }
                }
            }

            for i in 0..inst_len {
                let byte = binary_data[consumed + i];
                signature.push((byte, wildcard_mask[i]));
            }

            if self.is_signature_unique(binary_name, &signature)? {
                Self::trim_signature(&mut signature);
                return Ok(Self::build_ida_signature(&signature));
            }

            consumed += inst_len;
        }

        bail!("Signature not found");
    }

    fn find_func_start_via_padding_rva(&self, binary_name: &str, include_rva: u64) -> Option<u64> {
        const MIN_PADDING: usize = 2;

        let binary_data = self.get_binary(binary_name).ok()?;
        let object = object::File::parse(binary_data).ok()?;

        let mut bounds = None;
        for section in object.sections() {
            let section_rva = section.address();
            let section_size = section.size();
            if include_rva >= section_rva && include_rva < section_rva + section_size {
                let (file_start, _file_size) = section.file_range()?;
                bounds = Some((section_rva, file_start));
                break;
            }
        }
        let (section_rva, file_start) = bounds?;

        let include_off = self.rva_to_file_offset(binary_name, include_rva).ok()? as usize;
        let file_start = file_start as usize;

        let mut i = include_off;
        let mut run = 0usize;
        while i > file_start {
            i -= 1;
            if binary_data[i] == 0xCC {
                run += 1;
                if run >= MIN_PADDING {
                    let start_off = i + run;
                    return Some(section_rva + (start_off as u64 - file_start as u64));
                }
            } else {
                run = 0;
            }
        }
        None
    }

    pub fn find_xref_func_start_rva(&self, binary_name: &str, include_rva: u64) -> Result<u64> {
        let mut nearest_rva = 0u64;
        if let Some(cache) = self.calls_targets_cache.get(binary_name) {
            let rva = match cache.binary_search(&include_rva) {
                Ok(idx) => {
                    if idx > 0 {
                        Some(cache[idx - 1])
                    } else {
                        None
                    }
                }
                Err(idx) => {
                    if idx > 0 {
                        Some(cache[idx - 1])
                    } else {
                        None
                    }
                }
            };
            if let Some(rva) = rva {
                nearest_rva = rva;
            }
        };

        Ok(nearest_rva)
    }

    pub fn find_xref_func_start(&self, binary_name: &str, include_rva: u64) -> Result<u64> {
        self.rva_to_mem_address(
            binary_name,
            self.find_xref_func_start_rva(binary_name, include_rva)?,
        )
    }

    pub fn find_vfunc_start_rva(
        &self,
        binary_name: &str,
        include_rva: u64,
    ) -> Option<(&VTableInfo, usize, u64)> {
        let mut nearest_rva = 0u64;
        let mut result: Option<(&VTableInfo, usize, u64)> = None;
        if let Some(vtables) = self.vtables.get(binary_name) {
            for vtable in vtables {
                for (i, func) in vtable.methods.iter().enumerate() {
                    let func = *func;
                    if func > nearest_rva && func < include_rva {
                        nearest_rva = func;
                        result = Some((vtable, i, nearest_rva));
                    }
                }
            }
        };

        result
    }

    pub fn find_vfunc_start(
        &self,
        binary_name: &str,
        include_rva: u64,
    ) -> Result<(&VTableInfo, usize, u64)> {
        let result = self.find_vfunc_start_rva(binary_name, include_rva);
        if result.is_none() {
            bail!("No vfunc found.");
        }
        Ok((
            result.unwrap().0,
            result.unwrap().1,
            self.rva_to_mem_address(binary_name, result.unwrap().2)?,
        ))
    }

    pub fn find_func_start_rva(&self, binary_name: &str, include_rva: u64) -> Result<u64> {
        let xref_start = self.find_xref_func_start_rva(binary_name, include_rva)?;
        let vfunc_start = self
            .find_vfunc_start_rva(binary_name, include_rva)
            .map(|(_, _, rva)| rva)
            .unwrap_or_default();
        let mut func_start = std::cmp::max(xref_start, vfunc_start);

        if let Some(padding_start) = self.find_func_start_via_padding_rva(binary_name, include_rva) {
            func_start = padding_start;
        }

        if func_start == 0 {
            bail!("No function found.");
        }

        Ok(func_start)
    }

    pub fn find_func_start(&self, binary_name: &str, include_rva: u64) -> Result<u64> {
        self.rva_to_mem_address(
            binary_name,
            self.find_func_start_rva(binary_name, include_rva)?,
        )
    }

    fn get_string_reference_xref(&self, binary_name: &str, string: &str) -> Result<u64> {
        let str_rva = self.find_string_rva(binary_name, string)?;
        let xref = self.find_xrefs_cached(binary_name, str_rva);

        if xref.is_none() || xref.unwrap().iter().count() < 1 {
            bail!("String reference not found.");
        };

        let xref = xref.unwrap();

        if xref.iter().count() > 1 {
            bail!("Multiple string references found.")
        };

        Ok(*xref.get(0).unwrap())
    }

    pub fn find_xref_func_with_string_rva(&self, binary_name: &str, string: &str) -> Result<u64> {
        self.find_xref_func_start_rva(
            binary_name,
            self.get_string_reference_xref(binary_name, string)?,
        )
    }

    pub fn find_xref_func_with_string(&self, binary_name: &str, string: &str) -> Result<u64> {
        self.rva_to_mem_address(
            binary_name,
            self.find_xref_func_with_string_rva(binary_name, string)?,
        )
    }

    pub fn find_vfunc_with_string_rva(
        &self,
        binary_name: &str,
        string: &str,
    ) -> Result<(&VTableInfo, usize, u64)> {
        let result = self.find_vfunc_start_rva(
            binary_name,
            self.get_string_reference_xref(binary_name, string)?,
        );

        if result.is_none() {
            bail!("No vfunc found.")
        };

        Ok(result.unwrap())
    }

    pub fn find_vfunc_with_string(
        &self,
        binary_name: &str,
        string: &str,
    ) -> Result<(&VTableInfo, usize, u64)> {
        let result = self.find_vfunc_with_string_rva(binary_name, string)?;
        Ok((
            result.0,
            result.1,
            self.rva_to_mem_address(binary_name, result.2)?,
        ))
    }

    pub fn find_func_with_string_rva(&self, binary_name: &str, string: &str) -> Result<u64> {
        self.find_func_start_rva(
            binary_name,
            self.get_string_reference_xref(binary_name, string)?,
        )
    }

    pub fn find_func_with_string(&self, binary_name: &str, string: &str) -> Result<u64> {
        self.find_func_start(
            binary_name,
            self.get_string_reference_xref(binary_name, string)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_json_is_sorted_and_contains_rvas() {
        let mut strings = HashMap::new();
        strings.insert("zeta".to_string(), 0x200);
        strings.insert("alpha".to_string(), 0x100);
        let output_path =
            std::env::temp_dir().join(format!("s2binlib-strings-{}.json", std::process::id()));

        write_strings_to_json(&strings, &output_path).unwrap();

        let output = fs::read_to_string(&output_path).unwrap();
        let parsed: BTreeMap<String, u64> = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed.get("alpha"), Some(&0x100));
        assert_eq!(parsed.get("zeta"), Some(&0x200));
        assert!(output.find("alpha").unwrap() < output.find("zeta").unwrap());

        fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn find_func_start_rva_falls_back_to_xref_without_vtable_match() {
        let mut library = S2BinLib::new(".", "csgo", "windows");
        library
            .calls_targets_cache
            .insert("server".to_string(), vec![0x100]);

        assert_eq!(library.find_func_start_rva("server", 0x200).unwrap(), 0x100);
    }

    #[test]
    fn find_func_start_rva_returns_error_without_candidates() {
        let library = S2BinLib::new(".", "csgo", "windows");

        assert!(library.find_func_start_rva("server", 0x200).is_err());
    }
}
