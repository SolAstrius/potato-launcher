use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    files::hash_file,
    paths::{
        get_rel_extra_metadata_path, get_rel_metadata_path, get_rel_versions_dir,
        get_rel_versions_extra_dir,
    },
    utils::url_from_rel_path,
    version::{
        version_manifest::{MetadataInfo, VersionInfo},
        version_metadata::VersionMetadata,
    },
};

pub async fn get_version_info(
    work_dir: &Path,
    version_metadata: &Vec<VersionMetadata>,
    version_name: &str,
    download_server_base: Option<&str>,
    replaced_metadata: &HashMap<String, PathBuf>,
) -> anyhow::Result<VersionInfo> {
    // is used in local instances to be compliant with the manifest format
    let download_server_base = download_server_base.unwrap_or("empty-url");

    let rel_versions_dir = get_rel_versions_dir();
    let mut metadata_info = vec![];
    for metadata in version_metadata {
        let rel_metadata_path = rel_versions_dir.join(get_rel_metadata_path(&metadata.id));
        let metadata_path = replaced_metadata
            .get(&metadata.id)
            .cloned()
            .unwrap_or_else(|| work_dir.join(&rel_metadata_path));
        metadata_info.push(MetadataInfo {
            id: metadata.id.clone(),
            url: url_from_rel_path(&rel_metadata_path, download_server_base)?,
            sha1: hash_file(&metadata_path).await?,
        });
    }

    let rel_extra_metadata_path =
        get_rel_versions_extra_dir().join(get_rel_extra_metadata_path(version_name));
    let extra_metadata_path = work_dir.join(&rel_extra_metadata_path);

    let mut extra_metadata_url = None;
    let mut extra_metadata_sha1 = None;
    if extra_metadata_path.exists() {
        extra_metadata_url = Some(url_from_rel_path(
            &rel_extra_metadata_path,
            download_server_base,
        )?);
        extra_metadata_sha1 = Some(hash_file(&extra_metadata_path).await?);
    }

    let child_metadata_info = metadata_info
        .pop()
        .ok_or(anyhow::Error::msg("No child metadata"))?;
    Ok(VersionInfo {
        id: child_metadata_info.id,
        url: child_metadata_info.url,
        sha1: child_metadata_info.sha1,
        name: Some(version_name.to_string()),
        inherits_from: metadata_info,
        extra_metadata_url,
        extra_metadata_sha1,
        packwiz_url: None,
        packwiz_auth_backend: None,
        packwiz_recommended_xmx: None,
    })
}
