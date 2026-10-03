//! Tool permissions derive only from the persisted task, never request arguments.
use serde_json::{json, Value};

pub(crate) fn allowed(task: &Value, tool: &str, arguments: &Value) -> bool {
    crate::external_run_contract::enabled(task)
        && (matches!(tool, "file.read" | "workflow.list")
            || (tool == "workflow.run" && arguments["action"] == "inspect")
            || (task["capability"] == "code" && task["decompose"] != true))
}

pub(crate) fn contract(task: &Value) -> Value {
    let mut contract = crate::external_tools::contract();
    if let Some(tools) = contract["tools"].as_array_mut() {
        tools.retain(|tool| {
            tool["name"]
                .as_str()
                .is_some_and(|name| allowed(task, name, &json!({"action":"inspect"})))
        });
        if task["decompose"] == true || task["capability"] == "review" {
            for tool in tools
                .iter_mut()
                .filter(|tool| tool["name"] == "workflow.run")
            {
                tool["inputSchema"]["properties"]["action"]["enum"] = json!(["inspect"]);
            }
        }
    }
    contract["taskAccess"] = json!(if task["decompose"] == true {
        "read-only planning; submitPlan creates separately scheduled child tasks"
    } else if task["capability"] == "review" {
        "read-only review"
    } else {
        "task-workspace production"
    });
    contract
}
