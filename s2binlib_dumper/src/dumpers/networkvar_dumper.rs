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

use std::{fs::File, io::Write};

use anyhow::Result;
use log::{info, warn};
use s2binlib::S2BinLib;

pub fn dump_networkvars(s2binlib: &S2BinLib, dump_dir: &str) -> Result<()> {
    let vtables = s2binlib.get_vtables("server")?;

    let mut file = File::create(format!("{}/networkvars.txt", dump_dir))?;

    for vtable in vtables {
        if vtable.type_name.contains("NetworkVar_")
            && !vtable.type_name.starts_with("CUtlVectorDataOps")
        {
            let index = s2binlib.find_networkvar_vtable_statechanged_rva(vtable.vtable_address);

            if let Err(e) = index {
                warn!(
                    "Error finding statechanged index for {}: {}",
                    vtable.type_name, e
                );
                continue;
            }

            info!("Dumping {}", vtable.type_name);
            let index = index.unwrap();
            let vfunc_count = vtable.methods.len();
            let statechanged_index = index as usize;
            let statechanged_func = vtable.methods[statechanged_index];
            writeln!(file, "{:<50} [{}]", vtable.type_name, vfunc_count)?;
            writeln!(
                file,
                "  StateChanged: [ {} -> {:X} ]",
                statechanged_index, statechanged_func
            )?;
        }
    }

    file.flush()?;

    Ok(())
}
