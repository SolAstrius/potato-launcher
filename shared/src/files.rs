use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt as _;
use tokio::{fs, io};
use walkdir::WalkDir;

use crate::progress::{run_tasks_with_progress, ProgressBar};

/// Hash algorithm used to verify a downloaded file.
///
/// Server-generated manifests and Mojang/loader metadata are all SHA-1, which is the default
/// so existing JSON (where the field is absent) keeps deserializing unchanged. packwiz packs
/// use SHA-256 (index + loose files) and SHA-512 (mod jars).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgo {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

pub fn get_files_in_dir(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        let entries = std::fs::read_dir(path)?;
        for entry in entries.flatten() {
            files.extend(get_files_in_dir(&entry.path())?);
        }
    }
    Ok(files)
}

pub fn get_files_ignore_paths(
    path: &Path,
    ignore_paths: &HashSet<PathBuf>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        let entries = std::fs::read_dir(path)?;
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !ignore_paths.contains(&entry_path) {
                files.extend(get_files_ignore_paths(&entry_path, ignore_paths)?);
            }
        }
    }
    Ok(files)
}

/// Hash a file with SHA-1. Thin wrapper kept for the many existing callers.
pub async fn hash_file(path: &Path) -> anyhow::Result<String> {
    hash_file_algo(path, HashAlgo::Sha1).await
}

/// Hash a file with the given algorithm, returning the lowercase hex digest.
pub async fn hash_file_algo(path: &Path, algo: HashAlgo) -> anyhow::Result<String> {
    let mut file = fs::File::open(path).await?;
    let mut buffer = [0; 1024];

    macro_rules! hash_with {
        ($hasher:expr) => {{
            let mut hasher = $hasher;
            loop {
                let n = file.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            format!("{:x}", hasher.finalize())
        }};
    }

    Ok(match algo {
        HashAlgo::Sha1 => hash_with!(Sha1::new()),
        HashAlgo::Sha256 => hash_with!(Sha256::new()),
        HashAlgo::Sha512 => hash_with!(Sha512::new()),
    })
}

/// Hash an in-memory buffer with the given algorithm, returning the lowercase hex digest.
pub fn hash_bytes(bytes: &[u8], algo: HashAlgo) -> String {
    match algo {
        HashAlgo::Sha1 => format!("{:x}", Sha1::digest(bytes)),
        HashAlgo::Sha256 => format!("{:x}", Sha256::digest(bytes)),
        HashAlgo::Sha512 => format!("{:x}", Sha512::digest(bytes)),
    }
}

pub async fn hash_files<M>(
    files: Vec<PathBuf>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> anyhow::Result<Vec<String>> {
    hash_files_algo(files, HashAlgo::Sha1, progress_bar).await
}

pub async fn hash_files_algo<M>(
    files: Vec<PathBuf>,
    algo: HashAlgo,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> anyhow::Result<Vec<String>> {
    let tasks_count = files.len() as u64;

    let tasks = files
        .into_iter()
        .map(|path| async move { hash_file_algo(&path, algo).await });

    run_tasks_with_progress(tasks, progress_bar, tasks_count, num_cpus::get()).await
}

pub async fn remove_file_or_dir(path: &Path) -> anyhow::Result<()> {
    if path.is_file() {
        fs::remove_file(path).await?;
    } else if path.is_dir() {
        fs::remove_dir_all(path).await?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct DownloadEntry {
    pub url: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct CheckEntry {
    pub url: String,
    /// Expected remote hash. Despite the field name this holds a digest in the algorithm given
    /// by `algo` (SHA-1 for manifests/Mojang metadata, SHA-256/SHA-512 for packwiz files).
    pub remote_sha1: Option<String>,
    pub algo: HashAlgo,
    pub path: PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum CheckDownloadError {
    #[error("Hash of file {0} is missing")]
    HashMissing(PathBuf),
}

pub async fn get_download_entries<M>(
    check_entries: Vec<CheckEntry>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> anyhow::Result<Vec<DownloadEntry>> {
    // Group the files that need hashing by their algorithm, since packwiz mixes SHA-256
    // (loose files) and SHA-512 (mod jars) with the SHA-1 used everywhere else.
    let mut to_hash_by_algo: HashMap<HashAlgo, Vec<PathBuf>> = HashMap::new();
    for entry in &check_entries {
        if entry.path.is_file() && entry.remote_sha1.is_some() {
            to_hash_by_algo
                .entry(entry.algo)
                .or_default()
                .push(entry.path.clone());
        }
    }

    let mut hashes: HashMap<PathBuf, String> = HashMap::new();
    for (algo, paths) in to_hash_by_algo {
        let group = hash_files_algo(paths.clone(), algo, progress_bar.clone()).await?;
        hashes.extend(paths.into_iter().zip(group));
    }

    let mut download_entries = HashMap::new();
    for entry in check_entries {
        let mut need_download = false;
        if !entry.path.is_file() {
            need_download = true;
        } else if let Some(remote_sha1) = &entry.remote_sha1 {
            if remote_sha1
                != hashes
                    .get(&entry.path)
                    .ok_or(CheckDownloadError::HashMissing(entry.path.clone()))?
            {
                need_download = true;
            }
        }

        if need_download {
            download_entries.insert(
                entry.path.clone(),
                DownloadEntry {
                    url: entry.url.clone(),
                    path: entry.path.clone(),
                },
            );
        }
    }

    Ok(download_entries.into_values().collect())
}

async fn remove_empty_dirs(path: &Path) -> anyhow::Result<()> {
    let root = path;

    for entry in WalkDir::new(path)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let entry_path = entry.path();

        if entry_path == root {
            continue;
        }

        if entry_path.is_dir()
            && fs::read_dir(entry_path)
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(entry_path).await?;
        }
    }
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum CopyFilesError {
    #[error("Source entry {0} does not exist")]
    SourceEntryMissing(PathBuf),
    #[error("Invalid path")]
    InvalidPath,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncMappingStats {
    pub total_files: usize,
    pub copied_files: usize,
    pub deleted_files: usize,
}

// copy mapped files and directories
// and delete all other files and directores in the target directory
// mapping: target -> source
pub async fn sync_mapping(
    target_dir: &Path,
    mapping: &HashMap<PathBuf, PathBuf>,
) -> anyhow::Result<SyncMappingStats> {
    let mut mappings_files = HashMap::new();
    for (target, source) in mapping {
        if !target.starts_with(target_dir) {
            return Err(CopyFilesError::InvalidPath.into());
        }
        if source.is_file() {
            mappings_files.insert(target.clone(), source.clone());
        } else if source.is_dir() {
            let files = get_files_in_dir(source)?;
            for file in files {
                let relative_path = file.strip_prefix(source).unwrap();
                let target_path = target.join(relative_path);
                mappings_files.insert(target_path, file);
            }
        } else {
            return Err(CopyFilesError::SourceEntryMissing(source.clone()).into());
        }
    }

    let mut deleted_files: usize = 0;
    let paths = get_files_in_dir(target_dir)?;
    for path in paths {
        if !mappings_files.contains_key(&path) {
            fs::remove_file(&path).await?;
            deleted_files += 1;
        }
    }

    remove_empty_dirs(target_dir).await?;

    async fn copy_file_if_needed(target: PathBuf, source: PathBuf) -> anyhow::Result<bool> {
        fs::create_dir_all(target.parent().ok_or(CopyFilesError::InvalidPath)?).await?;
        if target.is_dir() {
            fs::remove_dir(&target).await?;
        }
        let should_copy =
            !target.exists() || hash_file(&source).await? != hash_file(&target).await?;
        if should_copy {
            // copy and let umask set the permissions instead of fs::copy
            let mut src = File::open(&source).await?;
            let mut dst = File::create(&target).await?;
            io::copy(&mut src, &mut dst).await?;
        }
        Ok(should_copy)
    }

    const MAX_CONCURRENT_FILE_OPERATIONS: usize = 50;

    let total_files = mappings_files.len();
    let mut copied_files: usize = 0;

    let mut tasks = FuturesUnordered::new();
    let mut mapping_iter = mappings_files.iter();

    for _ in 0..MAX_CONCURRENT_FILE_OPERATIONS.min(mappings_files.len()) {
        if let Some((target, source)) = mapping_iter.next() {
            tasks.push(copy_file_if_needed(target.clone(), source.clone()));
        }
    }

    while let Some(result) = tasks.next().await {
        if result? {
            copied_files += 1;
        }

        if let Some((target, source)) = mapping_iter.next() {
            tasks.push(copy_file_if_needed(target.clone(), source.clone()));
        }
    }

    Ok(SyncMappingStats {
        total_files,
        copied_files,
        deleted_files,
    })
}

#[cfg(test)]
mod tests {
    use std::env;

    use maplit::hashmap;

    use super::*;

    #[tokio::test]
    async fn test_sync_mapping() {
        let temp_dir = env::temp_dir().join("instance_builder_test");
        let source_dir = temp_dir.join("source");
        let target_dir = temp_dir.join("target");
        let file1 = source_dir.join("file1");
        let dir1 = source_dir.join("dir1");
        let file2 = dir1.join("file2");
        let dir2 = source_dir.join("dir2");
        let file3 = dir2.join("file3");

        let file1_target = target_dir.join("file1");
        let file4 = target_dir.join("file4");
        let dir1_target = target_dir.join("dir1");
        let file2_target = dir1_target.join("file2");
        let file5 = dir1_target.join("file5");

        fs::create_dir_all(&dir1).await.unwrap();
        fs::create_dir_all(&dir2).await.unwrap();
        fs::create_dir_all(&dir1_target).await.unwrap();
        fs::write(&file1, "file1").await.unwrap();
        fs::write(&file2, "file2").await.unwrap();
        fs::write(&file3, "file3").await.unwrap();
        fs::write(&file1_target, "file1_other").await.unwrap();
        fs::write(&file4, "file4").await.unwrap();
        fs::write(&file2_target, "file2").await.unwrap();
        fs::write(&file5, "file5").await.unwrap();

        let mappings = hashmap! {
            file1_target.clone() => file1.clone(),
            file2_target.clone() => file2.clone(),
            target_dir.join("dir2") => dir2.clone(),
        };

        sync_mapping(&target_dir, &mappings).await.unwrap();

        assert!(file1_target.exists());
        assert!(fs::read_to_string(&file1_target).await.unwrap() == "file1");
        assert!(file2_target.exists());
        assert!(fs::read_to_string(&file2_target).await.unwrap() == "file2");
        assert!(target_dir.join("dir2").join("file3").exists());
        assert!(!file4.exists());
        assert!(!file5.exists());

        fs::remove_dir_all(&source_dir).await.unwrap();
        fs::remove_dir_all(&target_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_download_entries_multi_algo() {
        let dir = env::temp_dir().join("files_multi_algo_test");
        let _ = fs::remove_dir_all(&dir).await;
        fs::create_dir_all(&dir).await.unwrap();

        let f1 = dir.join("a");
        let f2 = dir.join("b");
        let f3 = dir.join("c");
        fs::write(&f1, "alpha").await.unwrap();
        fs::write(&f2, "beta").await.unwrap();
        fs::write(&f3, "gamma").await.unwrap();

        // Build entries whose expected hash is the file's correct digest under its own algorithm.
        let mut entries = vec![
            CheckEntry {
                url: "http://example/a".into(),
                remote_sha1: Some(hash_file_algo(&f1, HashAlgo::Sha1).await.unwrap()),
                algo: HashAlgo::Sha1,
                path: f1.clone(),
            },
            CheckEntry {
                url: "http://example/b".into(),
                remote_sha1: Some(hash_file_algo(&f2, HashAlgo::Sha256).await.unwrap()),
                algo: HashAlgo::Sha256,
                path: f2.clone(),
            },
            CheckEntry {
                url: "http://example/c".into(),
                remote_sha1: Some(hash_file_algo(&f3, HashAlgo::Sha512).await.unwrap()),
                algo: HashAlgo::Sha512,
                path: f3.clone(),
            },
        ];

        // All present and matching under their own algorithm => nothing to download.
        let to_download = get_download_entries(entries, crate::progress::no_progress_bar())
            .await
            .unwrap();
        assert!(
            to_download.is_empty(),
            "expected no downloads, got {to_download:?}"
        );

        // Corrupt the expected hash of the SHA-512 entry => exactly that one re-downloads.
        entries = vec![
            CheckEntry {
                url: "http://example/a".into(),
                remote_sha1: Some(hash_file_algo(&f1, HashAlgo::Sha1).await.unwrap()),
                algo: HashAlgo::Sha1,
                path: f1.clone(),
            },
            CheckEntry {
                url: "http://example/b".into(),
                remote_sha1: Some(hash_file_algo(&f2, HashAlgo::Sha256).await.unwrap()),
                algo: HashAlgo::Sha256,
                path: f2.clone(),
            },
            CheckEntry {
                url: "http://example/c".into(),
                remote_sha1: Some("deadbeef".into()),
                algo: HashAlgo::Sha512,
                path: f3.clone(),
            },
        ];
        let to_download = get_download_entries(entries, crate::progress::no_progress_bar())
            .await
            .unwrap();
        assert_eq!(to_download.len(), 1);
        assert_eq!(to_download[0].path, f3);

        fs::remove_dir_all(&dir).await.unwrap();
    }
}
