// kaku-tui command smoke harness — exercises commands::parse + execute
// without the TUI. Verifies the parser recognizes each command and the
// executor mutates AppState correctly.
//
//   cargo run --example cmd_smoke
//
// Stops at the first failing assertion; prints which one.

use kaku_tui_lib::app::{AppState, Role};
use kaku_tui_lib::commands;
use kaku_tui_lib::client::OpencodeClient;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // We bind a client with a bogus base URL. Commands other than /new
    // don't touch the network; /new is not exercised here.
    let client = OpencodeClient::new(
        "http://127.0.0.1:1".parse()?,
        "opencode",
        None,
    )?;

    let mut app = AppState::new();

    check(matches!(commands::parse("/help"), commands::Command::Help), "parse /help");
    check(matches!(commands::parse(":quit"), commands::Command::Quit), "parse :quit");
    check(
        matches!(commands::parse(""), commands::Command::Unknown(ref s) if s.is_empty()),
        "parse empty",
    );
    check(
        matches!(commands::parse(":frobnicate"), commands::Command::Unknown(ref s) if s == "frobnicate"),
        "parse garbage",
    );

    // execute: Help must push a System message.
    let n0 = app.messages.len();
    let outcome = commands::execute(commands::Command::Help, &mut app, &client).await;
    check(outcome == commands::Outcome::Continue, "/help outcome");
    check(app.messages.len() == n0 + 1, "/help pushed a message");
    check(
        matches!(app.messages.last().unwrap().role, Role::System),
        "/help pushed System role",
    );

    // execute: Clear removes the pushed help.
    commands::execute(commands::Command::Clear, &mut app, &client).await;
    check(app.messages.is_empty(), "/clear wipes messages");

    // execute: Quit returns Outcome::Quit.
    let outcome = commands::execute(commands::Command::Quit, &mut app, &client).await;
    check(outcome == commands::Outcome::Quit, "/quit returns Quit");

    // execute: Model with no arg shows current + default.
    commands::execute(commands::Command::Model(None), &mut app, &client).await;
    let last = app.messages.last().unwrap();
    check(
        matches!(last.role, Role::System) && last.text.starts_with("model:"),
        "/model show output shape",
    );

    // execute: Model with valid spec sets override + pushes confirmation.
    commands::execute(
        commands::Command::Model(Some("anthropic/claude-opus-4-5".into())),
        &mut app,
        &client,
    )
    .await;
    check(
        app.current_model_override.as_deref() == Some("anthropic/claude-opus-4-5"),
        "/model sets override",
    );
    let last = app.messages.last().unwrap();
    check(
        matches!(last.role, Role::System) && last.text.contains("→ anthropic/claude-opus-4-5"),
        "/model confirmation message",
    );

    // execute: Model with bad spec (no slash) keeps override unchanged.
    let prev_override = app.current_model_override.clone();
    commands::execute(
        commands::Command::Model(Some("nope-no-slash".into())),
        &mut app,
        &client,
    )
    .await;
    check(
        app.current_model_override == prev_override,
        "/model bad spec doesn't change override",
    );

    // execute: Unknown pushes an error message.
    let n_before = app.messages.len();
    let outcome = commands::execute(
        commands::Command::Unknown("nope".to_string()),
        &mut app,
        &client,
    )
    .await;
    check(outcome == commands::Outcome::Continue, "unknown outcome");
    check(app.messages.len() == n_before + 1, "unknown pushed message");

    println!("OK: all command assertions passed.");
    Ok(())
}

fn check(cond: bool, what: &'static str) {
    if !cond {
        eprintln!("FAIL: {what}");
        std::process::exit(1);
    }
}
