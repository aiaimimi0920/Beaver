use super::model::CodeCase;
use anyhow::{ensure, Context, Result};
use quick_xml::{
    encoding::Decoder,
    events::{BytesStart, Event},
    Reader, XmlVersion,
};

fn attributes(start: &BytesStart<'_>, decoder: Decoder) -> Result<Vec<(Vec<u8>, String)>> {
    start
        .attributes()
        .map(|a| {
            let a = a?;
            Ok((
                a.key.as_ref().to_vec(),
                a.decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)?
                    .into_owned(),
            ))
        })
        .collect()
}

/// Accept GUT's JUnit report only when the document and every case are complete.
/// Empty XML elements carry the same failure/skip semantics as paired elements.
pub fn parse_report(xml: &str) -> Result<Vec<CodeCase>> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Vec<u8>> = vec![];
    let mut cases = vec![];
    let mut case: Option<CodeCase> = None;
    let mut declared = None;
    let mut complete = false;
    loop {
        let event = reader.read_event()?;
        let empty = matches!(&event, Event::Empty(_));
        let closing = match event {
            Event::Start(start) | Event::Empty(start) => {
                let name = start.name().as_ref().to_vec();
                ensure!(!complete, "Content follows completed GUT report");
                if stack.is_empty() {
                    ensure!(
                        name == b"testsuites" && declared.is_none(),
                        "Invalid GUT report root"
                    );
                }
                let attrs = attributes(&start, reader.decoder())?;
                match name.as_slice() {
                    b"testsuites" => {
                        ensure!(stack.is_empty(), "Nested GUT report");
                        declared = attrs
                            .iter()
                            .find(|(key, _)| key == b"tests")
                            .map(|(_, value)| value.parse::<usize>())
                            .transpose()?;
                    }
                    b"testcase" => {
                        ensure!(
                            case.is_none() && stack.last().is_some_and(|v| v == b"testsuite"),
                            "Invalid GUT testcase nesting"
                        );
                        let mut next = CodeCase::default();
                        for (key, value) in attrs {
                            match key.as_slice() {
                                b"name" => next.name = value,
                                b"classname" => next.file = value,
                                b"status" => next.status = value,
                                b"assertions" => next.assertions = value.parse()?,
                                _ => {}
                            }
                        }
                        case = Some(next);
                    }
                    b"failure" | b"error" | b"skipped" => {
                        let current = case.as_mut().context("Failure/skip outside a testcase")?;
                        if name != b"skipped" || current.status != "fail" {
                            current.status = if name == b"skipped" {
                                "pending"
                            } else {
                                "fail"
                            }
                            .into();
                        }
                        if let Some((_, message)) = attrs.iter().find(|(key, _)| key == b"message")
                        {
                            current.message.push_str(message);
                        }
                    }
                    _ => {}
                }
                stack.push(name.clone());
                empty.then_some(name)
            }
            Event::End(end) => Some(end.name().as_ref().to_vec()),
            Event::Text(text) => {
                if let Some(c) = &mut case {
                    c.message.push_str(&text.decode()?);
                }
                None
            }
            Event::CData(text) => {
                if let Some(c) = &mut case {
                    c.message.push_str(&text.decode()?);
                }
                None
            }
            Event::Eof => break,
            _ => None,
        };
        if let Some(name) = closing {
            ensure!(stack.pop().as_ref() == Some(&name), "Unbalanced GUT report");
            match name.as_slice() {
                b"testcase" => cases.push(case.take().context("Unexpected testcase end")?),
                b"testsuites" => complete = true,
                _ => {}
            }
        }
    }
    ensure!(
        complete
            && stack.is_empty()
            && case.is_none()
            && declared == Some(cases.len())
            && !cases.is_empty(),
        "GUT report is empty, truncated or inconsistent"
    );
    Ok(cases)
}
