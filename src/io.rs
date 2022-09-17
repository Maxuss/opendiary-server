use std::path::PathBuf;
use anyhow::bail;
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncReadExt, BufReader};

pub async fn prepare_io() {
    let mut diary_dir = PathBuf::from("diary");
    create_dir_all(diary_dir).await?;
}

pub async fn create_io_file<S: Into<String>>(path: S) -> anyhow::Result<File> {
    let pathbuf = PathBuf::from(path);
    create_dir_all(pathbuf.parent()).await?;
    if pathbuf.exists() {
        bail!("File already exists!")
    }
    return File::create(pathbuf).await.map_err(anyhow::Error::from)
}

pub async fn read_io_file<S: Into<String>>(path: S) -> anyhow::Result<Vec<u8>> {
    let buf = PathBuf::from(path);
    if !buf.exists() {
        bail!("Tried to read nonexistent file!")
    }
    let mut bytes = Vec::new();
    BufReader::new(File::open(buf).await?).read_to_end(&mut bytes).await?;
    Ok(bytes)
}