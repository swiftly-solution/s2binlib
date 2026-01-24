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
use log::info;
use s2binlib::S2BinLib;

pub fn dump_gamesystems(s2binlib: &S2BinLib, dump_dir: &str, binary: &str) -> Result<()> {
    let names = vec![
        "Init",                          // 0
        "PostInit",                      // 1
        "Shutdown",                      // 2
        "GameInit",                      // 3
        "GameShutdown",                  // 4
        "GamePostInit",                  // 5
        "GamePreShutdown",               // 6
        "BuildGameSessionManifest",      // 7
        "GameActivate",                  // 8
        "ClientFullySignedOn",           // 9
        "Disconnect",                    // 10
        "unk_001",                       // 11
        "GameDeactivate",                // 12
        "SpawnGroupPrecache",            // 13
        "SpawnGroupUncache",             // 14
        "PreSpawnGroupLoad",             // 15
        "PostSpawnGroupLoad",            // 16
        "PreSpawnGroupUnload",           // 17
        "PostSpawnGroupUnload",          // 18
        "ActiveSpawnGroupChanged",       // 19
        "ClientPostDataUpdate",          // 20
        "ClientPreRender",               // 21
        "ClientPreEntityThink",          // 22
        "unk_101",                       // 23
        "unk_102",                       // 24
        "ClientPollNetworking",          // 25
        "unk_201",                       // 26
        "ClientUpdate",                  // 27
        "unk_301",                       // 28
        "ClientPostRender",              // 29
        "ServerPreEntityThink",          // 30
        "ServerPostEntityThink",         // 31
        "unk_401",                       // 32
        "ServerPreClientUpdate",         // 33
        "ServerAdvanceTick",             // 34
        "ClientAdvanceTick",             // 35
        "ServerGamePostSimulate",        // 36
        "ClientGamePostSimulate",        // 37
        "ServerPostAdvanceTick",         // 38
        "ServerBeginAsyncPostTickWork",  // 39
        "unk_501",                       // 40
        "ServerEndAsyncPostTickWork",    // 41
        "ClientFrameSimulate",           // 42
        "ClientPauseSimulate",           // 43
        "ClientAdvanceNonRenderedFrame", // 44
        "GameFrameBoundary",             // 45
        "OutOfGameFrameBoundary",        // 46
        "SaveGame",                      // 47
        "RestoreGame",                   // 48
        "unk_601",                       // 49
        "unk_602",                       // 50
        "unk_603",                       // 51
        "unk_604",                       // 52
        "unk_605",                       // 53
        "unk_606",                       // 54
        "GetName",                       // 55
        "SetGameSystemGlobalPtrs",       // 56
        "SetName",                       // 57
        "DoesGameSystemReallocate",      // 58
        "~IGameSystem",                  // 59
        "~IGameSystem2",                 // 60
        "Preserved1",
        "Preserved2",
        "Preserved3",
        "Preserved4",
        "Preserved5",
        "Preserved6",
        "Preserved7",
        "Preserved8",
        "Preserved9",
        "Preserved10",
    ];
    let max_size = names.len();

    let mut funcs = vec![vec![]; max_size];

    let size = s2binlib.get_vtable_vfunc_count(binary, "IGameSystem")?;

    let res = s2binlib.get_vtables(binary)?;

    for vtable in res {
        for base in &vtable.bases {
            if base.type_name.contains("IGameSystem") {
                if vtable.methods.len() < size {
                    continue;
                }
                info!("Dumping {}", vtable.type_name);
                for i in 0..size {
                    let method = vtable.methods[i];
                    if !s2binlib.is_nullsub_rva(binary, method)? {
                        funcs[i].push(vtable.type_name.clone());
                    }
                }
            }
        }
    }

    let mut file = File::create(format!("{}/gamesystem_{}.txt", dump_dir, binary))?;
    for i in 0..max_size {
        writeln!(
            file,
            "{:<35} {}",
            format!("{} [{}]", names[i], i),
            format!("{:?}", funcs[i])
        )?;
    }
    file.flush()?;

    Ok(())
}
