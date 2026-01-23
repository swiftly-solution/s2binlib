use anyhow::Result;
use s2binlib::S2BinLib;

fn main() -> Result<()> {

    let mut s2binlib = S2BinLib::new("F:/cs2server/cs2/game", "csgo", "linuxx");

    s2binlib.load_binary("server");

    println!("{:X}", s2binlib.find_vtable_rva("server", "CPlayer_MovementServices")?);

    Ok(())

}
