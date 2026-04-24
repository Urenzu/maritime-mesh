use anyhow::Result;
use schema::GhostFrame;
use tokio::sync::mpsc;
use tracing::warn;
use zenoh::prelude::r#async::*;

const FRAMES_KEY: &str = "maritime/frames/**";

pub async fn open_subscriber() -> Result<mpsc::Receiver<GhostFrame>> {
    let session = zenoh::open(zenoh::config::peer())
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    // Leak the session to get a 'static reference. This is intentional:
    // the subscriber must live for the process lifetime and Zenoh 0.11's
    // Subscriber<'_> borrows from the Session — Box::leak is the correct
    // pattern for a process-lifetime singleton.
    let session: &'static zenoh::Session = Box::leak(Box::new(session));

    let (tx, rx) = mpsc::channel::<GhostFrame>(8192);

    // Subscriber<'static> — no borrow conflict, safe to forget.
    let subscriber = session
        .declare_subscriber(FRAMES_KEY)
        .callback(move |sample| {
            let bytes = sample.value.payload.contiguous();
            match serde_json::from_slice::<GhostFrame>(&bytes) {
                Ok(frame) => { let _ = tx.blocking_send(frame); }
                Err(e)    => warn!("frame deserialize: {e}"),
            }
        })
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    std::mem::forget(subscriber);

    Ok(rx)
}
