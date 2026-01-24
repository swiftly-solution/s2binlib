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

use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use s2binlib::{S2BinLib, VTableInfo};

fn dump_entities_internal(
    s2binlib: &S2BinLib,
    dump_dir: &str,
    binary_name: &str,
    class_name: &str,
) -> Result<()> {
    fs::create_dir_all(format!("{}/entity_diff/{}", dump_dir, binary_name))?;

    let vtables = s2binlib.get_vtables(binary_name)?;

    let base_count = s2binlib.get_vtable_vfunc_count(binary_name, class_name)?;

    let mut entity_classes_file =
        File::create(format!("{}/entity_classes_{}.txt", dump_dir, binary_name))?;

    let mut base_infos: HashMap<String, &VTableInfo> = HashMap::new();

    for vtable in vtables {
        if vtable
            .bases
            .iter()
            .any(|base| base.type_name.contains(class_name))
        {
            if vtable.methods.len() < base_count {
                continue; // multiple hierarchy vtable
            }

            let mut final_base_class = String::from(class_name);
            for base_class_info in &vtable.bases {
                let base_class = base_class_info.type_name.clone();
                if !base_infos.contains_key(&base_class) {
                    let base_info = vtables
                        .iter()
                        .find(|vtable| vtable.type_name.eq(&base_class));
                    if let Some(base_info) = base_info {
                        if base_info.methods.len() < base_count {
                            continue;
                        }
                        final_base_class = base_class.clone();
                        base_infos.insert(base_class.clone(), base_info);
                        break;
                    }
                }
            }
            let base_info = base_infos.get(&final_base_class).unwrap();

            writeln!(
                entity_classes_file,
                "{:<50} [{}]",
                vtable.type_name,
                vtable.methods.len()
            )?;

            let mut file = File::create(format!(
                "{}/entity_diff/{}/{}.txt",
                dump_dir, binary_name, vtable.type_name
            ))?;
            writeln!(
                file,
                "{:<15} VS {:>15}",
                base_info.type_name, vtable.type_name
            )?;
            for i in 0..vtable.methods.len() {
                if i < base_info.methods.len() {
                    let method = vtable.methods[i];
                    let base_method = base_info.methods[i];
                    if method == base_method {
                        writeln!(file, "[=]")?;
                    } else {
                        writeln!(file, "[!]")?;
                    }
                } else {
                    writeln!(file, "[?]")?;
                }
            }
        }
    }

    entity_classes_file.flush()?;

    Ok(())
}

pub fn dump_entities_server(s2binlib: &S2BinLib, dump_dir: &str) -> Result<()> {
    dump_entities_internal(s2binlib, dump_dir, "server", "CBaseEntity")
}

pub fn dump_entities_client(s2binlib: &S2BinLib, dump_dir: &str) -> Result<()> {
    dump_entities_internal(s2binlib, dump_dir, "client", "C_BaseEntity")
}
