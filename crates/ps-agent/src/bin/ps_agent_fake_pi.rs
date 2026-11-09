//! Offline test/support executable that emulates `pi --mode json`.
//!
//! Integration tests point the Pi backend at this binary (directly or through a
//! generated `.cmd` wrapper) so the full lifecycle — stdin prompt delivery,
//! JSONL framing, event mapping, timeouts, cancellation and stderr redaction —
//! is exercised without a model, credentials or network.
//!
//! The behaviour is selected by a file `ps_fake_pi_mode` in the working
//! directory (the Pi workspace), so parallel tests never share state:
//!   happy (default), stderr_secret, malformed, oversize, idle, tree, crash,
//!   no_session.
//!
//! This binary is never used by the app.

use std::io::{Read, Write};
use std::process::Command;
use std::thread;
use std::time::Duration;

fn emit(out: &mut impl Write, value: serde_json::Value) {
    let _ = writeln!(out, "{}", serde_json::to_string(&value).unwrap_or_default());
    let _ = out.flush();
}

fn sleep_forever() -> ! {
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn session(out: &mut impl Write) {
    emit(
        out,
        serde_json::json!({
            "type": "session",
            "version": 3,
            "id": "fake-session",
            "timestamp": "2026-11-09T00:00:00.000Z",
            "cwd": "C:/fake-workspace"
        }),
    );
}

fn happy(out: &mut impl Write, prompt: &str, secret: Option<&str>) {
    let snippet: String = prompt
        .chars()
        .filter(|character| !character.is_control())
        .take(80)
        .collect();
    emit(out, serde_json::json!({"type": "agent_start"}));
    emit(
        out,
        serde_json::json!({"type": "message_start", "message": {"role": "assistant", "content": [], "stopReason": "pending"}}),
    );
    emit(
        out,
        serde_json::json!({
            "type": "message_update",
            "usage": {"input": 10, "output": 1, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 11, "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}},
            "assistantMessageEvent": {"type": "text_delta", "contentIndex": 0, "delta": "Fake "}
        }),
    );
    emit(
        out,
        serde_json::json!({
            "type": "message_update",
            "usage": {"input": 10, "output": 2, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 12, "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}},
            "assistantMessageEvent": {"type": "text_delta", "contentIndex": 0, "delta": "answer."}
        }),
    );
    let mut tool_text = String::from("tool read completed");
    if let Some(secret) = secret {
        tool_text.push_str(&format!(" leaked {secret}"));
    }
    emit(
        out,
        serde_json::json!({"type": "tool_execution_start", "toolCallId": "call-1", "toolName": "read", "args": {"path": "notes.txt"}}),
    );
    emit(
        out,
        serde_json::json!({
            "type": "tool_execution_end",
            "toolCallId": "call-1",
            "toolName": "read",
            "result": {"content": [{"type": "text", "text": tool_text}]},
            "isError": false,
            "durationMs": 1
        }),
    );
    emit(
        out,
        serde_json::json!({
            "type": "message_end",
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": format!("Fake answer for: {snippet}")}],
                "api": "fake",
                "provider": "fake",
                "model": "fake-model",
                "usage": {"input": 10, "output": 5, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 15, "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}},
                "stopReason": "stop",
                "timestamp": 1
            }
        }),
    );
    emit(
        out,
        serde_json::json!({"type": "agent_end", "messages": [], "willRetry": false}),
    );
    emit(
        out,
        serde_json::json!({"type": "agent_settled", "aborted": false}),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("1.5.0-fake");
        return;
    }
    if args.iter().any(|arg| arg == "--child-sleep") {
        sleep_forever();
    }

    let mut prompt = String::new();
    let _ = std::io::stdin().read_to_string(&mut prompt);

    let cwd = std::env::current_dir().unwrap_or_default();
    let _ = std::fs::write(
        cwd.join("ps_fake_pi_args.json"),
        serde_json::to_string(&args).unwrap_or_default(),
    );
    let mode = std::fs::read_to_string(cwd.join("ps_fake_pi_mode"))
        .unwrap_or_else(|_| "happy".to_string())
        .trim()
        .to_string();

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    match mode.as_str() {
        "no_session" => {
            eprintln!("fatal: could not authenticate provider");
            std::process::exit(2);
        }
        "crash" => {
            session(&mut out);
            eprintln!("fatal: could not authenticate provider (token expired)");
            std::process::exit(2);
        }
        "idle" => {
            session(&mut out);
            sleep_forever();
        }
        "tree" => {
            session(&mut out);
            if let Ok(exe) = std::env::current_exe() {
                if let Ok(child) = Command::new(exe).arg("--child-sleep").spawn() {
                    let _ = std::fs::write(cwd.join("ps_fake_pi_child.pid"), child.id().to_string());
                }
            }
            sleep_forever();
        }
        "oversize" => {
            session(&mut out);
            let payload = vec![b'x'; 4 * 1024 * 1024 + 4096];
            let _ = out.write_all(&payload);
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            thread::sleep(Duration::from_secs(1));
        }
        "malformed" => {
            session(&mut out);
            let _ = writeln!(out, "{{not json");
            let _ = out.flush();
            happy(&mut out, &prompt, None);
        }
        "stderr_secret" => {
            session(&mut out);
            eprintln!("debug: provider token=sk-live-supersecret-abcdef");
            happy(&mut out, &prompt, Some("sk-live-supersecret-abcdef"));
        }
        _ => {
            session(&mut out);
            happy(&mut out, &prompt, None);
        }
    }
}
