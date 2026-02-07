use std::path::Path;
use std::process::Command;

/// Regression test for https://github.com/openai/codex/issues/8803.
#[test]
fn malformed_rules_should_not_panic() -> anyhow::Result<()> {
    if cfg!(windows) {
        return Ok(());
    }

    let tmp = tempfile::tempdir()?;
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join("rules"),
        "rules should be a directory not a file",
    )?;

    // TODO(mbolin): Figure out why using a temp dir as the cwd causes this test
    // to hang.
    let cwd = std::env::current_dir()?;
    let config_contents = format!(
        r#"
# Pick a local provider so the CLI doesn't prompt for OpenAI auth in this test.
model_provider = "ollama"

[projects]
"{cwd}" = {{ trust_level = "trusted" }}
"#,
        cwd = cwd.display()
    );
    std::fs::write(codex_home.join("config.toml"), config_contents)?;

    let CodexCliOutput { exit_code, output } = run_codex_cli(codex_home, &cwd)?;
    assert_ne!(0, exit_code, "Codex CLI should exit nonzero.");
    assert!(
        output.contains("ERROR: Failed to initialize codex:"),
        "expected startup error in output, got: {output}"
    );
    assert!(
        output.contains("failed to read rules files"),
        "expected rules read error in output, got: {output}"
    );
    Ok(())
}

struct CodexCliOutput {
    exit_code: i32,
    output: String,
}

fn run_codex_cli(codex_home: impl AsRef<Path>, cwd: &Path) -> anyhow::Result<CodexCliOutput> {
    let codex_cli = codex_utils_cargo_bin::cargo_bin("codex")?;
    let codex_home = codex_home.as_ref();
    let args = ["-c", "analytics.enabled=false"];

    // Use a non-interactive spawn to keep the test stable and avoid PTY flakes.
    // This code path still exercises codex initialization and error reporting.
    let mut command = Command::new(codex_cli);
    command.current_dir(cwd);
    command.env_clear();
    command.env("CODEX_HOME", codex_home);
    command.env("HOME", codex_home);
    if let Ok(path) = std::env::var("PATH") {
        command.env("PATH", path);
    }
    command.env("TERM", "xterm-256color");
    command.args(args);

    let output = command.output()?;
    let exit_code = output.status.code().unwrap_or(-1);
    let mut combined = output.stdout;
    combined.extend_from_slice(&output.stderr);
    let combined = String::from_utf8_lossy(&combined);

    Ok(CodexCliOutput {
        exit_code,
        output: combined.to_string(),
    })
}
