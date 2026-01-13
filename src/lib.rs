pub mod data;
pub mod equipment;
pub mod error;
pub mod model;
pub mod protocol;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
