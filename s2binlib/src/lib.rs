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

mod flags;
pub mod jit;
mod memory;
mod module;
mod pattern;
mod s2binlib;
mod view;
mod vtable;

pub use flags::*;
pub use pattern::*;
pub use s2binlib::*;
pub use vtable::*;

#[cfg(test)]
#[allow(unused_imports, unused_variables)]
mod tests {
    use std::{
        fs::{self, File},
        io::{BufWriter, Write},
        time::Instant,
    };

    use anyhow::Result;
    use iced_x86::{Code, Decoder, DecoderOptions, Mnemonic, OpKind};
    use object::BinaryFormat;

    use crate::{
        module::get_module_info,
        view::{BinaryView, FileBinaryView, MemoryView},
    };

    use super::*;

    #[test]
    fn test_find_vtable() -> Result<()> {
        let mut s2binlib = S2BinLib::new("F:/cs2server/cs2/game", "csgo", "linux");
        s2binlib.load_binary("server");
        // s2binlib.load_binary("engine2");

        let server_vtables = vec![
            "CBaseEntity",
            "CPlayer_MovementServices",
            "CSource2Server",
            "CGameEventManager",
            "CGameRulesGameSystem",
            "CSource2GameClients"
        ];

        let engine2_vtables = vec![
            "CLoopModeLevelLoad",
            "CServerSideClient",
            "CNetworkServerService",
            "CGameEventSystem"
        ];

        for vtable in server_vtables {
            let vtable = s2binlib.find_vtable_rva("server", vtable)?;
            println!("{:X}", vtable);
        }

        // for vtable in engine2_vtables {
        //     let vtable = s2binlib.find_vtable_rva("engine2", vtable)?;
        //     println!("{:X}", vtable);
        // }

        Ok(())
    }

    #[test]
    fn test_s2binlib() -> Result<()> {
        // fs::write("funcs.txt", serde_json::to_string_pretty(&funcs)?)?;

        let start = Instant::now();

        let mut s2binlib = S2BinLib::new("F:/cs2server/cs2/game", "csgo", "windows");

        s2binlib.load_binary("server");

        Ok(())
    }
}
