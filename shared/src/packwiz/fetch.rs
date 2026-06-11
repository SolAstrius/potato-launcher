use futures::stream::{FuturesUnordered, StreamExt};

use super::model::{IndexToml, Metafile, PackToml};

/// The parent "directory" of the pack.toml URL, without a trailing slash.
/// `https://host/pack/pack.toml` -> `https://host/pack`. A query string, if any, is dropped.
pub fn pack_base_url(pack_toml_url: &str) -> String {
    let without_query = pack_toml_url
        .split(['?', '#'])
        .next()
        .unwrap_or(pack_toml_url);
    match without_query.rsplit_once('/') {
        Some((base, _file)) => base.to_string(),
        None => without_query.to_string(),
    }
}

/// Join a pack-relative path onto the base URL.
pub fn join_url(base_url: &str, rel_path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        rel_path.trim_start_matches('/')
    )
}

pub async fn fetch_bytes(client: &reqwest::Client, url: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}

pub async fn fetch_pack(client: &reqwest::Client, pack_toml_url: &str) -> anyhow::Result<PackToml> {
    let bytes = fetch_bytes(client, pack_toml_url).await?;
    Ok(toml::from_str(std::str::from_utf8(&bytes)?)?)
}

/// Returns the parsed index plus its raw bytes, so the caller can verify the index hash.
pub async fn fetch_index(
    client: &reqwest::Client,
    index_url: &str,
) -> anyhow::Result<(IndexToml, Vec<u8>)> {
    let bytes = fetch_bytes(client, index_url).await?;
    let index: IndexToml = toml::from_str(std::str::from_utf8(&bytes)?)?;
    Ok((index, bytes))
}

const MAX_CONCURRENT_METAFILE_FETCHES: usize = 20;

/// Concurrently fetch + parse every metafile, keyed by its pack-relative index path.
pub async fn fetch_metafiles(
    client: &reqwest::Client,
    base_url: &str,
    metafile_paths: Vec<String>,
) -> anyhow::Result<Vec<(String, Metafile)>> {
    async fn fetch_one(
        client: reqwest::Client,
        base_url: String,
        rel_path: String,
    ) -> anyhow::Result<(String, Metafile)> {
        let url = join_url(&base_url, &rel_path);
        let bytes = fetch_bytes(&client, &url).await?;
        let meta: Metafile = toml::from_str(std::str::from_utf8(&bytes)?)?;
        Ok((rel_path, meta))
    }

    let mut tasks = FuturesUnordered::new();
    let mut iter = metafile_paths.into_iter();

    for _ in 0..MAX_CONCURRENT_METAFILE_FETCHES {
        if let Some(path) = iter.next() {
            tasks.push(fetch_one(client.clone(), base_url.to_string(), path));
        }
    }

    let mut results = Vec::new();
    while let Some(result) = tasks.next().await {
        results.push(result?);
        if let Some(path) = iter.next() {
            tasks.push(fetch_one(client.clone(), base_url.to_string(), path));
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_strips_file() {
        assert_eq!(
            pack_base_url("https://mc.sol.moe/pack/pack.toml"),
            "https://mc.sol.moe/pack"
        );
    }

    #[test]
    fn base_url_strips_query() {
        assert_eq!(
            pack_base_url("https://h/p/pack.toml?ref=main"),
            "https://h/p"
        );
    }

    #[test]
    fn join_handles_slashes() {
        assert_eq!(
            join_url("https://h/p", "config/x.json"),
            "https://h/p/config/x.json"
        );
        assert_eq!(
            join_url("https://h/p/", "/config/x.json"),
            "https://h/p/config/x.json"
        );
    }
}
