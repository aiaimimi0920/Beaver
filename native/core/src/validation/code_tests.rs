use super::code::{directories, parse_report};
use serde_json::json;

fn report(case: &str) -> String {
    format!("<testsuites tests=\"1\"><testsuite>{case}</testsuite></testsuites>")
}

#[test]
fn empty_failures_and_skips_cannot_be_false_green() {
    for (element, expected) in [
        ("failure", "fail"),
        ("error", "fail"),
        ("skipped", "pending"),
    ] {
        let xml = report(&format!("<testcase name=\"test_input\" classname=\"tests/test_input.gd\" status=\"pass\" assertions=\"1\"><{element} message=\"not executed\"/></testcase>"));
        let cases = parse_report(&xml).unwrap();
        assert_eq!(cases[0].status, expected);
        assert_eq!(cases[0].message, "not executed");
    }
    let cases = parse_report(&report(
        "<testcase name=\"test_input\" status=\"pass\" assertions=\"1\"/>",
    ))
    .unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].assertions, 1);
}

#[test]
fn report_must_be_complete_and_counts_must_match() {
    for xml in [
        "<testsuites tests=\"0\"/>",
        "<testsuites tests=\"1\"><testsuite><testcase/>",
        "<testsuites tests=\"2\"><testsuite><testcase/></testsuite></testsuites>",
        "<testcase/>",
        "<testsuites tests=\"1\"><testcase/></testsuites>",
    ] {
        assert!(parse_report(xml).is_err(), "{xml}");
    }
    let xml = report(
        "<testcase status=\"fail\"><failure><![CDATA[expected 2 < 3]]></failure></testcase>",
    );
    assert_eq!(parse_report(&xml).unwrap()[0].message, "expected 2 < 3");
    assert!(parse_report(&(xml.clone() + &xml)).is_err());
}

#[test]
fn explicit_missing_suites_never_shrink_required_scope() {
    let config = json!({"code":{"directories":["tests", "integration"]}});
    assert!(directories(&config, |d| d == "tests").is_err());
    assert_eq!(
        directories(&config, |_| true).unwrap(),
        ["tests", "integration"]
    );
    assert_eq!(
        directories(&json!({}), |d| d == "tests").unwrap(),
        ["tests"]
    );
    assert!(directories(&json!({"code":{"directories":[]}}), |_| true).is_err());
    assert!(directories(&json!({"code":{"directories":"tests"}}), |_| true).is_err());
}
