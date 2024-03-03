use super::Store;
use log::{debug, info};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;

#[derive(Deserialize, Debug, Clone)]
struct Package {
    outputs: Option<HashMap<String, String>>,
}

//nix-env -qa --meta --json --out-path -f https://github.com/NixOS/nixpkgs/archive/5f64a12a728902226210bf01d25ec6cbb9d9265b.tar.gz
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
    let nixpath = String::from_utf8_lossy(&nixpath.stdout).trim().to_string();
    debug!("nixpath: {}", nixpath);

    let output = Command::new("nix-env")
        .env("NIXPKGS_ALLOW_UNFREE", "1")
        .env("NIXPKGS_ALLOW_INSECURE", "1")
        // .env("NIXPKGS_ALLOW_BROKEN", "0")
        // .env("NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM", "0")
        .arg("-f")
        .arg(&nixpath)
        .arg("-I")
        .arg(format!("nixpkgs={}", nixpath))
        .arg("-qa")
        // .arg("--meta")
        .arg("--json")
        .arg("--out-path")
        .arg("--arg")
        .arg("config")
        .arg(format!(
            "import {}/pkgs/top-level/packages-config.nix",
            nixpath
        ))
        .output()
        .await
        .expect("failed to execute process");

    let output: HashMap<String, Package> = serde_json::from_slice(&output.stdout).expect("failed to parse nix-env output");

    info!("nix-env: got {} packages", output.len());

    let store = output
        .iter()
        .filter_map(|(attr, pkg)| {
            if let Some(outpath) = pkg.outputs.as_ref().and_then(|x| x.get("out")) {
                let store = Store {
                    attribute: attr.to_string(),
                    store: outpath.split("/").last().unwrap().to_string(),
                };

                Some((attr.to_string(), store))
            } else {
                None
            }
        })
        .collect::<HashMap<String, Store>>();

    info!("nix-env: got {} store paths", store.len());

    return store;
}
