// Quick direct upstream test - no proxy involved
use std::time::Duration;

#[tokio::main]
async fn main() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    // Non-streaming
    println!("\n=== Non-streaming /v1/messages ===");
    let resp = client
        .post("http://103.38.81.122:8080/v1/messages")
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header(
            "authorization",
            "Bearer sk-6baf2df413e4174230cafd79d8b34bad8fd768955aeb17defd50e75fd214bd7b",
        )
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "Reply with exactly: ok"}]
        }))
        .send()
        .await
        .expect("request failed");
    println!("Status: {}", resp.status());
    let ct = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    println!("Content-Type: {ct}");
    let body = resp.text().await.expect("read body");
    println!(
        "Body ({} bytes): {}",
        body.len(),
        &body[..body.len().min(600)]
    );

    // Streaming
    println!("\n=== Streaming /v1/messages ===");
    let resp = client
        .post("http://103.38.81.122:8080/v1/messages")
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header(
            "authorization",
            "Bearer sk-6baf2df413e4174230cafd79d8b34bad8fd768955aeb17defd50e75fd214bd7b",
        )
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 256,
            "stream": true,
            "messages": [{"role": "user", "content": "Reply with exactly: ok"}]
        }))
        .send()
        .await
        .expect("request failed");
    println!("Status: {}", resp.status());
    let ct = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");
    println!("Content-Type: {ct}");
    let body = resp.text().await.expect("read body");
    let lines: Vec<_> = body.lines().take(10).collect();
    for l in &lines {
        println!("  {l}");
    }
    if body.lines().count() > 10 {
        println!("  ... ({} more lines)", body.lines().count() - 10);
    }
}
