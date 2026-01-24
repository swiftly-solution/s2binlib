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

use std::fs;

use anyhow::Result;
use clap::{Parser, arg, command};
use log::info;
use s2binlib::S2BinLib;
use stringvec::stringvec;

mod dumpers {
    pub mod diff_dumper;
    pub mod entity_dumper;
    pub mod gamesystem_dumper;
    pub mod networkvar_dumper;
    pub mod strings_dumper;
    pub mod vtable_dumper;
}

#[derive(Parser)]
#[command(name = "s2binlib_dumper")]
#[command(about = "A dumper for Source 2 game engine", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        help = "Linux binary directory, should ends with 'game' folder."
    )]
    linux_path: Option<String>,

    #[arg(
        short,
        long,
        help = "Windows binary directory, should ends with 'game' folder."
    )]
    windows_path: Option<String>,

    #[arg(short, long, default_value = "./dump", help = "Output directory.")]
    output_path: Option<String>,

    #[arg(
        short,
        long,
        default_value = "2",
        help = "0: windows, 1: linux, 2: both"
    )]
    binary: Option<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let linux_path = args.linux_path.unwrap_or("F:/cs2server/game".to_string());
    let windows_path = args.windows_path.unwrap_or("F:/cs2server/game".to_string());

    tracing_subscriber::fmt::init();
    let tracked_binaries = stringvec![
        "server",
        "engine2",
        "tier0",
        "client",
        "networksystem",
        "soundsystem",
        "pulse_system",
        "schemasystem",
        "materialsystem2",
    ];

    let os = match args.binary {
        Some(0) => vec!["windows"],
        Some(1) => vec!["linux"],
        Some(2) => vec!["windows", "linux"],
        _ => unreachable!(),
    };

    for os in os {
        let mut s2binlib = S2BinLib::new(
            if os == "windows" {
                &windows_path
            } else {
                &linux_path
            },
            "csgo",
            os,
        );
        for binary in &tracked_binaries {
            info!("Initializing {}", binary);
            s2binlib.load_binary(&binary);
            s2binlib.dump_vtables(&binary)?;
            s2binlib.dump_strings(&binary)?;
        }

        let dump_dir = format!(
            "{}/{}",
            args.output_path.clone().unwrap_or("dump".to_string()),
            s2binlib.get_os()
        );
        if fs::exists(&dump_dir)? {
            fs::remove_dir_all(&dump_dir)?;
        }
        fs::create_dir_all(&dump_dir)?;

        dumpers::diff_dumper::dump_diffs(&s2binlib, &dump_dir)?;
        dumpers::entity_dumper::dump_entities_server(&s2binlib, &dump_dir)?;
        dumpers::entity_dumper::dump_entities_client(&s2binlib, &dump_dir)?;
        dumpers::gamesystem_dumper::dump_gamesystems(&s2binlib, &dump_dir, "server")?;
        dumpers::gamesystem_dumper::dump_gamesystems(&s2binlib, &dump_dir, "client")?;
        dumpers::vtable_dumper::dump_vtables(&s2binlib, &tracked_binaries, &dump_dir)?;
        dumpers::networkvar_dumper::dump_networkvars(&s2binlib, &dump_dir)?;
        dumpers::strings_dumper::dump_strings(&s2binlib, &dump_dir, &tracked_binaries)?;
    }

    Ok(())
}
