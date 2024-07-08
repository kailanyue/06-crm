use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    let path = "src/pb";
    fs::create_dir_all(path)?;
    let builder = tonic_build::configure();

    builder
        .out_dir(path)
        .compile(
            &[
                "../protos/metadata/messages.proto",
                "../protos/metadata/rpc.proto",
            ],
            &["../protos"],
        )
        .unwrap();

    Ok(())
}
