use std::path::Path;
use std::sync::Arc;

use log::info;

use crate::files::hash_bytes;
use crate::generate::manifest::get_version_info;
use crate::loader_generator::fabric::FabricGenerator;
use crate::loader_generator::forge::{ForgeGenerator, Loader};
use crate::loader_generator::generator::VersionGenerator;
use crate::loader_generator::vanilla::VanillaGenerator;
use crate::paths::get_versions_extra_dir;
use crate::progress::NoProgressBar;
use crate::utils::{get_vanilla_version_info, VANILLA_MANIFEST_URL};
use crate::version::extra_version_metadata::{AuthBackend, ExtraVersionMetadata, Include, Object};
use crate::version::version_manifest::{VersionInfo, VersionManifest};

use super::fetch::{self, join_url, pack_base_url};
use super::model::{map_hash_format, IndexToml, Metafile, PackToml, PackwizError};

/// Result of generating a packwiz instance: the launcher [`VersionInfo`] plus the pack's
/// index hash (used later for cheap update detection).
pub struct PackwizInstance {
    pub version_info: VersionInfo,
    pub index_hash: String,
    pub pack_name: String,
}

/// Build the `client`-relevant [`Object`] list from the index + fetched metafiles.
///
/// Pure (no I/O) so it can be unit-tested. `base_url` is used to resolve loose-file URLs;
/// metafile `[download].url`s are absolute and used verbatim.
fn build_objects(
    base_url: &str,
    index: &IndexToml,
    metafiles: &[(String, Metafile)],
) -> Result<Vec<Object>, PackwizError> {
    let mut objects: Vec<Object> = Vec::new();

    // Loose files: hosted at base_url + path, hash from the index entry.
    for entry in &index.files {
        if entry.metafile {
            continue;
        }
        let algo = map_hash_format(entry.hash_format.as_deref().unwrap_or(&index.hash_format))?;
        objects.push(Object {
            path: entry.file.clone(),
            sha1: entry.hash.clone(),
            algo,
            url: join_url(base_url, &entry.file),
        });
    }

    // Metafiles: one mod/resource each. Apply the client side filter + reject CurseForge.
    for (rel_path, meta) in metafiles {
        if !meta.side.wanted_on_client() {
            continue;
        }
        if meta.download.mode.as_deref() == Some("metadata:curseforge") {
            return Err(PackwizError::CurseForgeUnsupported {
                file: rel_path.clone(),
            });
        }
        let (url, hash) = match (&meta.download.url, &meta.download.hash) {
            (Some(url), Some(hash)) => (url.clone(), hash.clone()),
            _ => {
                return Err(PackwizError::MissingDownload {
                    file: rel_path.clone(),
                })
            }
        };
        let algo = map_hash_format(meta.download.hash_format.as_deref().unwrap_or("sha512"))?;

        // Install path = directory of the metafile + its declared filename.
        let dir = Path::new(rel_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|s| !s.is_empty());
        let install_path = match dir {
            Some(dir) => format!("{dir}/{}", meta.filename),
            None => meta.filename.clone(),
        };

        objects.push(Object {
            path: install_path,
            sha1: hash,
            algo,
            url,
        });
    }

    // Deterministic order so the saved extra-metadata JSON (and its hash) is stable.
    objects.sort_by(|a, b| a.path.cmp(&b.path));

    // Reject colliding install paths rather than silently dropping one.
    for pair in objects.windows(2) {
        if pair[0].path == pair[1].path {
            return Err(PackwizError::DuplicatePath(pair[0].path.clone()));
        }
    }

    Ok(objects)
}

/// Pick + construct the loader version generator from a pack's `[versions]` table.
fn loader_generator(
    instance_name: &str,
    vanilla_info: VersionInfo,
    versions: &std::collections::BTreeMap<String, String>,
) -> Result<Box<dyn VersionGenerator + Send>, PackwizError> {
    if let Some(v) = versions.get("neoforge") {
        Ok(Box::new(ForgeGenerator::new(
            instance_name.to_string(),
            vanilla_info,
            Loader::Neoforge,
            Some(v.clone()),
            Arc::new(NoProgressBar),
        )))
    } else if let Some(v) = versions.get("forge") {
        Ok(Box::new(ForgeGenerator::new(
            instance_name.to_string(),
            vanilla_info,
            Loader::Forge,
            Some(v.clone()),
            Arc::new(NoProgressBar),
        )))
    } else if let Some(v) = versions.get("fabric-loader") {
        Ok(Box::new(FabricGenerator::new(
            instance_name.to_string(),
            vanilla_info,
            Some(v.clone()),
        )))
    } else if versions.contains_key("quilt-loader") {
        Err(PackwizError::UnsupportedLoader {
            name: "quilt-loader".to_string(),
        })
    } else {
        Ok(Box::new(VanillaGenerator::new(
            instance_name.to_string(),
            vanilla_info,
        )))
    }
}

/// Fetch + parse a packwiz pack and emit a launcher [`VersionInfo`].
///
/// Writes all base + extra metadata into `work_dir` (the launcher data dir) so that
/// [`crate::version::version_metadata::VersionMetadata::read_local`] and the `"empty-url"`
/// placeholder URLs from [`get_version_info`] are never fetched at sync time.
/// Split the resolved pack files into include rules. `mods/` is fully pack-controlled, so it
/// gets `delete_extra:true` to prune jars no longer in the pack — without this, renamed/removed
/// jars accumulate, and a stale Sinytra Connector jar (picked up as a modlauncher transformation
/// service, which has no version dedup) silently wins over the new one, stranding the client on
/// the old loader. Everything else stays rooted at the minecraft dir with `delete_extra:false`
/// so user files (saves, options.txt, logs, edited configs, …) are never deleted.
fn split_includes(objects: Vec<Object>) -> Vec<Include> {
    let is_under_mods = |p: &str| {
        let p = p.replace('\\', "/");
        p == "mods" || p.starts_with("mods/")
    };
    let (mods_objects, other_objects): (Vec<_>, Vec<_>) =
        objects.into_iter().partition(|o| is_under_mods(&o.path));

    vec![
        Include {
            path: "mods".to_string(),
            overwrite: true,
            delete_extra: true,
            recursive: true,
            objects: mods_objects,
        },
        Include {
            path: ".".to_string(),
            overwrite: true,
            delete_extra: false,
            recursive: true,
            objects: other_objects,
        },
    ]
}

pub async fn generate_packwiz_instance(
    work_dir: &Path,
    pack_url: &str,
    instance_name: &str,
    auth_backend: Option<AuthBackend>,
    recommended_xmx: Option<String>,
) -> anyhow::Result<PackwizInstance> {
    let client = reqwest::Client::new();
    let base_url = pack_base_url(pack_url);

    let vanilla_manifest = VersionManifest::fetch(VANILLA_MANIFEST_URL).await?;

    info!("Fetching packwiz pack from {pack_url}");
    let pack: PackToml = fetch::fetch_pack(&client, pack_url).await?;

    let index_url = join_url(&base_url, &pack.index.file);
    let (index, index_bytes) = fetch::fetch_index(&client, &index_url).await?;

    // Verify the index against the hash pinned in pack.toml.
    let index_algo = map_hash_format(&pack.index.hash_format)?;
    let actual = hash_bytes(&index_bytes, index_algo);
    if actual != pack.index.hash {
        return Err(PackwizError::IndexHashMismatch {
            expected: pack.index.hash.clone(),
            actual,
        }
        .into());
    }

    // Fetch every metafile concurrently.
    let metafile_paths: Vec<String> = index
        .files
        .iter()
        .filter(|f| f.metafile)
        .map(|f| f.file.clone())
        .collect();
    info!("Fetching {} packwiz metafiles", metafile_paths.len());
    let metafiles = fetch::fetch_metafiles(&client, &base_url, metafile_paths).await?;

    let objects = build_objects(&base_url, &index, &metafiles)?;
    info!("Resolved {} client files from pack", objects.len());

    // Build the base (vanilla + loader) metadata using the existing generators.
    let mc_version = pack
        .versions
        .get("minecraft")
        .ok_or(PackwizError::MissingMinecraftVersion)?;
    let vanilla_info = get_vanilla_version_info(&vanilla_manifest, mc_version)?;
    let generator = loader_generator(instance_name, vanilla_info, &pack.versions)?;
    let generator_result = generator.generate(work_dir).await?;

    let extra_metadata = ExtraVersionMetadata {
        auth_backend,
        include: split_includes(objects),
        resources_url_base: None,
        extra_forge_libs: vec![],
        recommended_xmx,
    };
    extra_metadata
        .save(instance_name, &get_versions_extra_dir(work_dir))
        .await?;

    let version_info = get_version_info(
        work_dir,
        &generator_result.metadata,
        instance_name,
        None,
        &std::collections::HashMap::new(),
    )
    .await?;

    Ok(PackwizInstance {
        version_info,
        index_hash: pack.index.hash,
        pack_name: pack.name,
    })
}

/// Fetch only `pack.toml` and return its index hash — the cheap update-check signal.
pub async fn fetch_index_hash(pack_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let pack = fetch::fetch_pack(&client, pack_url).await?;
    Ok(pack.index.hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::HashAlgo;
    use crate::packwiz::model::{IndexFile, MetaDownload, Side};

    fn index_with(files: Vec<IndexFile>) -> IndexToml {
        IndexToml {
            hash_format: "sha256".to_string(),
            files,
        }
    }

    fn meta(filename: &str, side: Side, url: Option<&str>, mode: Option<&str>) -> Metafile {
        Metafile {
            name: filename.to_string(),
            filename: filename.to_string(),
            side,
            download: MetaDownload {
                url: url.map(|s| s.to_string()),
                hash_format: Some("sha512".to_string()),
                hash: url.map(|_| "ff".to_string()),
                mode: mode.map(|s| s.to_string()),
            },
        }
    }

    #[test]
    fn loose_file_and_mods_with_side_filter() {
        let index = index_with(vec![
            IndexFile {
                file: "config/foo.json".to_string(),
                hash: "abc".to_string(),
                hash_format: None,
                metafile: false,
            },
            IndexFile {
                file: "mods/sodium.pw.toml".to_string(),
                hash: "meta1".to_string(),
                hash_format: None,
                metafile: true,
            },
            IndexFile {
                file: "mods/serverthing.pw.toml".to_string(),
                hash: "meta2".to_string(),
                hash_format: None,
                metafile: true,
            },
        ]);
        let metafiles = vec![
            (
                "mods/sodium.pw.toml".to_string(),
                meta(
                    "sodium.jar",
                    Side::Client,
                    Some("https://cdn/sodium.jar"),
                    None,
                ),
            ),
            (
                "mods/serverthing.pw.toml".to_string(),
                meta(
                    "serverthing.jar",
                    Side::Server,
                    Some("https://cdn/st.jar"),
                    None,
                ),
            ),
        ];

        let objects = build_objects("https://mc.sol.moe/pack", &index, &metafiles).unwrap();
        // server-only mod dropped; loose file + client mod kept; sorted by path.
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].path, "config/foo.json");
        assert_eq!(objects[0].algo, HashAlgo::Sha256);
        assert_eq!(objects[0].url, "https://mc.sol.moe/pack/config/foo.json");
        assert_eq!(objects[1].path, "mods/sodium.jar");
        assert_eq!(objects[1].algo, HashAlgo::Sha512);
        assert_eq!(objects[1].url, "https://cdn/sodium.jar");
    }

    #[test]
    fn split_includes_prunes_mods_only() {
        let obj = |path: &str| Object {
            path: path.to_string(),
            sha1: "ff".to_string(),
            algo: HashAlgo::Sha256,
            url: format!("https://h/{path}"),
        };
        let includes = split_includes(vec![
            obj("mods/connector.jar"),
            obj("mods/owo.jar"),
            obj("config/foo.json"),
            obj("options.txt"),
        ]);

        let mods = &includes[0];
        assert_eq!(mods.path, "mods");
        assert!(mods.overwrite && mods.delete_extra, "mods/ must prune");
        let mut mod_paths: Vec<_> = mods.objects.iter().map(|o| o.path.as_str()).collect();
        mod_paths.sort();
        assert_eq!(mod_paths, ["mods/connector.jar", "mods/owo.jar"]);

        let rest = &includes[1];
        assert_eq!(rest.path, ".");
        assert!(rest.overwrite && !rest.delete_extra, "user files must survive");
        let mut rest_paths: Vec<_> = rest.objects.iter().map(|o| o.path.as_str()).collect();
        rest_paths.sort();
        assert_eq!(rest_paths, ["config/foo.json", "options.txt"]);
    }

    #[test]
    fn curseforge_metadata_errors() {
        let index = index_with(vec![IndexFile {
            file: "mods/cf.pw.toml".to_string(),
            hash: "m".to_string(),
            hash_format: None,
            metafile: true,
        }]);
        let metafiles = vec![(
            "mods/cf.pw.toml".to_string(),
            meta("cf.jar", Side::Both, None, Some("metadata:curseforge")),
        )];
        let err = build_objects("https://h/p", &index, &metafiles).unwrap_err();
        assert!(matches!(err, PackwizError::CurseForgeUnsupported { .. }));
    }

    #[test]
    fn url_less_metafile_errors() {
        let index = index_with(vec![IndexFile {
            file: "mods/x.pw.toml".to_string(),
            hash: "m".to_string(),
            hash_format: None,
            metafile: true,
        }]);
        let metafiles = vec![(
            "mods/x.pw.toml".to_string(),
            meta("x.jar", Side::Both, None, None),
        )];
        let err = build_objects("https://h/p", &index, &metafiles).unwrap_err();
        assert!(matches!(err, PackwizError::MissingDownload { .. }));
    }
}
