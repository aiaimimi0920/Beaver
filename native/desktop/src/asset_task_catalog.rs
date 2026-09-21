pub(crate) fn definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str, bool)> {
    vec![
        ("assetTask.deliveryState", "Inspect immutable delivery candidates, approved input versions and owner decisions.", "id:s", "", true),
        ("assetTask.deliveryFile", "Read a frozen candidate file as base64 (16 MiB maximum). Never reads the live workspace.", "id:s,candidateId:s,path:s", "", true),
        ("assetTask.deliveryExport", "Export a candidate's verified files to a new Beaver-owned directory; returns its absolute path.", "id:s,candidateId:s", "", false),
        ("assetTask.deliveryDecide", "Owner-only approve, reject or reopen. Waits for the executor to stop, verifies approval files, then queues only the current stage. Use the latest asset revision and a unique requestId.", "id:s,requestId:s,expectedRevision:i,candidateId:s,decision:s,note:s", "", false),
        ("assetTask.open", "Open or focus the independent window bound to an asset task.", "id:s", "", false),
        ("assetTask.state", "Read durable asset stages, feedback and same-project task context.", "id:s", "", true),
        ("assetTask.status", "Read live observer status. A subscriber leases preview collection; release unsubscribes without stopping the task.", "id:s", "subscriber:s,release:b", true),
        ("assetTask.view", "Set the independent observer camera using a monotonically increasing seq. No model call or production camera mutation.", "id:s,sessionId:s,view:o", "", false),
        ("assetTask.freeze", "Persist the exact displayed frame as a bounded server-owned reference. Expired frames must be refreshed.", "id:s,sessionId:s,frameId:s", "", false),
        ("assetTask.pick", "Pick geometry on a frozen reference using normalized point [x,y]. Stale scene and session references are rejected.", "id:s,referenceId:s,point:a", "", false),
        ("assetTask.image", "Read one frozen reference as PNG base64 for low-frequency visual inspection.", "id:s,referenceId:s", "", true),
        ("assetTask.submit", "Persist feedback with an idempotency key. timing is now or afterRound. Returns its explicit destination task, including late followups.", "id:s,feedbackId:s,referenceId:s,text:s,timing:s", "annotations:a", false),
        ("assetTask.cancel", "Withdraw feedback that has not been delivered to the model.", "id:s,feedbackId:s", "", false),
    ]
}
