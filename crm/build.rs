use std::fs;

use anyhow::Result;

fn main() -> Result<()> {
    let path = "src/pb";
    fs::create_dir_all(path)?;

    tonic_build::configure()
        .out_dir(path)
        .compile(&["../protos/crm/crm.proto"], &["../protos"])?;

    Ok(())
}
