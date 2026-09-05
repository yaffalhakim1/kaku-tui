// kaku-tui smoke harness — exercises the SSE pipeline + prompt_async against
// a live opencode server. Bypasses the TUI so we can prove the streaming
// contract end-to-end with no terminal gymnastics.
//
//   KAKU_TUI_PASSWORD=... cargo run --example sse_smoke -- <port>

use anyhow::Result;
use futures_util::StreamExt;
use kaku_tui_lib::client::OpencodeClient;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let port: u16 = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: sse_smoke <port>"))?
        .parse()?;
    let base: reqwest::Url = format!("http://127.0.0.1:{port}").parse()?;
    // Same fallback as the binary: prefer KAKU_TUI_PASSWORD, else OPENCODE_SERVER_PASSWORD.
    let pw = std::env::var("KAKU_TUI_PASSWORD")
        .ok()
        .or_else(|| std::env::var("OPENCODE_SERVER_PASSWORD").ok());
    let user = std::env::var("OPENCODE_SERVER_USERNAME").unwrap_or_else(|_| "opencode".to_string());

    let c = OpencodeClient::new(base, &user, pw.as_deref())?;

    // 1. Health.
    let h = c.health().await?;
    println!("[health] {} healthy={}", h.version, h.healthy);

    // 2. Session.
    let s = c.create_session(Some("sse_smoke")).await?;
    println!("[session] {} ({})", s.id, s.title);

    // 3. Open SSE stream FIRST, then send prompt.
    //    Order matters: opencode emits events as soon as we send.
    let url = c.base_url().join("/event")?;
    let resp = c.http_get(url).send().await?;
    assert!(resp.status().is_success(), "SSE status {}", resp.status());

    // Print raw response status to stderr so it's distinct from stdout events.
    eprintln!("[sse] opened, status {}", resp.status());

    let mut stream = resp.bytes_stream();

    // 4. Send the prompt immediately. The SSE stream will start producing
    //    events that we read inline below.
    let prompt = "Reply with exactly one word: PONG. Nothing else.";
    println!("[send] -> {prompt:?}");
    c.send_prompt(&s.id, prompt).await?;

    // 5. Read until session.idle or 30s elapse. Print every event we see.
    let start = std::time::Instant::now();
    let mut n = 0;
    let mut last_text = String::new();
    let mut buf: Vec<u8> = Vec::new();
    while start.elapsed() < std::time::Duration::from_secs(30) {
        // Pull next chunk with a 1-second ceiling so we can bail on quiet streams.
        let next = tokio::time::timeout(std::time::Duration::from_millis(1000), stream.next()).await;
        match next {
            Ok(Some(Ok(chunk))) => {
                buf.extend_from_slice(&chunk);
                while let Some(idx) = find_subseq(&buf, b"\n\n") {
                    let raw = buf.drain(..idx + 2).collect::<Vec<u8>>();
                    let Ok(s) = std::str::from_utf8(&raw) else { continue };
                    let json: String = s
                        .lines()
                        .filter_map(|l| l.strip_prefix("data: "))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if json.is_empty() { continue; }
                    n += 1;
                    let v: serde_json::Value = match serde_json::from_str(&json) {
                        Ok(v) => v,
                        Err(_) => { eprintln!("[bad-json] {json}"); continue; }
                    };
                    // SSE wire shape per opencode: { type, properties } at root, NOT wrapped in payload.
                    let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("?").to_string();
                    // Print first 3 events raw so we can see the actual shape.
                    if n <= 2 {
                        println!("[#{n} RAW] {json}");
                    }
                    if t == "?" {
                        println!("[#{n} keys] {:?}", v.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                    }
                    println!("[#{n}] type={t}");
                    if t == "message.part.updated" {
                        // Dump the first part-updated event raw so we know shape.
                        if n <= 5 {
                            println!("[#{n} PART] {json}");
                        }
                        let text = v.pointer("/properties/part/text").and_then(|x| x.as_str()).unwrap_or("");
                        let delta = v.pointer("/properties/delta").and_then(|x| x.as_str());
                        if let Some(d) = delta {
                            last_text.push_str(d);
                            eprintln!("  delta=({d:?}) running={last_text:?}");
                        } else if !text.is_empty() {
                            eprintln!("  replace text={text:?}");
                        }
                    }
                    if t == "session.idle" {
                        println!("[done] session.idle observed");
                        println!("[result] last text was: {last_text:?}");
                        return Ok(());
                    }
                }
            }
            Ok(Some(Err(e))) => eprintln!("[stream err] {e}"),
            Ok(None) => { eprintln!("[stream closed]"); break; }
            Err(_) => eprintln!("[quiet 1s, {} events so far]", n),
        }
    }
    eprintln!("[timeout, {} events]", n);
    Ok(())
}

fn find_subseq(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
