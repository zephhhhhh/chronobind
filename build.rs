fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=proto/productdb.proto");

    prost_build::compile_protos(&["proto/productdb.proto"], &["proto"])?;

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/chronobind.ico");
        res.compile().unwrap();
    }

    Ok(())
}
