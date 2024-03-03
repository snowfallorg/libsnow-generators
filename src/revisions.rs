use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, fs, io::Write};

#[derive(Debug, Deserialize)]
struct ListBucketResult {
    #[serde(rename = "Contents", default)]
    contents: Vec<Content>,
    #[serde(rename = "IsTruncated")]
    is_truncated: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct Content {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
}

pub async fn get_revisions(dir: &str) -> Result<HashMap<String, Vec<String>>> {
    let mut out = HashMap::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.ends_with("nixos") {
            let channel = path
                .file_name()
                .context("Failed to read file name")?
                .to_str()
                .context("Failed to get channel name")?
                .to_string();

            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let release = path
                        .file_name()
                        .context("Failed to read file name")?
                        .to_str()
                        .context("Failed to get release name")?
                        .to_string();

                    let last = std::fs::read_to_string(format!("{}/last", path.to_string_lossy()))
                        .context("Failed to read last file")?;

                    let revs = get_all_objects(&format!("{}/{}", channel, release), &last).await?;

                    out.insert(format!("{}/{}", channel, release), revs);
                }
            }
        } else if path.is_dir() && path.ends_with("nixpkgs") {
            let channel = path
                .file_name()
                .context("Failed to read file name")?
                .to_str()
                .context("Failed to get release name")?
                .to_string();

            let last = std::fs::read_to_string(format!("{}/last", path.to_string_lossy()))
                .context("Failed to read last file")?;

            let revs = get_all_objects(&channel, &last).await?;

            out.insert(channel, revs);
        }
    }

    return Ok(out);
}

async fn get_all_objects(channel: &str, last_key: &str) -> Result<Vec<String>> {
    let mut truncated = true;
    let mut marker = "".to_string();
    let url = format!(
        "https://nix-releases.s3.amazonaws.com/?delimiter=/&prefix={}/",
        channel
    );

    let mut objects = vec![];

    while truncated {
        let output = reqwest::get(format!("{}&marker={}", url, marker))
            .await?
            .text()
            .await?;

        let output: ListBucketResult = quick_xml::de::from_str(&output)?;

        for content in &output.contents {
            objects.push(content.clone());
        }

        truncated = output.is_truncated;
        marker = output
            .contents
            .last()
            .context("Failed to get last item")?
            .key
            .clone();
    }

    objects.sort_by(|a, b| a.last_modified.cmp(&b.last_modified));

    // Remove everything before last, inclusive
    if let Some(last) = objects
        .iter()
        .position(|x| x.key.trim_matches('"').split('/').last().unwrap() == last_key)
    {
        objects.drain(0..last + 1);
    }

    let revs = objects
        .into_iter()
        .filter_map(|x| x.key.split('/').last().map(|x| x.to_string()))
        .collect::<Vec<_>>();

    Ok(revs)
}

pub fn update_markers(dir: &str, revs: HashMap<String, Vec<String>>) -> Result<()> {
    for (channel, revs) in revs {
        if revs.is_empty() {
            continue;
        } else if let Some(last) = revs.last() {
            let mut file = fs::File::create(format!("{}/{}/last", dir, channel))?;
            file.write_all(last.as_bytes())?;
        }
    }
    Ok(())
}
