use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    adaptive_download::download_files,
    files::{self, CheckEntry},
    paths::get_asset_index_path,
    progress,
    version::version_metadata::AssetIndex,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ObjectData {
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct AssetsMetadata {
    pub objects: HashMap<String, ObjectData>,
}

impl AssetsMetadata {
    pub async fn fetch(url: &str) -> anyhow::Result<Self> {
        let client = Client::new();
        let response = client.get(url).send().await?.json().await?;
        Ok(response)
    }

    pub async fn get_path(assets_dir: &Path, asset_id: &str) -> anyhow::Result<PathBuf> {
        tokio::fs::create_dir_all(assets_dir.join("indexes")).await?;
        Ok(assets_dir.join("indexes").join(format!("{asset_id}.json")))
    }

    pub async fn read_local(asset_id: &str, assets_dir: &Path) -> anyhow::Result<Self> {
        let data = tokio::fs::read(Self::get_path(assets_dir, asset_id).await?).await?;
        let data: Self = serde_json::from_slice(&data)?;
        Ok(data)
    }

    pub async fn read_or_download(
        asset_index: &AssetIndex,
        assets_dir: &Path,
    ) -> anyhow::Result<Self> {
        let asset_index_path = get_asset_index_path(assets_dir, &asset_index.id);
        let check_entry = CheckEntry {
            url: asset_index.url.clone(),
            remote_sha1: Some(asset_index.sha1.clone()),
            algo: files::HashAlgo::Sha1,
            path: asset_index_path.clone(),
        };
        let check_entries = vec![check_entry];
        let download_entries =
            files::get_download_entries(check_entries, progress::no_progress_bar()).await?;
        download_files(download_entries, progress::no_progress_bar()).await?;
        Self::read_local(&asset_index.id, assets_dir).await
    }

    pub fn get_check_entries(
        &self,
        assets_dir: &Path,
        resources_url_base: &str,
        check_hashes: bool,
    ) -> anyhow::Result<Vec<CheckEntry>> {
        let mut download_entries = vec![];

        download_entries.extend(self.objects.values().map(|object| {
            CheckEntry {
                url: format!(
                    "{}/{}/{}",
                    resources_url_base,
                    &object.hash[..2],
                    object.hash
                ),
                path: assets_dir
                    .join("objects")
                    .join(&object.hash[..2])
                    .join(&object.hash),
                remote_sha1: if check_hashes {
                    Some(object.hash.clone())
                } else {
                    None
                },
                algo: files::HashAlgo::Sha1,
            }
        }));

        Ok(download_entries)
    }

    pub async fn save_to_file(&self, asset_id: &str, assets_dir: &Path) -> anyhow::Result<()> {
        let data = serde_json::to_vec(self)?;
        tokio::fs::write(Self::get_path(assets_dir, asset_id).await?, data).await?;
        Ok(())
    }
}
