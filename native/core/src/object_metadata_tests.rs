use super::*;
use anyhow::Result;
use serde_json::json;

fn store() -> (tempfile::TempDir, crate::store::Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = crate::store::Store::open(temp.path()).unwrap();
    store
        .put("project", "project-1", &json!({"id":"project-1"}))
        .unwrap();
    (temp, store)
}

fn object(id: &str) -> ObjectRecord {
    ObjectRecord {
        id: id.into(),
        project_id: "project-1".into(),
        name: id.into(),
        category: default_category(),
        tags: vec![],
        thumbnail_path: None,
        parent_object_id: None,
        revision: 0,
        components: vec![],
        files: vec![],
        references: vec![],
        versions: vec![],
    }
}

#[test]
fn metadata_defaults_keep_legacy_records_readable() -> Result<()> {
    let record: ObjectRecord = serde_json::from_value(json!({
        "id":"object-1", "projectId":"project-1", "name":"Legacy",
        "components":[], "files":[], "references":[], "versions":[]
    }))?;
    assert_eq!(record.category, "其他");
    assert!(record.tags.is_empty());
    assert!(record.thumbnail_path.is_none());
    assert!(record.parent_object_id.is_none());
    Ok(())
}

#[test]
fn parent_must_exist_and_stay_in_the_same_project() -> Result<()> {
    let (_temp, mut store) = store();
    let mut child = object("child");
    child.parent_object_id = Some("missing".into());
    assert!(insert(&store.connection, &child).is_err());

    let mut foreign = object("foreign");
    foreign.project_id = "project-2".into();
    store.put("project", "project-2", &json!({"id":"project-2"}))?;
    insert(&store.connection, &foreign)?;
    child.parent_object_id = Some("foreign".into());
    assert!(insert(&store.connection, &child).is_err());
    Ok(())
}

#[test]
fn parent_self_and_cycles_are_rejected() -> Result<()> {
    let (_temp, mut store) = store();
    let mut self_parent = object("self");
    self_parent.parent_object_id = Some("self".into());
    assert!(insert(&store.connection, &self_parent).is_err());

    let parent = object("parent");
    insert(&store.connection, &parent)?;
    let mut child = object("child");
    child.parent_object_id = Some("parent".into());
    insert(&store.connection, &child)?;
    let mut cycle = parent;
    cycle.parent_object_id = Some("child".into());
    assert!(validate_write(&store.connection, &cycle).is_err());
    Ok(())
}

#[test]
fn metadata_is_searchable() -> Result<()> {
    let (_temp, mut store) = store();
    let mut record = object("object-1");
    record.category = "角色".into();
    record.tags = vec!["Hero".into()];
    record.thumbnail_path = Some("art/hero.png".into());
    insert(&store.connection, &record)?;
    for query in ["角色", "hero", "art/hero", "object-1"] {
        assert_eq!(
            search(&store, "project-1", Some(query))?.len(),
            1,
            "{query}"
        );
    }
    Ok(())
}
