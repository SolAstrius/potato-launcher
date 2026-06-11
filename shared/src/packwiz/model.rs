use std::collections::BTreeMap;

use serde::Deserialize;

use crate::files::HashAlgo;

/// `pack.toml` — the packwiz pack root.
#[derive(Deserialize, Debug)]
pub struct PackToml {
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "pack-format", default)]
    pub pack_format: Option<String>,
    pub index: IndexRef,
    /// Loader keys: "minecraft", "neoforge", "forge", "fabric-loader", "quilt-loader".
    pub versions: BTreeMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct IndexRef {
    pub file: String,
    #[serde(rename = "hash-format")]
    pub hash_format: String,
    pub hash: String,
}

/// `index.toml` — the list of every tracked file in the pack.
#[derive(Deserialize, Debug)]
pub struct IndexToml {
    #[serde(rename = "hash-format")]
    pub hash_format: String,
    #[serde(default)]
    pub files: Vec<IndexFile>,
}

#[derive(Deserialize, Debug)]
pub struct IndexFile {
    /// Path relative to the pack root (e.g. "mods/sodium.pw.toml", "config/foo.json").
    pub file: String,
    pub hash: String,
    /// Per-file override; falls back to [`IndexToml::hash_format`].
    #[serde(rename = "hash-format", default)]
    pub hash_format: Option<String>,
    #[serde(default)]
    pub metafile: bool,
}

/// A `*.pw.toml` metafile (one per mod / resource pack / etc.).
#[derive(Deserialize, Debug)]
pub struct Metafile {
    pub name: String,
    pub filename: String,
    #[serde(default)]
    pub side: Side,
    pub download: MetaDownload,
}

#[derive(Deserialize, Debug)]
pub struct MetaDownload {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(rename = "hash-format", default)]
    pub hash_format: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
    /// "metadata:curseforge" means there is no direct URL — unsupported.
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Client,
    Server,
    #[default]
    Both,
}

impl Side {
    pub fn wanted_on_client(self) -> bool {
        matches!(self, Side::Client | Side::Both)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PackwizError {
    #[error("CurseForge metadata-only mod is not supported (no direct download URL): {file}")]
    CurseForgeUnsupported { file: String },
    #[error("Metafile is missing a download URL or hash: {file}")]
    MissingDownload { file: String },
    #[error("Unsupported loader in pack.toml: {name}")]
    UnsupportedLoader { name: String },
    #[error("pack.toml lists no Minecraft version")]
    MissingMinecraftVersion,
    #[error("index.toml hash mismatch (expected {expected}, got {actual})")]
    IndexHashMismatch { expected: String, actual: String },
    #[error("Unknown hash format: {0}")]
    UnknownHashFormat(String),
    #[error("Duplicate install path in pack: {0}")]
    DuplicatePath(String),
}

pub fn map_hash_format(s: &str) -> Result<HashAlgo, PackwizError> {
    match s {
        "sha1" => Ok(HashAlgo::Sha1),
        "sha256" => Ok(HashAlgo::Sha256),
        "sha512" => Ok(HashAlgo::Sha512),
        other => Err(PackwizError::UnknownHashFormat(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pack_toml() {
        let pack: PackToml = toml::from_str(
            r#"
name = "Driftward"
author = "Sol"
version = "1.0.0"
pack-format = "packwiz:1.1.0"

[index]
file = "index.toml"
hash-format = "sha256"
hash = "abc123"

[versions]
minecraft = "1.21.1"
neoforge = "21.1.233"
"#,
        )
        .unwrap();
        assert_eq!(pack.name, "Driftward");
        assert_eq!(pack.index.hash, "abc123");
        assert_eq!(pack.versions.get("minecraft").unwrap(), "1.21.1");
        assert_eq!(pack.versions.get("neoforge").unwrap(), "21.1.233");
    }

    #[test]
    fn parse_metafile_modrinth() {
        let meta: Metafile = toml::from_str(
            r#"
name = "Sodium"
filename = "sodium-neoforge-0.6.13+mc1.21.1.jar"
side = "client"

[download]
url = "https://cdn.modrinth.com/data/AANobbMI/versions/Pb3OXVqC/sodium.jar"
hash-format = "sha512"
hash = "deadbeef"

[update]
[update.modrinth]
mod-id = "AANobbMI"
version = "Pb3OXVqC"
"#,
        )
        .unwrap();
        assert_eq!(meta.side, Side::Client);
        assert!(meta.side.wanted_on_client());
        assert_eq!(meta.download.hash_format.as_deref(), Some("sha512"));
        assert!(meta.download.url.is_some());
    }

    #[test]
    fn side_default_is_both() {
        let meta: Metafile = toml::from_str(
            r#"
name = "Some Lib"
filename = "lib.jar"

[download]
url = "https://example/lib.jar"
hash-format = "sha512"
hash = "ff"
"#,
        )
        .unwrap();
        assert_eq!(meta.side, Side::Both);
        assert!(meta.side.wanted_on_client());
    }

    #[test]
    fn server_side_not_wanted() {
        assert!(!Side::Server.wanted_on_client());
    }

    #[test]
    fn extra_keys_tolerated() {
        // driftward's FFAPI marker carries `pin = true` and an [update] table.
        let meta: Metafile = toml::from_str(
            r#"
name = "Forgified Fabric API"
filename = "ffapi.jar"
side = "both"
pin = true

[download]
url = "https://mc.sol.moe/pack/libs/ffapi.jar"
hash-format = "sha512"
hash = "aa"

[update]
[update.modrinth]
mod-id = "Aqlf1Shp"
version = "7nHK7hMg"
"#,
        )
        .unwrap();
        assert_eq!(meta.download.url.as_deref(), Some("https://mc.sol.moe/pack/libs/ffapi.jar"));
    }

    #[test]
    fn map_hash_formats() {
        assert_eq!(map_hash_format("sha1").unwrap(), HashAlgo::Sha1);
        assert_eq!(map_hash_format("sha256").unwrap(), HashAlgo::Sha256);
        assert_eq!(map_hash_format("sha512").unwrap(), HashAlgo::Sha512);
        assert!(map_hash_format("murmur2").is_err());
    }
}
