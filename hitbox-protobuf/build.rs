fn main() -> Result<(), Box<dyn std::error::Error>> {
    use prost::Message;

    let descriptor_path =
        std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("test_descriptor.bin");

    let fds = protox::compile(["proto/test.proto"], ["proto/"])?;
    std::fs::write(&descriptor_path, fds.encode_to_vec())?;

    // Make the descriptor path available to integration tests via env!()
    println!(
        "cargo:rustc-env=TEST_DESCRIPTOR_PATH={}",
        descriptor_path.display()
    );
    println!("cargo:rerun-if-changed=proto/test.proto");
    Ok(())
}
