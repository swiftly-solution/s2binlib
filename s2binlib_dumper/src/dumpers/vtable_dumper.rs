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
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use log::info;
use s2binlib::S2BinLib;

pub fn dump_vtables(
    s2binlib: &S2BinLib,
    tracked_binaries: &[String],
    dump_dir: &str,
) -> Result<()> {
    for binary in tracked_binaries {
        info!("Dumping vtables for {}", binary);
        let vtables = s2binlib.get_vtables(binary)?;
        fs::create_dir_all(format!("{}/vtables", dump_dir))?;
        let file = File::create(format!("{}/vtables/{}.json", dump_dir, binary))?;
        serde_json::to_writer_pretty(file, &vtables)?;

        let mut file = File::create(format!("{}/vtables/{}_short.txt", dump_dir, binary))?;

        let mut map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

        for vtable in vtables {
            if map.contains_key(&vtable.type_name) {
                map.get_mut(&vtable.type_name)
                    .unwrap()
                    .push(vtable.methods.len());
            } else {
                map.insert(vtable.type_name.clone(), vec![vtable.methods.len()]);
            }
        }

        for (type_name, methods_len) in map {
            write!(file, "{:<50}", type_name)?;
            for method_len in methods_len {
                write!(file, " [{}]", method_len)?;
            }
            writeln!(file)?;
        }
        file.flush()?;
    }

    Ok(())
}
