use serde::{Serialize, Deserialize};
use serde_json::Value;

pub mod revisions;
pub mod s3;
pub mod ddb;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Pkg {
    pub attribute: String,
    // pub store: String,
    pub meta: MetaData,
    pub pname: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct MetaData {
    // https://github.com/NixOS/nixpkgs/blob/master/doc/stdenv/meta.chapter.md
    pub description: Option<String>,
    #[serde(rename = "longDescription")]
    pub long_description: Option<String>,
    pub branch: Option<String>,
    pub homepage: Option<Value>,
    #[serde(rename = "downloadPage")]
    pub download_page: Option<Value>,
    pub changelog: Option<Value>,
    pub license: Option<Value>,
    pub maintainers: Option<Value>,
    #[serde(rename = "mainProgram")]
    pub main_program: Option<String>,
    pub platforms: Option<Value>,
    #[serde(rename = "badPlatforms")]
    pub bad_platforms: Option<Value>,
    pub broken: Option<bool>,
    pub unfree: Option<bool>,
    pub insecure: Option<bool>,
}
