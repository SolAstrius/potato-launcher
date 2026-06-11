use std::collections::HashSet;

use log::{error, info};
use shared::packwiz::{PackwizInstance, fetch_index_hash, generate_packwiz_instance};
use shared::utils::is_connect_error;
use shared::version::extra_version_metadata::AuthBackend;
use tokio::runtime::Runtime;

use crate::config::runtime_config::Config;
use crate::version::instance_storage::{InstanceStorage, LocalInstance};

use super::background_task::{BackgroundTask, BackgroundTaskResult};

/// What a provisioning run resolved to.
enum Outcome {
    /// Local generation already matches the pack index hash (or offline with a usable local copy).
    UpToDate,
    /// (Re)generated the instance from the pack.
    Generated(Box<PackwizInstance>),
    Error(String),
}

struct ProvisionResult {
    instance_name: String,
    pack_url: String,
    auth_backend: Option<AuthBackend>,
    recommended_xmx: Option<String>,
    outcome: Outcome,
}

/// Drives local generation of packwiz instances that arrive as descriptors in the version
/// manifest (`VersionInfo.packwiz_url`). On selecting such an instance the launcher fetches the
/// `pack.toml`, and if the pack's index hash differs from what was last generated (or nothing was
/// generated yet) it regenerates the instance locally. This is both the first-run provisioning
/// and the update-detection path.
pub struct PackwizProvisionState {
    task: Option<BackgroundTask<ProvisionResult>>,
    /// Instances already checked this session, so we don't re-hit the network every frame.
    checked: HashSet<String>,
    in_progress: Option<String>,
    last_error: Option<String>,
}

/// Is this selected instance a packwiz one, and if so what is its `pack.toml` URL?
/// A manifest descriptor carries it on `version_info.packwiz_url`; an already-generated local
/// instance carries it on `manifest_url` (with `packwiz_index_hash` set as the marker).
fn pack_url_of(instance: &LocalInstance) -> Option<String> {
    if let Some(url) = &instance.version_info.packwiz_url {
        Some(url.clone())
    } else if instance.packwiz_index_hash.is_some() {
        instance.manifest_url.clone()
    } else {
        None
    }
}

pub fn is_packwiz(instance: &LocalInstance) -> bool {
    pack_url_of(instance).is_some()
}

/// Auth backend + recommended Xmx the server set for this packwiz instance. Prefer the manifest
/// descriptor's fields; fall back to what was persisted locally (so regeneration keeps them).
fn server_overrides(instance: &LocalInstance) -> (Option<AuthBackend>, Option<String>) {
    let auth = instance
        .version_info
        .packwiz_auth_backend
        .clone()
        .or_else(|| instance.packwiz_auth_backend.clone());
    let xmx = instance
        .version_info
        .packwiz_recommended_xmx
        .clone()
        .or_else(|| instance.packwiz_recommended_xmx.clone());
    (auth, xmx)
}

/// True for a packwiz instance that has not been generated locally yet (manifest descriptor).
/// Its `version_info` has placeholder URLs, so metadata must not be loaded until provisioned.
pub fn needs_provisioning(instance: &LocalInstance) -> bool {
    instance.version_info.packwiz_url.is_some()
}

impl PackwizProvisionState {
    pub fn new() -> Self {
        Self {
            task: None,
            checked: HashSet::new(),
            in_progress: None,
            last_error: None,
        }
    }

    pub fn is_working(&self) -> bool {
        self.task.is_some()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Start a provisioning/update check for the selected instance if needed. Self-gating: only
    /// one task at a time, and each instance is checked once per session (until `invalidate`).
    pub fn maybe_start(
        &mut self,
        runtime: &Runtime,
        config: &Config,
        selected: &LocalInstance,
        ctx: &egui::Context,
    ) {
        if self.task.is_some() {
            return;
        }
        let Some(pack_url) = pack_url_of(selected) else {
            return;
        };
        let name = selected.version_info.get_name();
        if self.checked.contains(&name) {
            return;
        }

        let stored_hash = selected.packwiz_index_hash.clone();
        let already_generated = stored_hash.is_some();
        let launcher_dir = config.get_launcher_dir();
        let (auth_backend, recommended_xmx) = server_overrides(selected);
        let task_name = name.clone();
        let task_pack_url = pack_url.clone();
        let task_auth = auth_backend.clone();
        let task_xmx = recommended_xmx.clone();

        self.in_progress = Some(name);
        self.last_error = None;

        let fut = async move {
            let outcome = async {
                match fetch_index_hash(&task_pack_url).await {
                    Ok(current_hash) => {
                        if already_generated
                            && stored_hash.as_deref() == Some(current_hash.as_str())
                        {
                            return Outcome::UpToDate;
                        }
                        info!("Provisioning packwiz instance {task_name} from {task_pack_url}");
                        match generate_packwiz_instance(
                            &launcher_dir,
                            &task_pack_url,
                            &task_name,
                            task_auth,
                            task_xmx,
                        )
                        .await
                        {
                            Ok(inst) => Outcome::Generated(Box::new(inst)),
                            Err(e) => {
                                error!("Failed to provision packwiz instance {task_name}:\n{e:?}");
                                Outcome::Error(e.to_string())
                            }
                        }
                    }
                    Err(e) => {
                        // Offline but already generated locally: fall back to the local copy.
                        if already_generated && is_connect_error(&e) {
                            Outcome::UpToDate
                        } else {
                            error!("Failed to fetch pack.toml for {task_name}:\n{e:?}");
                            Outcome::Error(e.to_string())
                        }
                    }
                }
            }
            .await;

            ProvisionResult {
                instance_name: task_name,
                pack_url: task_pack_url,
                auth_backend,
                recommended_xmx,
                outcome,
            }
        };

        let ctx = ctx.clone();
        self.task = Some(BackgroundTask::with_callback(
            fut,
            runtime,
            Box::new(move || ctx.request_repaint()),
        ));
    }

    /// Apply a finished provisioning run. Returns `true` if the stored instance changed (newly
    /// generated or regenerated), so the caller should reload metadata and reset sync status.
    pub fn update(
        &mut self,
        runtime: &Runtime,
        config: &Config,
        storage: &mut InstanceStorage,
    ) -> bool {
        let Some(task) = self.task.as_ref() else {
            return false;
        };
        if !task.has_result() {
            return false;
        }
        let task = self.task.take().unwrap();
        self.in_progress = None;

        match task.take_result() {
            BackgroundTaskResult::Finished(result) => {
                self.checked.insert(result.instance_name.clone());
                match result.outcome {
                    Outcome::Generated(inst) => {
                        runtime.block_on(storage.add_packwiz_instance(
                            config,
                            inst.version_info,
                            result.pack_url,
                            inst.index_hash,
                            result.auth_backend,
                            result.recommended_xmx,
                        ));
                        true
                    }
                    Outcome::UpToDate => false,
                    Outcome::Error(e) => {
                        self.last_error = Some(e);
                        false
                    }
                }
            }
            BackgroundTaskResult::Cancelled => false,
        }
    }

    /// Force a re-check of all instances (e.g. after the manifest is refetched).
    pub fn invalidate_all(&mut self) {
        self.checked.clear();
    }
}
