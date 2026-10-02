//! Bounded read-ahead keeps EOF observable while an admitted call is running.
use anyhow::Result;
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt},
    sync::{mpsc, watch},
};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;

pub(crate) enum Frame {
    TooLarge,
    Data(Vec<u8>),
}

async fn read_frame(reader: &mut (impl AsyncBufRead + Unpin)) -> Result<Option<Frame>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(if oversized {
                Some(Frame::TooLarge)
            } else if bytes.is_empty() {
                None
            } else {
                Some(Frame::Data(bytes))
            });
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if !oversized && bytes.len().saturating_add(count) <= MAX_REQUEST_BYTES {
            bytes.extend_from_slice(&available[..count]);
        } else {
            oversized = true;
            bytes.clear();
        }
        reader.consume(count);
        if newline.is_some() {
            return Ok(Some(if oversized {
                Frame::TooLarge
            } else {
                Frame::Data(bytes)
            }));
        }
    }
}

pub(crate) async fn pump(
    mut reader: impl AsyncBufRead + Unpin,
    sender: mpsc::Sender<Result<Frame>>,
    ended: watch::Sender<bool>,
) -> Result<()> {
    let result = read_requests(&mut reader, sender).await;
    let _ = ended.send(true);
    result
}

async fn read_requests(
    reader: &mut (impl AsyncBufRead + Unpin),
    sender: mpsc::Sender<Result<Frame>>,
) -> Result<()> {
    loop {
        // A bounded peek sees EOF without waiting for queue capacity. Recheck
        // capacity after the await: a sequential client may have received its
        // response while this reader was waiting for the next input byte.
        if reader.fill_buf().await?.is_empty() {
            return Ok(());
        }
        anyhow::ensure!(sender.capacity() > 0,
            "Headless request pipeline exceeded: wait for each response before sending another request");
        let Some(frame) = read_frame(reader).await? else {
            return Ok(());
        };
        // Never wait for queue capacity: the consumer may be blocked on stdout.
        // Refuse excess input before draining an unbounded unterminated line.
        match sender.try_send(Ok(frame)) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => return Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                anyhow::bail!("Headless request pipeline exceeded: wait for each response before sending another request");
            }
        }
    }
}
