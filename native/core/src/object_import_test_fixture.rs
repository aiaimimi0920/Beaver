use super::super::*;
use crate::{object_catalog::ObjectVersion, project_storage::ProjectStore, store::Store};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
    time::SystemTime,
};

pub(super) struct Fixture {
    pub target: Arc<Mutex<Store>>,
    pub sources: Arc<ProjectStorageRouter>,
    pub host: Arc<Mutex<Store>>,
    pub source: PathBuf,
    pub temp: tempfile::TempDir,
}

pub(super) fn accepted(object_id: &str, version_id: &str) -> Value {
    json!({
        "schemaVersion":1, "projectId":"source-1", "objectId":object_id,
        "versionId":version_id, "status":"accepted", "name":format!("Frozen {object_id}"),
        "category":"其他", "tags":[], "thumbnailPath":null, "parentObjectId":null,
        "components":[{"id":format!("{object_id}-component"),"kind":"scene","name":"Frozen scene"}],
        "files":[], "references":[]
    })
}

pub(super) fn reference(object_id: &str, version_id: &str) -> Value {
    json!({"projectId":"source-1","objectId":object_id,"versionId":version_id})
}

pub(super) fn record(object_id: &str, manifests: Vec<Value>) -> ObjectRecord {
    ObjectRecord {
        id: object_id.into(),
        project_id: "source-1".into(),
        name: "Working metadata".into(),
        category: "其他".into(),
        tags: vec![],
        thumbnail_path: None,
        parent_object_id: None,
        revision: 0,
        components: vec![],
        files: vec![],
        references: vec![],
        versions: manifests
            .into_iter()
            .map(|manifest| ObjectVersion {
                version_id: manifest["versionId"].as_str().unwrap().into(),
                manifest,
            })
            .collect(),
    }
}

pub(super) fn file(path: &str, content: &str) -> Value {
    json!({"path":path,"role":"scene","bytes":content.len(),
        "sha256":format!("{:x}",Sha256::digest(content.as_bytes()))})
}

impl Fixture {
    pub fn new(objects: Vec<ObjectRecord>) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        fs::create_dir(&source)?;
        fs::write(
            source.join("project.godot"),
            "[application]\nconfig/name=Source\n",
        )?;
        let mut source_store = ProjectStore::initialize(&source, "source-1")?;
        source_store
            .store_mut()
            .put("project", "source-1", &json!({"id":"source-1"}))?;
        for object in objects {
            // Imported history fixtures deliberately include unsupported manifests.
            source_store.store().put("object", &object.id, &object)?;
            for version in &object.versions {
                source_store.store().put(
                    "object_version",
                    &version.version_id,
                    &(object.id.clone(), version),
                )?;
            }
        }
        drop(source_store);
        let target = Store::open(&temp.path().join("target"))?;
        target.put("project", "target-1", &json!({"id":"target-1"}))?;
        let host = Arc::new(Mutex::new(Store::open(&temp.path().join("host"))?));
        Ok(Self {
            target: Arc::new(Mutex::new(target)),
            sources: Arc::new(ProjectStorageRouter::new(host.clone())),
            host,
            source,
            temp,
        })
    }

    pub fn register_source(&self) -> Result<()> {
        self.host.lock().unwrap().put(
            "project",
            "source-1",
            &json!({
                "id":"source-1", "name":"Source", "path":self.source
            }),
        )
    }

    pub fn target(&self) -> MutexGuard<'_, Store> {
        self.target.lock().unwrap()
    }

    pub fn prepare(&self, request: &Request) -> Result<Preparation> {
        prepare(&self.target, &self.sources, request)
    }

    pub fn blob(&self, content: &str) -> Result<PathBuf> {
        let directory = self.source.join(".beaver/content/blobs");
        fs::create_dir_all(&directory)?;
        let path = directory.join(format!("{:x}", Sha256::digest(content.as_bytes())));
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn request(&self, version_id: &str) -> Request {
        Request {
            request_id: "request-1".into(),
            target_project_id: "target-1".into(),
            source_path: self.source.clone(),
            source_project_id: "source-1".into(),
            object_id: "hero".into(),
            baseline: Baseline::PinnedVersion(version_id.into()),
            source_digest: "0".repeat(64),
        }
    }

    pub fn inspected(&self, version_id: &str) -> Result<Request> {
        let snapshot =
            self.sources
                .inspect_object_source(&self.source, Some("source-1"), Some("hero"))?;
        let option = snapshot
            .import_versions
            .iter()
            .find(|version| version.version_id == version_id)
            .context("test version missing")?;
        let mut request = self.request(version_id);
        request.source_digest = option
            .source_digest
            .clone()
            .with_context(|| format!("test selection blocked: {:?}", option.blocker))?;
        Ok(request)
    }

    pub fn assert_no_imported_entities(&self) -> Result<()> {
        let target = self.target();
        for kind in ["object", "object_version", "task", "asset_task"] {
            assert!(target.list::<Value>(kind)?.is_empty(), "unexpected {kind}");
        }
        Ok(())
    }

    pub fn assert_rejected(&self, request: &Request, code: &str) -> Result<()> {
        let before = inventory(&self.source)?;
        let error = self.prepare(request).unwrap_err();
        assert!(
            error.to_string().contains(code),
            "expected {code}, got {error:#}"
        );
        assert_eq!(inventory(&self.source)?, before);
        assert!(self.target().list::<Value>(PREPARATION_KIND)?.is_empty());
        self.assert_no_imported_entities()
    }
}

pub(super) fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, (Vec<u8>, SystemTime)>> {
    fn visit(
        root: &Path,
        path: &Path,
        output: &mut BTreeMap<PathBuf, (Vec<u8>, SystemTime)>,
    ) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                visit(root, &entry.path(), output)?;
            } else if entry.file_type()?.is_file() {
                let entry_path = entry.path();
                let relative = entry_path.strip_prefix(root)?;
                if relative
                    == Path::new(crate::project_storage_layout::CONTROL_DIR)
                        .join(crate::project_storage_layout::LOCK)
                {
                    // The project lock is runtime ownership evidence, not source content.
                    continue;
                }
                output.insert(
                    relative.into(),
                    (fs::read(entry_path)?, entry.metadata()?.modified()?),
                );
            }
        }
        Ok(())
    }
    let mut output = BTreeMap::new();
    visit(root, root, &mut output)?;
    Ok(output)
}
