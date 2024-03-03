use serde::{Deserialize, Serialize};

pub mod nix;
pub mod batch_put;


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Store {
    pub attribute: String,
    pub store: String,
    pub version: Option<String>,
}

pub const REGISTRY: &str = "./registry.nix";
