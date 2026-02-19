use anyhow::Result;
use codex_core::CodexAuth;
use codex_core::ThreadManager;
use codex_core::built_in_model_providers;
use codex_core::models_manager::manager::RefreshStrategy;
use core_test_support::load_default_config_for_test;
use pretty_assertions::assert_eq;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_models_returns_api_key_models() -> Result<()> {
    let codex_home = tempdir()?;
    let config = load_default_config_for_test(&codex_home).await;
    let manager = ThreadManager::with_models_provider(
        CodexAuth::from_api_key("sk-test"),
        built_in_model_providers()["openai"].clone(),
    );
    let models = manager.list_models(&config, RefreshStrategy::Offline).await;

    let actual = sorted_models(&models);
    let expected = sorted_expected_models_for_api_key();
    assert_eq!(expected, actual);

    assert_eq!(models.iter().filter(|m| m.is_default).count(), 1);
    assert_eq!(
        models
            .iter()
            .find(|m| m.is_default)
            .map(|m| m.model.as_str()),
        Some("gpt-5.3-codex")
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_models_returns_chatgpt_models() -> Result<()> {
    let codex_home = tempdir()?;
    let config = load_default_config_for_test(&codex_home).await;
    let manager = ThreadManager::with_models_provider(
        CodexAuth::create_dummy_chatgpt_auth_for_testing(),
        built_in_model_providers()["openai"].clone(),
    );
    let models = manager.list_models(&config, RefreshStrategy::Offline).await;

    let actual = sorted_models(&models);
    let expected = sorted_expected_models_for_chatgpt();
    assert_eq!(expected, actual);

    Ok(())
}

fn sorted_models(models: &[codex_protocol::openai_models::ModelPreset]) -> Vec<String> {
    let mut slugs: Vec<String> = models.iter().map(|m| m.model.clone()).collect();
    slugs.sort();
    slugs
}

fn sorted_expected_models_for_api_key() -> Vec<String> {
    let mut slugs: Vec<String> = vec![
        "gpt-5.3-codex",
        "gpt-5.3-codex-spark",
        "gpt-5.2-codex",
        "gpt-5.2",
        "gpt-5.1-codex-max",
        "gpt-5.1-codex",
        "gpt-5.1-codex-mini",
        "gpt-5.1",
        "gpt-5-codex",
        "gpt-5",
        "gpt-5-codex-mini",
        "bengalfox",
        "boomslang",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    slugs.sort();
    slugs
}

fn sorted_expected_models_for_chatgpt() -> Vec<String> {
    // Keep this separate so we can tighten it later if the auth-mode filtering diverges.
    sorted_expected_models_for_api_key()
}
