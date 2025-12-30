mod productdb {
    include!(concat!(env!("OUT_DIR"), "/productdb.rs"));
}

const TEST_PROTO_SOURCE: &[u8] = include_bytes!("../product.db");

/// Get the product database from the Battle.net agent 'product.db' file, used to find
/// the install location of World of Warcraft.
/// # Errors
/// This function will return an error if the 'product.db' file cannot be decoded.
pub fn get_product_db() -> Result<productdb::Database, prost::DecodeError> {
    todo!();
}
