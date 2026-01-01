fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=proto/productdb.proto");

    prost_build::compile_protos(&["proto/productdb.proto"], &["proto"])?;

    Ok(())
}
