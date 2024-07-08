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
                "../protos/notification/messages.proto",
                "../protos/notification/rpc.proto",
            ],
            &["../protos"],
        )
        .unwrap();

    Ok(())
}
