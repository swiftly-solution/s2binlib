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

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;

use anyhow::Result;
use log::info;
use s2binlib::{BaseClassModel, S2BinLib};
use stringvec::stringvec;

pub fn dump_diff(
    s2binlib: &S2BinLib,
    dump_dir: &str,
    binary_name: &str,
    base_class: &str,
) -> Result<()> {
    info!("Dumping diff for {} in {}", base_class, binary_name);

    fs::create_dir_all(format!("{}/vtable_diff/{}", dump_dir, binary_name))?;
    let dump_file = format!(
        "{}/vtable_diff/{}/{}.txt",
        dump_dir, binary_name, base_class
    );

    let vtables = s2binlib.get_vtables(binary_name)?;
    let base_info = vtables
        .iter()
        .find(|vtable| vtable.type_name.eq(base_class))
        .unwrap();
    let base_count = base_info.methods.len();
    let mut file = File::create(dump_file)?;

    let mut diffs = vec![vec![]; base_count];
    let mut children = vec![];
    let mut bytes = vec![];

    for method in &base_info.methods {
        bytes.push(s2binlib.read_by_rva(binary_name, *method, 16)?.to_vec());
    }

    for vtable in s2binlib.get_all_vtable_children(binary_name, base_class)? {
        if vtable.methods.len() < base_count {
            continue; // multiple hierarchy vtable
        }
        children.push(vtable.type_name.clone());

        for i in 0..vtable.methods.len() {
            if i < base_info.methods.len() {
                let method = vtable.methods[i];
                let base_method = base_info.methods[i];
                if method != base_method {
                    diffs[i].push(vtable.type_name.clone());
                }
            }
        }
    }

    writeln!(file, "Vtable: {}", base_class)?;
    writeln!(file, "Virtual Function Count: {}", base_count)?;
    writeln!(file, "Children Count: {}", children.iter().count())?;
    writeln!(file, "=============================================")?;
    for i in 0..diffs.len() {
        write!(
            file,
            "{:<10} [ DIFF {:<3} ] {:>30}",
            format!("func_{}", i),
            diffs[i].iter().count(),
            ""
        )?;
        writeln!(file, "{:?}", diffs[i])?;
        writeln!(
            file,
            "    {:?}",
            bytes[i]
                .iter()
                .map(|x| format!("{:02X}", x))
                .collect::<Vec<String>>()
                .join(" ")
        )?;
    }
    writeln!(file)?;
    writeln!(file, "=============================================")?;
    writeln!(file, "Bases: ")?;
    for base in &base_info.bases {
        writeln!(file, "{:?}", base.type_name)?;

        match &base.details {
            BaseClassModel::Msvc {
                attributes,
                displacement,
                num_contained_bases,
            } => {
                writeln!(file, "    Attributes: {:?}", attributes)?;
                writeln!(file, "    Displacement: {:?}", displacement)?;
                writeln!(file, "    Num Contained Bases: {:?}", num_contained_bases)?;
            }
            BaseClassModel::Itanium {
                offset,
                flags,
                bases,
            } => {
                writeln!(file, "    Offset: {:?}", offset)?;
                writeln!(file, "    Flags: {:?}", flags)?;
                if !bases.is_empty() {
                    writeln!(file, "    Nested Bases:")?;
                    for nested in bases {
                        writeln!(file, "        {:?}", nested.type_name)?;
                    }
                }
            }
        }
    }
    writeln!(file, "=============================================")?;
    writeln!(file)?;
    writeln!(file, "=============================================")?;
    writeln!(file, "Children: ")?;
    for child in children {
        writeln!(file, "{:?}", child)?;
    }
    writeln!(file, "================================================")?;
    file.flush()?;

    Ok(())
}

pub fn dump_diffs(s2binlib: &S2BinLib, dump_dir: &str) -> Result<()> {
    let vtables = HashMap::from([
        (
            "server",
            stringvec![
                "CBaseEntity",
                "CBaseModelEntity",
                "CCSWeaponBase",
                "IGameSystem",
                "CTraceFilter",
                "CRecipientFilter",
                "CCSPlayerController",
                "CCSPlayer_WeaponServices",
                "CCSPlayer_MovementServices",
                "CGameSceneNode",
                "CCSGameRules",
                "CCSPlayer_ItemServices",
                "CSource2GameClients",
                "CSource2GameEntities",
                "CGameEventManager",
                "CLoopModeGame",
                "CGameEventListener",
                "CGameEntitySystem",
                "CSource2Server",
                "CLoadingSpawnGroup",
                "CTraceFilter",
                "CRecipientFilter",
                "CGameEvent"
            ],
        ),
        (
            "engine2",
            stringvec![
                "CEntityResourceManifest",
                "CServerSideClient",
                "CNetworkServerService",
                "CEngineServiceMgr",
                "CEngineClient",
                "CEngineServer"
            ],
        ),
        ("tier0", stringvec!["CCvar",]),
        (
            "networksystem",
            stringvec!["CNetworkMessages", "CNetworkSerializerPB", "CNetworkSystem"],
        ),
        ("materialsystem2", stringvec!["CMaterialSystem2"]),
        ("schemasystem", stringvec!["CSchemaSystem", "CSchemaType"]),
        (
            "client",
            stringvec![
                "C_BaseEntity",
                "C_BaseModelEntity",
                "C_CSWeaponBase",
                "IGameSystem",
            ],
        ),
    ]);

    for (binary_name, vtables) in vtables {
        for vtable in vtables {
            dump_diff(s2binlib, dump_dir, binary_name, &vtable)?;
        }
    }

    Ok(())
}
