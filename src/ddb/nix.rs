use crate::ddb::REGISTRY;

use super::Store;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;

#[derive(Deserialize, Debug, Clone)]
struct Package {
    #[serde(rename = "storePaths")]
    outputs: HashMap<String, String>,
    version: Option<String>,
}

pub async fn get_store(rev: &str) -> HashMap<String, Store> {
    let nixpath = Command::new("nix-instantiate")
        .arg("--eval")
        .arg("-E")
        .arg(format!("with import <nixpkgs> {{}}; pkgs.path"))
        .arg("-I")
        .arg(format!(
            "nixpkgs=https://github.com/NixOS/nixpkgs/archive/{}.tar.gz",
            rev
        ))
        .output()
        .await
        .expect("failed to execute process");
    if !nixpath.status.success() {
        error!(
            "nix-instantiate failed: {}",
            String::from_utf8_lossy(&nixpath.stderr)
        );
        std::process::exit(1);
    }
    let nixpath = String::from_utf8_lossy(&nixpath.stdout).trim().to_string();
    debug!("nixpath: {}", nixpath);

    let mut output = Command::new("nix-instantiate")
        .env("NIXPKGS_ALLOW_UNFREE", "1")
        .env("NIXPKGS_ALLOW_INSECURE", "1")
        // .env("NIXPKGS_ALLOW_BROKEN", "0")
        // .env("NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM", "0")
        .arg("--eval")
        .arg("-E")
        .arg(format!("with import {nixpath} {{ config = import {nixpath}/pkgs/top-level/packages-config.nix; }}; (import {REGISTRY} {{ inherit lib; }}).genRegistry \"x86_64-linux\" pkgs"))
        .arg("-I")
        .arg(format!("nixpkgs={}", nixpath))
        .arg("--json")
        .arg("--strict")
        .output()
        .await
        .expect("failed to execute process");

    if !output.status.success() {
        warn!("nix-instantiate failed, falling back to default nixpkgs config");
        output = Command::new("nix-instantiate")
        .env("NIXPKGS_ALLOW_UNFREE", "1")
        .env("NIXPKGS_ALLOW_INSECURE", "1")
        // .env("NIXPKGS_ALLOW_BROKEN", "0")
        // .env("NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM", "0")
        .arg("--eval")
        .arg("-E")
        .arg(format!("with import {nixpath} {{ config = {{ allowAliases = false; }}; }}; (import {REGISTRY} {{ inherit lib; }}).genRegistry \"x86_64-linux\" pkgs"))
        .arg("-I")
        .arg(format!("nixpkgs={}", nixpath))
        .arg("--json")
        .arg("--strict")
        .output()
        .await
        .expect("failed to execute process");
    }

    let output: HashMap<String, Package> =
        serde_json::from_slice(&output.stdout).expect("failed to parse nix-instantiate output");

    info!("nix-instantiate: got {} packages", output.len());

    let mut store: HashMap<String, Store> = HashMap::new();
    for (attr, pkg) in &output {
        if let Some(outpath) = pkg.outputs.get("out") {
            if let Some(store_val) = store.get_mut(outpath) {
                store_val.attribute.push(attr.to_string());
            } else {
                store.insert(
                    outpath.to_string(),
                    Store {
                        attribute: vec![attr.to_string()],
                        version: pkg.version.clone(),
                    },
                );
            }
        }
    }

    info!("nix-instantiate: got {} store paths", store.len());

    return store;
}
