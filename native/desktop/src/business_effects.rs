pub(crate) struct DataEffects {
    pub(crate) wake_scheduler: bool,
    pub(crate) notify: bool,
}

pub(crate) fn data_effects(method: &str) -> DataEffects {
    let wake_scheduler = matches!(
        method,
        "document.save"
            | "project.import"
            | "project.reassociate"
            | "project.unregister"
            | "settings.save"
            | "settings.importLocalCodex"
            | "settings.clearKey"
            | "task.rollback"
            | "task.accept"
            | "task.retryMerge"
            | "project.blueprint.save"
            | "project.overview.save"
            | "task.create"
            | "task.followup"
            | "task.delegate"
            | "task.dialogueRollback"
            | "task.direction"
            | "task.autonomy"
            | "task.approval"
            | "feature.add"
            | "objectTask.enqueue"
            | "objectTask.reorder"
            | "objectTask.setPaused"
            | "objectTask.setCoarsePaused"
    );
    let notify = wake_scheduler
        || matches!(
            method,
            "object.register"
                | "object.updateRegistration"
                | "object.captureVersion"
                | "object.acceptVersion"
                | "object.scenePreview.run"
                | "object.attemptScenePreview.run"
                | "objectTask.saveDraft"
                | "objectTask.unlockDraft"
                | "objectTask.commit"
                | "objectTask.cancelPlanned"
                | "objectTask.revisePlanned"
                | "objectTask.declarePlanningComplete"
                | "objectTask.claim"
                | "objectTask.finish"
                | "objectTask.cancelClaim"
        );
    DataEffects {
        wake_scheduler,
        notify,
    }
}

#[cfg(test)]
mod tests {
    use super::data_effects;

    #[test]
    fn object_enqueue_and_dispatch_control_wake_scheduler() {
        for method in [
            "objectTask.saveDraft",
            "objectTask.unlockDraft",
            "objectTask.commit",
            "objectTask.cancelPlanned",
            "objectTask.revisePlanned",
            "objectTask.declarePlanningComplete",
            "objectTask.claim",
            "objectTask.finish",
            "objectTask.cancelClaim",
        ] {
            let effects = data_effects(method);
            assert!(effects.notify, "{method}");
            assert!(!effects.wake_scheduler, "{method}");
        }
        for method in [
            "objectTask.getDraft",
            "objectTask.queue",
            "objectTask.queueView",
            "objectTask.snapshot",
            "objectTask.get",
            "objectTask.getRun",
        ] {
            let effects = data_effects(method);
            assert!(!effects.notify, "{method}");
            assert!(!effects.wake_scheduler, "{method}");
        }
        let legacy = data_effects("task.create");
        assert!(legacy.notify && legacy.wake_scheduler);
        let enqueue = data_effects("objectTask.enqueue");
        assert!(enqueue.notify && enqueue.wake_scheduler);
        let reorder = data_effects("objectTask.reorder");
        assert!(reorder.notify && reorder.wake_scheduler);
        let pause = data_effects("objectTask.setPaused");
        assert!(pause.notify && pause.wake_scheduler);
        let coarse_pause = data_effects("objectTask.setCoarsePaused");
        assert!(coarse_pause.notify && coarse_pause.wake_scheduler);
    }
}
