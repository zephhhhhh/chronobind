fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=src/proto/productdb.proto");

    prost_build::compile_protos(&["src/proto/productdb.proto"], &["src/proto"])?;

    Ok(())
}
