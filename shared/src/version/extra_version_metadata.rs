use std::path::Path;

use log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    files::{CheckEntry, HashAlgo},
    paths::get_extra_metadata_path,
};

use super::{version_manifest::VersionInfo, version_metadata::Library};

#[derive(Deserialize, Serialize, Debug)]
pub struct Object {
    pub path: String,
    /// Expected hash. Despite the field name this holds a digest in the algorithm given by
    /// `algo` (SHA-1 for server-generated manifests, SHA-256/SHA-512 for packwiz packs).
    pub sha1: String,
    #[serde(default)]
    pub algo: HashAlgo,
    pub url: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct TelegramAuthBackend {
    pub auth_base_url: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
pub struct ElyByAuthBackend {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthBackend {
    Telegram(TelegramAuthBackend),
    #[serde(rename = "ely.by")]
    ElyBy(ElyByAuthBackend),
    #[default]
    Microsoft,
    Offline,
}

impl AuthBackend {
    pub fn get_id(&self) -> String {
        match self {
            AuthBackend::Telegram(auth_data) => format!("telegram_{}", auth_data.auth_base_url),
            AuthBackend::ElyBy(auth_data) => {
                format!("elyby_{}_{}", auth_data.client_id, auth_data.client_secret)
            }
            AuthBackend::Microsoft => "microsoft".to_string(),
            AuthBackend::Offline => "offline".to_string(),
        }
    }

    pub fn from_id(id: &str) -> Self {
        let parts: Vec<&str> = id.split('_').collect();
        match parts[0] {
            "telegram" => AuthBackend::Telegram(TelegramAuthBackend {
                auth_base_url: parts[1].to_string(),
            }),
            "elyby" => AuthBackend::ElyBy(ElyByAuthBackend {
                client_id: parts[1].to_string(),
                client_secret: parts[2].to_string(),
            }),
            "microsoft" => AuthBackend::Microsoft,
            "offline" => AuthBackend::Offline,
            _ => {
                warn!("Unknown auth backend id: {id}");
                AuthBackend::Microsoft
            }
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, Serialize)]
pub struct Include {
    pub path: String,

    #[serde(default = "yes")]
    pub overwrite: bool,

    #[serde(default = "yes")]
    pub delete_extra: bool,

    #[serde(default)]
    pub recursive: bool,

    #[serde(default)]
    pub objects: Vec<Object>,
}

#[derive(Deserialize, Serialize)]
pub struct ExtraVersionMetadata {
    #[serde(default)]
    pub auth_backend: Option<AuthBackend>,

    #[serde(default)]
    pub include: Vec<Include>,

    #[serde(default)]
    pub resources_url_base: Option<String>,

    #[serde(default)]
    pub extra_forge_libs: Vec<Library>,

    pub recommended_xmx: Option<String>,
}

impl ExtraVersionMetadata {
    pub async fn read_local(
        version_info: &VersionInfo,
        versions_extra_dir: &Path,
    ) -> anyhow::Result<Option<Self>> {
        if version_info.extra_metadata_url.is_none() || version_info.extra_metadata_sha1.is_none() {
            return Ok(None);
        }

        let extra_version_metadata_path =
            get_extra_metadata_path(versions_extra_dir, &version_info.get_name());
        let extra_version_metadata_file = tokio::fs::read(extra_version_metadata_path).await?;

        Ok(Some(serde_json::from_slice(&extra_version_metadata_file)?))
    }

    pub fn get_check_entry(
        version_info: &VersionInfo,
        versions_extra_dir: &Path,
    ) -> Option<CheckEntry> {
        if version_info.extra_metadata_url.is_none() || version_info.extra_metadata_sha1.is_none() {
            return None;
        }

        let url = version_info.extra_metadata_url.as_ref().unwrap();
        let sha1 = version_info.extra_metadata_sha1.as_ref().unwrap();

        Some(CheckEntry {
            url: url.clone(),
            remote_sha1: Some(sha1.clone()),
            algo: HashAlgo::Sha1,
            path: get_extra_metadata_path(versions_extra_dir, &version_info.get_name()),
        })
    }

    pub async fn save(&self, version_name: &str, versions_extra_dir: &Path) -> anyhow::Result<()> {
        let path = get_extra_metadata_path(versions_extra_dir, version_name);
        let serialized = serde_json::to_string(self)?;
        tokio::fs::write(path, serialized).await?;

        Ok(())
    }
}
