use super::*;
use anyhow::Result;

#[test]
fn removed_registration_cannot_claim_a_previously_selected_host_task() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(temp.path())?;
    let task = json!({"id":"t","projectId":"p","status":"queued"});
    store.put("project", "p", &json!({"id":"p"}))?;
    store.put("task", "t", &task)?;
    assert_eq!(
        revalidate_claims(&store, &RuntimeOwner::Host, vec![task.clone()])?,
        vec![task.clone()]
    );
    store.remove("project", "p")?;
    let selected = revalidate_claims(&store, &RuntimeOwner::Host, vec![task.clone()])?;
    assert!(prepare_claims(&mut store, &Files::new(temp.path().into()), selected)?.is_empty());
    assert_eq!(store.get::<Value>("task", "t")?, Some(task));
    Ok(())
}

#[test]
fn claim_rechecks_status_owner_and_dependencies_after_selection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let store = Store::open(temp.path())?;
    store.put("project", "p", &json!({"id":"p"}))?;
    let snapshot = json!({"id":"t","projectId":"p","status":"queued"});
    for current in [
        json!({"id":"t","projectId":"p","status":"interrupted"}),
        json!({"id":"t","projectId":"other","status":"queued"}),
        json!({"id":"t","projectId":"p","status":"queued","waitingDelivery":"delivery"}),
        json!({"id":"t","projectId":"p","status":"queued","dependsOn":["unfinished"]}),
        json!({"id":"t","projectId":"p","status":"queued","migrationRetained":true}),
    ] {
        store.put("task", "t", &current)?;
        assert!(revalidate_claims(&store, &RuntimeOwner::Host, vec![snapshot.clone()])?.is_empty());
        assert_eq!(store.get::<Value>("task", "t")?, Some(current));
    }
    store.remove("task", "t")?;
    assert!(revalidate_claims(&store, &RuntimeOwner::Host, vec![snapshot.clone()])?.is_empty());
    let mut current = snapshot.clone();
    current["text"] = json!("updated request");
    store.put("task", "t", &current)?;
    assert_eq!(
        revalidate_claims(&store, &RuntimeOwner::Host, vec![snapshot.clone()])?,
        vec![current]
    );
    assert!(revalidate_claims(
        &store,
        &RuntimeOwner::Project("other".into()),
        vec![snapshot]
    )?
    .is_empty());
    Ok(())
}
