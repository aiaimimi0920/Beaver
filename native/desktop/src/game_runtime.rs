use crate::{game_storage::GameStorage, Backend};
use anyhow::{bail, ensure, Context, Result};
use beaver_core::{
    export_bundle, game_export, journal::Journal, preferences, process, tools, validation,
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::Duration,
};

pub fn prepare_play(backend: &Backend, input: Value) -> Result<(PathBuf, PathBuf), String> {
    (|| -> Result<_> {
        let _operation = backend
            .operations
            .lock()
            .map_err(|_| anyhow::anyhow!("工具操作锁不可用"))?;
        if backend.closing.load(Ordering::SeqCst) {
            bail!("应用正在退出");
        }
        let id = input["id"].as_str().context("缺少项目标识")?;
        let storage = GameStorage::for_project(backend, id)?;
        let settings = {
            let host = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            preferences::read(
                &host,
                serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
            )?
        };
        let (root, configured) = {
            let mut store = storage
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
            if Journal::new(&mut store, &storage.files).blocked(id)? {
                bail!("项目存在未完成的文件恢复，禁止试玩");
            }
            let root = storage.root.clone();
            if !beaver_core::files::safe_path(&root, "project.godot")?.is_file() {
                bail!("项目缺少 project.godot");
            }
            (
                root,
                beaver_core::workflows::configured_engine(
                    &storage.project,
                    settings["tools"]["godot"].as_str().unwrap_or(""),
                )?,
            )
        };
        let godot = tools::find("godot", &configured)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let operations = storage.scratch.join("operations");
        std::fs::create_dir_all(&operations)?;
        let log = operations.join(format!("play-import-{stamp}.log"));
        // Merged projects exclude .godot; first play must rebuild imported assets.
        game_export::import_project(&godot, &root, &log, &backend.closing)?;
        Ok((godot, root))
    })()
    .map_err(|error| error.to_string())
}

pub fn call(backend: &Backend, method: &str, input: Value) -> Result<Value, String> {
    let mut authorized: Option<validation::model::Release> = None;
    let mut result = (|| -> Result<Value> {
        if backend.closing.load(Ordering::SeqCst) {
            bail!("应用正在退出");
        }
        if method == "game.verifyExport" {
            let path = input["path"]
                .as_str()
                .filter(|s| !s.is_empty() && s.encode_utf16().count() <= 2000)
                .context("导出目录无效")?;
            return export_bundle::verify(Path::new(path));
        }
        let id = input["id"].as_str().context("缺少项目标识")?;
        let purpose = input["purpose"].as_str().unwrap_or("internal");
        ensure!(matches!(purpose, "internal" | "formal"), "未知的导出用途");
        let storage = GameStorage::for_project(backend, id)?;
        let settings = {
            let host = backend
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("数据库锁不可用"))?;
            preferences::read(
                &host,
                serde_json::from_str(include_str!("../../../dist-native/default-settings.json"))?,
            )?
        };
        let (job, configured, expected_engine) = {
            let _operation = backend
                .operations
                .lock()
                .map_err(|_| anyhow::anyhow!("工具操作锁不可用"))?;
            let mut store = storage
                .store
                .lock()
                .map_err(|_| anyhow::anyhow!("项目数据库锁不可用"))?;
            let project = &storage.project;
            if method == "game.presets" {
                return Ok(json!(game_export::presets(Path::new(
                    project["path"].as_str().context("项目路径无效")?
                ))?));
            }
            if Journal::new(&mut store, &storage.files).blocked(id)? {
                bail!("项目存在未完成的文件恢复，禁止导出");
            }
            let configured = beaver_core::workflows::configured_engine(
                project,
                settings["tools"]["godot"].as_str().unwrap_or(""),
            )?;
            let destination = Path::new(input["destination"].as_str().context("缺少导出目录")?);
            let preset = input["preset"].as_str().context("缺少导出预设")?;
            if purpose == "formal" {
                let release = validation::release::authorize(&store, &storage.files, &input)?;
                let version = validation::release::engine_version(&store, &release)?;
                let provenance = json!({"purpose":"formal","releaseCheckId":release.id,
                    "snapshotId":release.snapshot_id,"scopeId":release.scope_id,
                    "visualRequired":release.visual_required,"runIds":release.run_ids,
                    "policyVersion":release.policy_version,"engineVersion":version});
                let job = game_export::prepare_snapshot(
                    &storage.files,
                    &storage.scratch,
                    project,
                    destination,
                    preset,
                    &release.snapshot,
                )?
                .with_provenance(provenance);
                authorized = Some(release);
                (job, configured, Some(version))
            } else {
                (
                    game_export::prepare(
                        &storage.files,
                        &storage.scratch,
                        project,
                        destination,
                        preset,
                    )?,
                    configured,
                    None,
                )
            }
        };
        let godot = tools::find("godot", &configured)?;
        if let Some(expected) = expected_engine {
            let actual = process::run_cancellable(
                &godot,
                &["--version"],
                None,
                Duration::from_secs(15),
                &backend.closing,
            )?;
            ensure!(
                actual.code == 0 && actual.text.trim() == expected,
                "Godot 版本与发布诊断不同，请使用相同引擎或重新诊断"
            );
        }
        game_export::execute(job, &godot, &backend.closing)
    })()
    .map_err(|error| error.to_string());
    if matches!(method, "game.export" | "game.verifyExport") {
        let storage = if let Some(id) = input["id"].as_str() {
            Some(GameStorage::for_project(backend, id).map_err(|error| error.to_string())?)
        } else if let Some(path) = input["path"].as_str() {
            GameStorage::for_export_path(backend, path).map_err(|error| error.to_string())?
        } else {
            None
        };
        let Some(storage) = storage else {
            return result;
        };
        let mut store = storage.store.lock().map_err(|_| "项目数据库锁不可用")?;
        if let Some(release) = authorized {
            let mut current: validation::model::Release =
                validation::repository::get(&store, "validationRelease", &release.id)
                    .map_err(|error| error.to_string())?;
            let mut receipt = game_export::delivery_record(&result);
            receipt["releaseCheckId"] = json!(release.id);
            receipt["snapshotId"] = json!(release.snapshot_id);
            receipt["scopeId"] = json!(release.scope_id);
            receipt["visualRequired"] = json!(release.visual_required);
            current.exports.push(receipt);
            store
                .put("validationRelease", &current.id, &current)
                .map_err(|error| error.to_string())?;
        }
        let mut project = storage.project.clone();
        if method == "game.verifyExport" {
            project["delivery"] = beaver_core::export_receipt::verification(
                &project["delivery"],
                input["path"].as_str().unwrap_or(""),
                &mut result,
            );
        } else {
            project["delivery"] = game_export::delivery_record(&result);
            if let Ok(value) = &result {
                project["delivery"]["validation"] = value["validation"].clone();
            }
        }
        store
            .put("project", &storage.project_id, &project)
            .map_err(|e| e.to_string())?;
    }
    result
}
