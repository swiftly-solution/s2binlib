use anyhow::Result;
use s2binlib::S2BinLib;

fn main() -> Result<()> {

    let mut s2binlib = S2BinLib::new("F:/cs2server/cs2/game", "csgo", "linux");

    s2binlib.load_binary("server");
    s2binlib.dump_xrefs("server");
    println!("123");
    // s2binlib.dump_vtables("server");

    println!("{}", s2binlib.make_sig_rva("server", 0x1856AD0)?);

    Ok(())

}
