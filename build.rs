fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file
    tonic_build::compile_protos("proto/kv_store.proto")?;
    Ok(())
}
