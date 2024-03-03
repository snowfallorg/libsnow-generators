// https://releases.nixos.org/nixos/unstable/nixos-24.05pre564493.b0d36bd0a420/packages.json.br

use std::collections::HashMap;
use anyhow::Result;
use serde::Deserialize;
use crate::{MetaData, Pkg};

#[derive(Deserialize, Debug, Clone)]
struct Package {
    meta: Option<MetaData>,
    pname: String,
    version: String,
}

#[derive(Deserialize, Debug)]
struct PkgJson {
    packages: HashMap<String, Package>,
}
pub async fn getmeta(channel: &str, rev: &str) -> Result<HashMap<String, Pkg>> {

    let url = format!("https://releases.nixos.org/{}/{}/packages.json.br", channel, rev);

    // reqwest brotli decompression
    let client = reqwest::Client::builder()
        .brotli(true)
        .build()?;
    let output = client.get(&url).send().await?.bytes().await?;

    let output: PkgJson = serde_json::from_slice(&output).unwrap();

    let data = output
        .packages
        .iter()
        .map(|(attr, pkg)| {
            let metadata = pkg.meta.as_ref().unwrap().clone();
            let store = Pkg {
                attribute: attr.to_string(),
                // store: outpath.split("/").last().unwrap().to_string(),
                meta: metadata,
                pname: pkg.pname.clone(),
                version: pkg.version.clone(),
            };
            (attr.to_string(), store)
        })
        .collect::<HashMap<String, Pkg>>();

    return Ok(data);
}

pub async fn getrevision(channel: &str, rev: &str) -> Result<String> {
    let url = format!("https://releases.nixos.org/{}/{}/git-revision", channel, rev);
    let output = reqwest::get(&url).await?.text().await?;
    Ok(output)
}
