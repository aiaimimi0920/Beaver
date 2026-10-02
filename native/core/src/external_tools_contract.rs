use serde_json::{json, Value};

pub fn contract() -> Value {
    let path = json!({"type":"string","description":"Workspace-relative slash-separated project file. Hidden/control paths, links, traversal and absolute paths are rejected.","maxLength":1024});
    let hash = json!({"type":["string","null"],"description":"Required. null approves creation only. Existing files require the current lowercase SHA-256 from file.read or a prior tool result.","pattern":"^[0-9a-f]{64}$"});
    let output = json!({"type":"object","properties":{"path":path,"expectedSha256":hash},"required":["path","expectedSha256"],"additionalProperties":false});
    let blender = json!({"type":"object","properties":{"code":{"type":"string","description":"Trusted Blender Python, maximum 1 MiB. Use beaver_input(relative) to read project inputs and beaver_output(relative) / BEAVER_OUTPUTS[relative] for all declared outputs. Files are staged and published after successful process exit. Save .blend with compress=False and pack or use portable relative texture references. This is NOT an OS or Python sandbox.","minLength":1},"outputs":{"type":"array","items":output,"minItems":1,"maxItems":64},"timeoutSeconds":{"type":"integer","minimum":1,"maximum":3600,"default":300}},"required":["code","outputs"],"additionalProperties":false});
    let job = json!({"type":"object","properties":{"jobId":{"type":"string"}},"required":["jobId"],"additionalProperties":false});
    let mut tools = vec![
        json!({"name":"file.read","description":"Read up to 1 MiB of UTF-8 project content with SHA-256.","inputSchema":{"type":"object","properties":{"path":path},"required":["path"],"additionalProperties":false}}),
        json!({"name":"file.write","description":"Write up to 1 MiB of UTF-8 project content with expected-hash protection. Requires no active Blender job.","inputSchema":{"type":"object","properties":{"path":path,"content":{"type":"string"},"expectedSha256":hash},"required":["path","content","expectedSha256"],"additionalProperties":false}}),
        json!({"name":"blender.start","description":"Start a real Beaver-owned background Blender process using the host-selected executable, factory startup and isolated HOME/XDG/TMP. No shell or arbitrary executable arguments. Returns jobId immediately; poll to terminal state. Maximum one active job per run; 256 starts per run.","inputSchema":blender}),
        json!({"name":"blender.poll","description":"Read the owned job's status, command, real PID, exit, log tail and published output hashes. Jobs from another run are inaccessible. Host-restart uncertainty never permits replay.","inputSchema":job}),
        json!({"name":"blender.cancel","description":"Request cancellation of only this run's owned job. Poll until terminal before finish; cancellation does not imply immediate exit.","inputSchema":job}),
    ];
    for mut tool in crate::workflows::tools() {
        tool["name"] = json!(if tool["name"] == "beaver_workflow_list" {
            "workflow.list"
        } else {
            "workflow.run"
        });
        tools.push(tool);
    }
    json!({"protocolVersion":1,"executionTrust":"trusted_python_not_os_sandbox","tools":tools})
}
