use super::*;

pub(super) fn historical(f: &Fixture) -> Result<(Request, deferred::Request, Value, Value)> {
    let (original, mut source) = image_feedback(f)?;
    let mut image = source.image.clone().unwrap();
    source
        .image
        .as_mut()
        .unwrap()
        .regions
        .push(resume::rework::image::Region {
            x: 0.7,
            y: 0.1,
            width: 0.2,
            height: 0.2,
            prompt: "Remove old badge".into(),
        });
    let source_frame = super::super::frames::attach_named(f, &mut source, "old-frame")?;
    let request = rework(f, &source, "change-output", false)?;
    let (_, lease) = resume::execute(&f.runtime, &request, &AtomicBool::new(false))?;
    let lease = lease.unwrap();
    image::RgbaImage::from_pixel(20, 10, image::Rgba([0, 255, 0, 255]))
        .save(workspace(f, lease.record())?.join("preview.png"))?;
    let publish = finish_candidate(f, lease, "new-output")?;
    assert_ne!(original.target.attempt_id, publish.target.attempt_id);
    let mut target = input(&publish);
    image.sha256 = publication::preview(&f.runtime, &target.review)?
        .paths
        .into_iter()
        .find(|p| p.path == "preview.png")
        .unwrap()
        .after
        .unwrap();
    target.image = Some(image);
    let target_frame = super::super::frames::attach_named(f, &mut target, "current-frame")?;
    target.relocation = Some(relocation::Confirmation {
        source_attempt_id: source.review.target.attempt_id,
        source_frame: source.preview_frame.unwrap(),
        regions: vec![
            relocation::Region::Matched {
                source_region: 0,
                target_region: 0,
            },
            relocation::Region::Absent {
                source_region: 1,
                note: "Badge removed in current output".into(),
            },
        ],
        confirmed: true,
    });
    Ok((publish, target, source_frame, target_frame))
}

pub(super) fn rework(
    f: &Fixture,
    feedback: &deferred::Request,
    id: &str,
    attach: bool,
) -> Result<resume::Request> {
    let verify =
        super::super::super::object_recovery_verification::request(f, &format!("verify-{id}"))?;
    recovery::verify(&f.runtime, &verify, false)?;
    Ok(resume::Request {
        project_id: "project-1".into(),
        request_id: id.into(),
        target: recovery::get(&f.runtime, "project-1", "head")?
            .unwrap()
            .target,
        verification_request_id: verify.request_id,
        advance: None,
        rework: Some(resume::rework::Approval {
            review_request_id: feedback.review.review_request_id.clone(),
            attempt_id: feedback.review.target.attempt_id.clone(),
            fine_task_id: "fine-b".into(),
            feedback: "Apply confirmed correspondence".into(),
            image: None,
            preview_frame: if attach {
                feedback.preview_frame.clone()
            } else {
                None
            },
            relocation: if attach {
                feedback.relocation.clone()
            } else {
                None
            },
        }),
    })
}

pub(super) fn finish_candidate(
    f: &Fixture,
    lease: object_attempt::Lease,
    id: &str,
) -> Result<Request> {
    let target = crate::object_attempt_view::Target::from_record(lease.record());
    object_attempt::finish(
        &f.runtime,
        lease,
        AttemptState::AwaitingGate,
        None,
        &AtomicBool::new(false),
    )?;
    checks::run(
        &f.runtime,
        &checks::Request {
            project_id: "project-1".into(),
            request_id: format!("check-{id}"),
            target: target.clone(),
        },
    )?;
    request(
        f,
        &candidate::Request {
            project_id: "project-1".into(),
            request_id: format!("review-{id}"),
            check_request_id: format!("check-{id}"),
            target,
        },
    )
}

pub(super) fn reopen(f: Fixture) -> Result<Fixture> {
    let Fixture { runtime, temp } = f;
    drop(runtime);
    Ok(Fixture {
        runtime: crate::project_storage::ProjectStore::open(temp.path(), "project-1")?
            .into_runtime(),
        temp,
    })
}

pub(super) fn assert_delivery(
    f: &Fixture,
    lease: &mut object_attempt::Lease,
    feedback: &deferred::Request,
    source: &Value,
    target: &Value,
) -> Result<Value> {
    object_attempt::bind(&f.runtime, lease, "thread", Some("turn"))?;
    let reply = callback::call(
        &f.runtime,
        lease,
        &json!({"tool":callback::TOOL,
        "threadId":"thread","turnId":"turn","arguments":{"operation":"feedbackImage"}}),
    )?;
    let images = reply["contentItems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["type"] == "inputImage")
        .map(|v| &v["imageUrl"])
        .collect::<Vec<_>>();
    assert_eq!(
        images,
        vec![&target["frame"]["dataUrl"], &source["frame"]["dataUrl"]]
    );
    assert_ne!(images[0], images[1]);
    let binding = callback::context(&f.runtime, lease)?["feedbackImage"].clone();
    assert_eq!(
        binding["relocation"]["confirmation"],
        serde_json::to_value(&feedback.relocation)?
    );
    assert_eq!(binding["relocation"]["sourceFrame"]["id"], source["id"]);
    assert_eq!(binding["previewFrame"]["id"], target["id"]);
    assert!(!serde_json::to_string(&binding)?.contains("data:image"));
    assert!(lease.record().fine.prompt.contains("correspondence"));
    Ok(binding)
}

pub(super) fn approve(f: &Fixture, publish: &mut Request) -> Result<()> {
    super::super::frames::approve(f, publish)
}
