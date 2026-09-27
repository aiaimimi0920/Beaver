pub(super) use super::attempt_fixture::*;
use super::object_candidate_review::{final_output, final_output_with};
pub(super) use crate::object_run_recovery::candidate::publication::{self, Request, State};
use crate::{object_catalog_test_fixture::Fixture, object_run_recovery::candidate};
use anyhow::Result;
use std::fs;

pub(super) fn request(f: &Fixture, review: &candidate::Request) -> Result<Request> {
    candidate::prepare(&f.runtime, review)?;
    let preview = publication::preview(
        &f.runtime,
        &publication::ReviewTarget {
            project_id: review.project_id.clone(),
            target: review.target.clone(),
            review_request_id: review.request_id.clone(),
        },
    )?;
    Ok(Request {
        project_id: review.project_id.clone(),
        request_id: "publish".into(),
        target: review.target.clone(),
        review_request_id: review.request_id.clone(),
        preview_digest: preview.digest,
        acceptance_note: "Owner reviewed the final output".into(),
        confirm_files: true,
        confirm_replacement: false,
        feedback: vec![],
    })
}

pub(super) fn ready(f: &Fixture) -> Result<Request> {
    request(f, &final_output(f)?)
}

pub(super) fn multiple_files(f: &Fixture) -> Result<Request> {
    request(
        f,
        &final_output_with(f, |root| {
            fs::write(root.join("a.txt"), "first")?;
            fs::write(root.join("z.txt"), "last")?;
            Ok(())
        })?,
    )
}

pub(super) fn review(request: &Request) -> publication::ReviewTarget {
    publication::ReviewTarget {
        project_id: request.project_id.clone(),
        target: request.target.clone(),
        review_request_id: request.review_request_id.clone(),
    }
}

pub(super) fn fail_at(
    f: &Fixture,
    request: &Request,
    point: &str,
) -> Result<publication::Operation> {
    publication::execute_with(&f.runtime, request, |name| {
        anyhow::ensure!(name != point, "simulated host loss");
        Ok(())
    })
}
