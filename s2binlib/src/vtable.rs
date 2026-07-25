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

use hashbrown::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use cpp_demangle::Symbol;
use msvc_demangler::{DemangleFlags, demangle};
use object::{BinaryFormat, Object, read::pe::ImageOptionalHeader};

use crate::{
    s2binlib::S2BinLib,
    view::{BinaryView, FileBinaryView, SectionInfo},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VTableInfo {
    pub type_name: String,
    pub vtable_address: u64,
    pub methods: Vec<u64>,
    pub bases: Vec<BaseClassInfo>,
    pub model: VTableModel,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VTableModel {
    Msvc {
        complete_object_locator: u64,
        offset: u32,
        constructor_displacement: u32,
        class_attributes: u32,
    },
    Itanium {
        offset_to_top: i64,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseClassInfo {
    pub type_name: String,
    pub details: BaseClassModel,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BaseClassModel {
    Msvc {
        attributes: u32,
        displacement: Pmd,
        num_contained_bases: u32,
    },
    Itanium {
        offset: i64,
        flags: u32,
        bases: Vec<BaseClassInfo>,
    },
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pmd {
    pub mdisp: i32,
    pub pdisp: i32,
    pub vdisp: i32,
}

impl<'a> S2BinLib<'a> {
    pub fn dump_vtables(&mut self, binary_name: &str) -> Result<()> {
        let binary = self.get_binary(binary_name)?;
        let file = object::File::parse(binary)?;

        if !file.is_64() {
            return Err(anyhow!("only x64 binaries are supported"));
        }

        let image_base = match &file {
            object::File::Pe64(pe) => pe.nt_headers().optional_header.image_base(),
            object::File::Pe32(pe) => pe.nt_headers().optional_header.image_base() as u64,
            object::File::Elf64(_) | object::File::Elf32(_) => 0,
            _ => 0,
        };

        let view = FileBinaryView::new(binary, &file, image_base)?;

        let mut vtables = match file.format() {
            BinaryFormat::Pe => MsvcParser::new(&view).parse(),
            BinaryFormat::Elf => ItaniumParser::new(&view).parse(),
            _ => Err(anyhow!("unsupported binary format")),
        }?;
        vtables.sort_by(|a, b| a.type_name.cmp(&b.type_name));

        self.vtables.insert(binary_name.to_string(), vtables);
        Ok(())
    }

    pub fn get_vtables(&self, binary_name: &str) -> Result<&Vec<VTableInfo>> {
        self.vtables
            .get(binary_name)
            .ok_or(anyhow!("vtables not found"))
    }

    fn get_vtable_children_recursively(
        &self,
        binary_name: &str,
        target_name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<Vec<&VTableInfo>> {
        let mut children = Vec::new();
        for vtable in self.get_vtables(binary_name)? {
            if vtable
                .bases
                .iter()
                .any(|base| base.type_name.contains(target_name))
            {
                if vtable.type_name.eq(target_name) {
                    continue;
                }
                if visited.contains(&vtable.type_name) {
                    continue;
                }
                children.push(vtable);
                visited.insert(vtable.type_name.to_string());

                children.extend(self.get_vtable_children_recursively(
                    binary_name,
                    &vtable.type_name,
                    visited,
                )?);
            }
        }

        Ok(children)
    }

    pub fn get_all_vtable_children(
        &self,
        binary_name: &str,
        vtable_name: &str,
    ) -> Result<Vec<&VTableInfo>> {
        let mut visited = HashSet::new();
        let mut children =
            self.get_vtable_children_recursively(binary_name, vtable_name, &mut visited)?;
        children.sort_by(|a, b| a.type_name.cmp(&b.type_name));
        Ok(children)
    }

    pub fn get_object_ptr_vtable_info(&self, object_ptr: u64) -> Result<VTableInfo> {
        let vtable_ptr = unsafe { *(object_ptr as *const u64) };
        let memory_view = self.get_memory_view_from_ptr(vtable_ptr)?;

        if !memory_view.contains(vtable_ptr - 8) {
            bail!(
                "Vtable pointer {:X} is not in the memory view {:X}.",
                vtable_ptr - 8,
                memory_view.image_base()
            );
        };
        let rtti_ptr = memory_view.read::<u64>(vtable_ptr - 8);
        if rtti_ptr.is_none() {
            bail!("Failed to read rtti pointer.");
        }
        let rtti_ptr = rtti_ptr.unwrap();

        #[cfg(target_os = "windows")]
        {
            let mut parser = MsvcParser::new(&memory_view);
            let locator = parser.parse_locator(rtti_ptr);
            if locator.is_none() {
                bail!("Failed to parse locator.");
            }
            let locator = locator.unwrap();
            let vtable_info = parser.build_vtable_info(vtable_ptr, &locator);
            if vtable_info.is_none() {
                bail!("Failed to build vtable info.");
            }
            Ok(vtable_info.unwrap())
        }
        #[cfg(target_os = "linux")]
        {
            let mut parser = ItaniumParser::new(&memory_view);

            // TODO: get offset_to_top from rtti_ptr
            let vtable_info = parser.build_vtable_info(vtable_ptr, rtti_ptr, 0);
            if vtable_info.is_none() {
                bail!("Failed to build vtable info.");
            }
            Ok(vtable_info.unwrap())
        }
    }

    pub fn get_object_ptr_vtable_name(&self, object_ptr: u64) -> Result<String> {
        let vtable_info = self.get_object_ptr_vtable_info(object_ptr)?;
        Ok(vtable_info.type_name)
    }

    pub fn object_ptr_has_vtable(&self, object_ptr: u64) -> bool {
        self.get_object_ptr_vtable_info(object_ptr).is_ok()
    }

    pub fn object_ptr_has_base_class(
        &self,
        object_ptr: u64,
        base_class_name: &str,
    ) -> Result<bool> {
        let vtable_info = self.get_object_ptr_vtable_info(object_ptr)?;
        Ok(vtable_info
            .bases
            .iter()
            .any(|base| base.type_name.eq(base_class_name)))
    }
}

pub struct MsvcParser<'a, V: BinaryView<'a>> {
    view: &'a V,
    type_cache: HashMap<u64, String>,
    hierarchy_cache: HashMap<u64, (u32, Vec<BaseClassInfo>)>,
}

impl<'a, V: BinaryView<'a>> MsvcParser<'a, V> {
    pub fn new(view: &'a V) -> Self {
        Self {
            view,
            type_cache: HashMap::new(),
            hierarchy_cache: HashMap::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<VTableInfo>> {
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        for section in self.view.sections() {
            if !self.is_candidate_section(section) {
                continue;
            }

            let mut offset = 0usize;
            while offset + 8 <= section.len() {
                let locator_ptr = self
                    .view
                    .read::<u64>(section.address() + offset as u64)
                    .unwrap();
                if locator_ptr != 0 {
                    if let Some(locator) = self.parse_locator(locator_ptr) {
                        let vtable_va = section.address() + offset as u64 + 8;
                        if seen.insert(vtable_va) {
                            if let Some(info) = self.build_vtable_info(vtable_va, &locator) {
                                results.push(info);
                            }
                        }
                    }
                }
                offset += 8;
            }
        }

        results.sort_by_key(|entry| entry.vtable_address);
        Ok(results)
    }

    fn is_candidate_section(&self, section: &SectionInfo<'_>) -> bool {
        section.name().map_or(false, |name| {
            let lower = name.to_ascii_lowercase();
            lower.contains(".rdata") || lower.contains(".data")
        })
    }

    pub fn parse_locator(&self, locator_ptr: u64) -> Option<CompleteObjectLocator> {
        let signature: u32 = self.view.read(locator_ptr)?;
        if signature > 1 {
            return None;
        }

        let offset: u32 = self.view.read(locator_ptr + 4)?;
        let cd_offset: u32 = self.view.read(locator_ptr + 8)?;
        let type_descriptor_rva: u32 = self.view.read(locator_ptr + 12)?;
        let class_descriptor_rva: u32 = self.view.read(locator_ptr + 16)?;

        let type_descriptor = self.view.follow_rva(type_descriptor_rva)?;
        let class_descriptor = self.view.follow_rva(class_descriptor_rva)?;

        if !self.view.contains(type_descriptor) || !self.view.contains(class_descriptor) {
            return None;
        }

        Some(CompleteObjectLocator {
            address: locator_ptr,
            offset,
            cd_offset,
            type_descriptor,
            class_descriptor,
        })
    }

    pub fn build_vtable_info(
        &mut self,
        vtable_va: u64,
        locator: &CompleteObjectLocator,
    ) -> Option<VTableInfo> {
        let type_name = self.get_type_name(locator.type_descriptor)?;
        let (class_attributes, mut bases) = self.get_class_hierarchy(locator.class_descriptor)?;
        if !bases.is_empty() {
            bases.remove(0);
        }

        let methods = self.collect_methods(vtable_va);

        Some(VTableInfo {
            type_name,
            vtable_address: vtable_va,
            methods,
            bases,
            model: VTableModel::Msvc {
                complete_object_locator: locator.address,
                offset: locator.offset,
                constructor_displacement: locator.cd_offset,
                class_attributes,
            },
        })
    }

    pub fn get_type_name(&mut self, type_descriptor: u64) -> Option<String> {
        if let Some(name) = self.type_cache.get(&type_descriptor) {
            return Some(name.clone());
        }

        let name_va = type_descriptor.checked_add(16)?;
        let raw = self.view.read_c_string(name_va)?;
        let trimmed = raw.trim_start_matches(".");
        let wrapped = format!("??_R0{}@8", trimmed);
        let demangled = demangle(&wrapped, DemangleFlags::COMPLETE)
            .map(|s| s.to_string())
            .and_then(|s| {
                let splited = s.split_once(" ").unwrap().1;
                let splited = splited.rsplit_once("::").unwrap().0;
                Ok(splited.to_string())
            })
            .unwrap_or(wrapped);

        self.type_cache.insert(type_descriptor, demangled.clone());
        Some(demangled)
    }

    fn get_class_hierarchy(&mut self, class_descriptor: u64) -> Option<(u32, Vec<BaseClassInfo>)> {
        if let Some(cached) = self.hierarchy_cache.get(&class_descriptor) {
            return Some(cached.clone());
        }

        let signature: u32 = self.view.read(class_descriptor)?;
        if signature > 1 {
            return None;
        }

        let attributes: u32 = self.view.read(class_descriptor + 4)?;
        let base_count: u32 = self.view.read(class_descriptor + 8)?;
        let base_array_rva: u32 = self.view.read(class_descriptor + 12)?;

        if base_count == 0 {
            self.hierarchy_cache
                .insert(class_descriptor, (attributes, Vec::new()));
            return Some((attributes, Vec::new()));
        }

        let base_array = self.view.follow_rva(base_array_rva)?;

        let mut bases = Vec::new();
        for index in 0..base_count {
            let entry_va = base_array + (index as u64 * 4);
            let descriptor_rva: u32 = self.view.read(entry_va)?;
            let descriptor_va = self.view.follow_rva(descriptor_rva)?;
            if let Some(base) = self.parse_base_descriptor(descriptor_va) {
                bases.push(base);
            }
        }

        self.hierarchy_cache
            .insert(class_descriptor, (attributes, bases.clone()));

        Some((attributes, bases))
    }

    fn parse_base_descriptor(&mut self, addr: u64) -> Option<BaseClassInfo> {
        let type_descriptor_rva: u32 = self.view.read(addr)?;
        let type_descriptor = self.view.follow_rva(type_descriptor_rva)?;

        let num_contained_bases: u32 = self.view.read(addr + 4)?;
        let mdisp: i32 = self.view.read(addr + 8)?;
        let pdisp: i32 = self.view.read(addr + 12)?;
        let vdisp: i32 = self.view.read(addr + 16)?;
        let attributes: u32 = self.view.read(addr + 20)?;

        let type_name = self.get_type_name(type_descriptor)?;

        Some(BaseClassInfo {
            type_name,
            details: BaseClassModel::Msvc {
                attributes,
                displacement: Pmd {
                    mdisp,
                    pdisp,
                    vdisp,
                },
                num_contained_bases,
            },
        })
    }

    fn collect_methods(&self, mut vtable_va: u64) -> Vec<u64> {
        let mut methods = Vec::new();
        for _ in 0..1024 {
            let Some(method) = self.view.read::<u64>(vtable_va) else {
                break;
            };
            if method == 0 || !self.view.is_executable(method) {
                break;
            }
            methods.push(method);
            vtable_va += 8;
        }
        methods
    }
}

#[derive(Debug)]
pub struct CompleteObjectLocator {
    address: u64,
    offset: u32,
    cd_offset: u32,
    type_descriptor: u64,
    class_descriptor: u64,
}

pub struct ItaniumParser<'a, V: BinaryView<'a>> {
    view: &'a V,
    type_cache: HashMap<u64, TypeInfoData>,
    visiting: HashSet<u64>,
}

#[derive(Clone, Debug)]
pub struct TypeInfoData {
    name: String,
    bases: Vec<BaseClassInfo>,
}

impl<'a, V: BinaryView<'a>> ItaniumParser<'a, V> {
    pub fn new(view: &'a V) -> Self {
        Self {
            view,
            type_cache: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    pub fn parse(mut self) -> Result<Vec<VTableInfo>> {
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        for section in self.view.sections() {
            if !self.is_candidate_section(section) {
                continue;
            }

            let mut offset = 0usize;
            while offset + 24 <= section.len() {
                let typeinfo_ptr = self
                    .view
                    .read::<u64>(section.address() + offset as u64 + 8)
                    .unwrap();
                if typeinfo_ptr == 0 || !self.view.contains(typeinfo_ptr) {
                    offset += 8;
                    continue;
                }

                if self.resolve_typeinfo(typeinfo_ptr).is_none() {
                    offset += 8;
                    continue;
                }

                let first_method = self
                    .view
                    .read::<u64>(section.address() + offset as u64 + 16)
                    .unwrap_or(0);
                if first_method != 0 && !self.view.is_executable(first_method) {
                    offset += 8;
                    continue;
                }

                let vtable_va = section.address() + offset as u64 + 16;
                if !seen.insert(vtable_va) {
                    offset += 8;
                    continue;
                }

                let offset_to_top = self
                    .view
                    .read::<i64>(section.address() + offset as u64)
                    .unwrap();

                if let Some(info) = self.build_vtable_info(vtable_va, typeinfo_ptr, offset_to_top) {
                    results.push(info);
                }

                offset += 8;
            }
        }

        results.sort_by_key(|entry| entry.vtable_address);
        Ok(results)
    }

    fn is_candidate_section(&self, section: &SectionInfo<'_>) -> bool {
        section
            .name()
            .map(|name| {
                name.contains(".data.rel.ro") || name.contains(".data") || name.contains(".rodata")
            })
            .unwrap_or(false)
    }

    fn build_vtable_info(
        &mut self,
        vtable_va: u64,
        typeinfo_ptr: u64,
        offset_to_top: i64,
    ) -> Option<VTableInfo> {
        let info = self.resolve_typeinfo(typeinfo_ptr)?;
        let methods = self.collect_methods(vtable_va);
        let TypeInfoData { name, bases } = info;
        Some(VTableInfo {
            type_name: name,
            vtable_address: vtable_va,
            methods,
            bases: Self::flatten_bases(&bases),
            model: VTableModel::Itanium { offset_to_top },
        })
    }

    fn collect_methods(&self, mut vtable_va: u64) -> Vec<u64> {
        let mut methods = Vec::new();
        for _ in 0..1024 {
            let Some(method) = self.view.read::<u64>(vtable_va) else {
                break;
            };
            if method == 0 || !self.view.is_executable(method) {
                break;
            }
            methods.push(method);
            vtable_va += 8;
        }
        methods
    }

    fn resolve_typeinfo(&mut self, addr: u64) -> Option<TypeInfoData> {
        if let Some(cached) = self.type_cache.get(&addr) {
            return Some(cached.clone());
        }
        if !self.visiting.insert(addr) {
            return None;
        }

        let result = (|| {
            let (section_index, offset) = self.validate_typeinfo_header(addr)?;
            let raw_name = self.read_typeinfo_name(section_index, offset)?;
            let demangled_name = Symbol::new(&raw_name)
                .ok()
                .and_then(|s| s.demangle(&Default::default()).ok())
                .unwrap_or(raw_name);
            let bases = self.resolve_bases(section_index, offset);
            Some(TypeInfoData {
                name: demangled_name,
                bases,
            })
        })();

        self.visiting.remove(&addr);
        if let Some(data) = result {
            self.type_cache.insert(addr, data.clone());
            Some(data)
        } else {
            None
        }
    }

    fn resolve_bases(&mut self, section_index: usize, offset: u64) -> Vec<BaseClassInfo> {
        if let Some(section) = self.view.section_by_index(section_index) {
            if let Some(candidate) = self.view.read::<u64>(section.address() + offset + 16) {
                if candidate != 0 && self.looks_like_typeinfo(candidate) {
                    if let Some(base) = self.resolve_typeinfo(candidate) {
                        let TypeInfoData { name, bases } = base;
                        return vec![BaseClassInfo {
                            type_name: name,
                            details: BaseClassModel::Itanium {
                                offset: 0,
                                flags: 0,
                                bases,
                            },
                        }];
                    }
                    return Vec::new();
                }
            }
        }
        self.parse_vmi_typeinfo(section_index, offset)
    }

    /// Itanium RTTI only records direct base classes. Flatten the nested
    /// hierarchy in pre-order (direct base first, then its ancestors) so that
    /// `VTableInfo::bases` contains the full transitive base list, matching
    /// what the MSVC parser produces from the class hierarchy descriptor.
    /// The nested subtrees are stripped from the flattened entries to avoid
    /// duplicating the hierarchy at every level.
    fn flatten_bases(bases: &[BaseClassInfo]) -> Vec<BaseClassInfo> {
        let mut flattened = Vec::new();
        for base in bases {
            let mut flat = base.clone();
            let nested = match &mut flat.details {
                BaseClassModel::Itanium { bases, .. } => std::mem::take(bases),
                _ => Vec::new(),
            };
            flattened.push(flat);
            flattened.extend(Self::flatten_bases(&nested));
        }
        flattened
    }

    fn parse_vmi_typeinfo(&mut self, section_index: usize, offset: u64) -> Vec<BaseClassInfo> {
        let section = match self.view.section_by_index(section_index) {
            Some(section) => section,
            None => return Vec::new(),
        };
        let base_va = section.address() + offset;
        let base_count = self.view.read::<u32>(base_va + 20).unwrap_or(0);
        if base_count == 0 {
            return Vec::new();
        }

        let mut bases = Vec::new();
        let mut entry_va = base_va + 24;
        for _ in 0..base_count {
            let Some(typeinfo_ptr) = self.view.read::<u64>(entry_va) else {
                break;
            };
            let offset_flags = self.view.read::<i64>(entry_va + 8).unwrap_or(0);
            let offset = offset_flags >> 8;
            let flags = (offset_flags & 0xFF) as u32;

            if let Some(info) = self.resolve_typeinfo(typeinfo_ptr) {
                let TypeInfoData {
                    name,
                    bases: nested,
                } = info;
                bases.push(BaseClassInfo {
                    type_name: name,
                    details: BaseClassModel::Itanium {
                        offset,
                        flags,
                        bases: nested,
                    },
                });
            }

            entry_va += 16;
        }

        bases
    }

    fn looks_like_typeinfo(&self, addr: u64) -> bool {
        if let Some((section_index, offset)) = self.validate_typeinfo_header(addr) {
            return self.read_typeinfo_name(section_index, offset).is_some();
        }
        false
    }

    fn validate_typeinfo_header(&self, addr: u64) -> Option<(usize, u64)> {
        let (section_index, offset) = self.view.locate_address(addr)?;
        let section = self.view.section_by_index(section_index)?;
        if section.executable() {
            return None;
        }
        if offset & 0x7 != 0 {
            return None;
        }
        let offset_usize = usize::try_from(offset).ok()?;
        if offset_usize + 16 > section.len() {
            return None;
        }

        let vtable_ptr = self.view.read::<u64>(section.address() + offset)?;
        if vtable_ptr != 0 && !self.view.contains(vtable_ptr) {
            return None;
        }
        Some((section_index, offset))
    }

    fn read_typeinfo_name(&self, section_index: usize, offset: u64) -> Option<String> {
        let section = self.view.section_by_index(section_index)?;
        let name_ptr = self.view.read::<u64>(section.address() + offset + 8)?;
        if name_ptr == 0 || !self.view.contains(name_ptr) {
            return None;
        }
        let (name_section_index, _) = self.view.locate_address(name_ptr)?;
        let name_section = self.view.section_by_index(name_section_index)?;
        if name_section.executable() {
            return None;
        }
        let name = self.view.read_c_string(name_ptr)?;
        if !Self::typeinfo_name_is_plausible(&name) {
            return None;
        }
        Some(name)
    }

    fn typeinfo_name_is_plausible(name: &str) -> bool {
        if name.is_empty() || name.len() > 512 {
            return false;
        }
        if !name
            .chars()
            .all(|ch| ch.is_ascii() && !ch.is_ascii_control())
        {
            return false;
        }
        matches!(
            name.as_bytes().first(),
            Some(b'_') | Some(b'0'..=b'9') | Some(b'A'..=b'Z') | Some(b'a'..=b'z')
        )
    }
}
