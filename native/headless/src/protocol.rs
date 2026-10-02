use crate::{
    contract,
    frames::{self, Frame},
    Host,
};
use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, watch};

pub use crate::frames::MAX_REQUEST_BYTES;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: Value,
    method: String,
    #[serde(default = "empty_input")]
    input: Value,
}
fn empty_input() -> Value {
    json!({})
}

fn failure(id: Value, message: impl ToString) -> Value {
    json!({"id":id,"error":{"message":message.to_string()}})
}

async fn write_response(
    host: &Host,
    writer: &mut (impl AsyncWrite + Unpin),
    response: &Value,
    stop: &mut watch::Receiver<bool>,
    eof: &mut watch::Receiver<bool>,
) -> Result<bool> {
    if *stop.borrow() {
        return Ok(false);
    }
    if *eof.borrow() {
        let _ = host.request_stop();
    }
    let mut bytes = serde_json::to_vec(response)?;
    bytes.push(b'\n');
    // The business call has already finished. Response delivery may be abandoned
    // on termination so a client that stopped reading cannot prevent shutdown.
    tokio::select! {
        biased;
        _ = stop.changed() => Ok(false),
        result = async {
            writer.write_all(&bytes).await?;
            writer.flush().await
        } => { result?; Ok(true) },
        _ = ended(eof) => { let _ = host.request_stop(); Ok(false) },
    }
}

pub async fn serve<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    host: Arc<Host>,
    reader: R,
    mut writer: W,
    stop: watch::Receiver<bool>,
) -> Result<()> {
    let (sender, receiver) = mpsc::channel(1);
    let (ended, eof) = watch::channel(false);
    // One queued frame plus one overflow-detection frame is the bounded limit.
    // Dropping the reader never drops an admitted business operation.
    let input = frames::pump(BufReader::new(reader), sender, ended);
    let work = requests(&host, receiver, &mut writer, stop, eof);
    tokio::pin!(input, work);
    let result = tokio::select! {
        biased;
        transport = &mut input => {
            if transport.is_err() {
                // Reject queued work on malformed transport, before draining any
                // already-admitted mutation. EOF alone preserves the final frame.
                let _ = host.request_stop();
            }
            let business = work.await;
            transport.and(business)
        },
        result = &mut work => result,
    };
    let shutdown = host.shutdown().await;
    result.and(shutdown)
}

async fn ended(receiver: &mut watch::Receiver<bool>) {
    if !*receiver.borrow() {
        let _ = receiver.changed().await;
    }
}

async fn admitted_call(
    host: &Arc<Host>,
    method: &str,
    input: Value,
    stop: &mut watch::Receiver<bool>,
    eof: &mut watch::Receiver<bool>,
) -> Result<Value> {
    let call = host.call(method, input);
    tokio::pin!(call);
    tokio::select! {
        // Poll admission first. EOF immediately after a complete request does not
        // discard that request. Subsequent buffered frames are never admitted.
        biased;
        result = &mut call => result,
        _ = async { tokio::select! { _ = ended(stop) => {}, _ = ended(eof) => {} } } => {
            if host.request_stop().is_err() {
                eprintln!("Headless cancellation request failed; orderly drain will retry");
            }
            // Retain the owned mutation, receipt and audit through cancellation.
            call.await
        }
    }
}

async fn requests<W: AsyncWrite + Unpin>(
    host: &Arc<Host>,
    mut reader: mpsc::Receiver<Result<Frame>>,
    writer: &mut W,
    mut stop: watch::Receiver<bool>,
    mut eof: watch::Receiver<bool>,
) -> Result<()> {
    let mut processed = false;
    loop {
        if *stop.borrow() || (processed && *eof.borrow()) {
            return Ok(());
        }
        let frame = tokio::select! {
            biased;
            _ = stop.changed() => return Ok(()),
            frame = reader.recv() => match frame {
                Some(frame) => frame?,
                None => return Ok(()),
            },
        };
        processed = true;
        let bytes = match frame {
            Frame::TooLarge => {
                if !write_response(
                    host,
                    writer,
                    &failure(Value::Null, "Request exceeds 1 MiB"),
                    &mut stop,
                    &mut eof,
                )
                .await?
                {
                    return Ok(());
                }
                continue;
            }
            Frame::Data(bytes) => bytes,
        };
        let request = match serde_json::from_slice::<Request>(&bytes) {
            Ok(request)
                if request.id.is_number()
                    || request.id.as_str().is_some_and(|id| id.len() <= 200) =>
            {
                request
            }
            _ => {
                if !write_response(
                    host,
                    writer,
                    &failure(Value::Null, "Invalid JSONL request"),
                    &mut stop,
                    &mut eof,
                )
                .await?
                {
                    return Ok(());
                }
                continue;
            }
        };
        if request.method == "shutdown" {
            match contract::validate(&request.method, &request.input) {
                Ok(()) => {
                    let response = match host.shutdown().await {
                        Ok(()) => json!({"id":request.id,"result":{"stopped":true}}),
                        Err(error) => failure(request.id, error),
                    };
                    write_response(host, writer, &response, &mut stop, &mut eof).await?;
                    return Ok(());
                }
                Err(error) => {
                    if !write_response(
                        host,
                        writer,
                        &failure(request.id, error),
                        &mut stop,
                        &mut eof,
                    )
                    .await?
                    {
                        return Ok(());
                    }
                    continue;
                }
            }
        }
        let response =
            match admitted_call(host, &request.method, request.input, &mut stop, &mut eof).await {
                Ok(value) => json!({"id":request.id,"result":value}),
                Err(error) => failure(request.id, error),
            };
        if !write_response(host, writer, &response, &mut stop, &mut eof).await? {
            return Ok(());
        }
    }
}
