use crate::project_runtime_lifecycle;
use anyhow::{anyhow, Result};
use beaver_core::{
    codex_read_only,
    execution_settings::ExecutionSettings,
    object_task_title::{Request, INSTRUCTIONS},
    object_task_title_service::Service,
    preferences,
    project_storage_router::ProjectStorageRouter,
    store::Store,
    tools,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub async fn call(
    service: &Service,
    host: Arc<Mutex<Store>>,
    router: &ProjectStorageRouter,
    input: Value,
) -> Result<Value> {
    crate::business_catalog::validate("objectTask.suggestTitle", &input)
        .map_err(|(_, message)| anyhow!(message))?;
    let request: Request = serde_json::from_value(input)?;
    request.validate().map_err(anyhow::Error::msg)?;
    let runtime = project_runtime_lifecycle::open_registered(router, &request.project_id)?;
    let title = service
        .suggest(request, move || {
            // Retain the admitted project while reading host configuration.
            let _runtime = runtime;
            let settings = {
                let store = host.lock().map_err(|_| anyhow!("数据库锁不可用"))?;
                ExecutionSettings::read(
                    &store,
                    &preferences::SystemVault,
                    serde_json::from_str(include_str!(
                        "../../../dist-native/default-settings.json"
                    ))?,
                    Some("code"),
                )?
            };
            let codex = tools::find("codex", settings.tools["codex"].as_str().unwrap_or(""))?;
            codex_read_only::prepare(
                &settings,
                &codex,
                INSTRUCTIONS,
                std::env::vars_os().collect(),
            )
        })
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(json!({"title":title}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn validates_contract_and_project_before_launch() -> Result<()> {
        let fixture = crate::object_task_test_fixture::Fixture::new()?;
        let service = Service::default();
        for input in [
            json!({"projectId":"p","prompt":"Build","acceptance":"","extra":true}),
            json!({"projectId":"p","prompt":"","acceptance":""}),
            json!({"projectId":"missing","prompt":"Build","acceptance":""}),
            json!({"projectId":"legacy","prompt":"Build","acceptance":""}),
        ] {
            assert!(call(&service, fixture.host.clone(), &fixture.router, input)
                .await
                .is_err());
        }
        let tool = crate::business_catalog::tools()
            .into_iter()
            .find(|tool| tool["name"] == "objectTask.suggestTitle")
            .unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        service.shutdown().await.map_err(anyhow::Error::msg)?;
        Ok(())
    }
}
