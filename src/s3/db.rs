use std::{path::Path, process::Stdio};

use aws_sdk_s3::{primitives::ByteStream, Client};
use anyhow::Result;
use log::info;
use rusqlite::Connection;
use tokio::{io::AsyncWriteExt, process::Command};

use crate::s3::nix::{getmeta, getrevision};

pub async fn create_db(
    client: &Client,
    channel: &str,
    revision: &str,
    upload: bool,
    bucket: &str,
) -> Result<()> {
    let data = getmeta(channel, revision).await?;
    let rev = getrevision(channel, revision).await?;

    info!("Got data, creating db");

    // Create database with rusqlite
    let db = format!("{}.db", rev);

    // Delete old db
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(format!("{}.br", db));

    let conn = Connection::open(&db).expect("Failed to open database");

    // Create table
    conn.execute(
        r#"CREATE TABLE pkgs (
                "attribute" TEXT NOT NULL UNIQUE,
                "pname" TEXT,
                "version" TEXT,
                PRIMARY KEY("attribute")
            )"#,
        [],
    )
    .expect("Failed to create table");

    conn.execute(
        r#"
        CREATE TABLE "meta" (
            "attribute"	TEXT NOT NULL UNIQUE,
            "description"	TEXT,
            "long_description"	TEXT,
            "branch"	TEXT,
            "homepage"	JSON,
            "download_page"	JSON,
            "changelog"	JSON,
            "license"	JSON,
            "maintainers"	JSON,
            "main_program"	TEXT,
            "platforms"	JSON,
            "bad_platforms"	JSON,
            "broken"	INTEGER,
            "unfree"	INTEGER,
            "insecure"	INTEGER,
            FOREIGN KEY("attribute") REFERENCES "pkgs" ("attribute"),
            PRIMARY KEY("attribute")
        )
            "#,
        [],
    )
    .expect("Failed to create table");

    // Create index
    conn.execute(r#"CREATE INDEX "idx_pkgs" ON "pkgs" ("attribute")"#, [])
        .expect("Failed to create index");

    conn.execute(r#"CREATE INDEX "idx_meta" ON "meta" ("attribute")"#, [])
        .expect("Failed to create index");

    let mut wtr = csv::Writer::from_writer(vec![]);
    for (_path, store) in &data {
        wtr.serialize((
            store.attribute.to_string(),
            store.pname.to_string(),
            store.version.to_string(),
        ))?;
    }
    let pkgdata = String::from_utf8(wtr.into_inner()?)?;
    let mut pkgcmd = Command::new("sqlite3")
        .arg("-csv")
        .arg(&db)
        .arg(".import '|cat -' pkgs")
        .stdin(Stdio::piped())
        .spawn()?;
    let pkgcmd_stdin = pkgcmd.stdin.as_mut().unwrap();
    pkgcmd_stdin.write_all(pkgdata.as_bytes()).await?;
    let _status = pkgcmd.wait().await?;

    let mut metawtr = csv::Writer::from_writer(vec![]);

    // Insert data
    for (_path, store) in data {
        // Insert into meta table
        metawtr
            .serialize((
                store.attribute,
                store.meta.description.unwrap_or_default(),
                store.meta.long_description.unwrap_or_default(),
                store.meta.branch.unwrap_or_default(),
                store
                    .meta
                    .homepage
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .download_page
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .changelog
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .license
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .maintainers
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store.meta.main_program.unwrap_or_default(),
                store
                    .meta
                    .platforms
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .bad_platforms
                    .map(|x| x.to_string())
                    .unwrap_or_default(),
                store
                    .meta
                    .broken
                    .map(|x| if x { 1 } else { 0 })
                    .unwrap_or(0)
                    .to_string(),
                store
                    .meta
                    .unfree
                    .map(|x| if x { 1 } else { 0 })
                    .unwrap_or(0)
                    .to_string(),
                store
                    .meta
                    .insecure
                    .map(|x| if x { 1 } else { 0 })
                    .unwrap_or(0)
                    .to_string(),
            ))
            .expect("Failed to insert into meta table");
    }

    let metadata = String::from_utf8(metawtr.into_inner()?)?;

    let mut metacmd = Command::new("sqlite3")
        .arg("-csv")
        .arg(&db)
        .arg(".import '|cat -' meta")
        .stdin(Stdio::piped())
        .spawn()?;
    let metacmd_stdin = metacmd.stdin.as_mut().unwrap();
    metacmd_stdin.write_all(metadata.as_bytes()).await?;
    let _status = metacmd.wait().await?;

    if upload {
        // Compress with brotli
        info!("Compressing with brotli");
        let mut cmd = Command::new("brotli").arg("--rm").arg(&db).spawn()?;
        let _status = cmd.wait().await?;

        // Upload to S3
        info!("Uploading to S3");
        let body = ByteStream::from_path(Path::new(&format!("{}.br", db))).await?;
        client
            .put_object()
            .content_encoding("br")
            .bucket(bucket)
            .key(rev)
            .body(body)
            .send()
            .await?;

        // Cleanup
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{}.br", db));
    }

    Ok(())
}
