use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use tuillem_config::{Config, DefaultsConfig, ProviderConfig, ProviderType, UiConfig};

/// Run the first-run setup wizard in the normal terminal (before ratatui starts).
/// Returns a fully-built Config on success.
pub fn run_setup_wizard() -> Result<Config> {
    println!();
    println!("Welcome to tuillem! Let's set up your configuration.");
    println!();

    // ── Step 1: Provider ────────────────────────────────────────────────
    println!("Step 1/5: Provider Setup");
    println!("Which LLM provider would you like to use?");
    println!("  1. Anthropic (Claude)");
    println!("  2. OpenAI");
    println!("  3. OpenRouter");
    println!("  4. Ollama (local)");
    println!("  5. LM Studio / OpenAI-compatible (local)");
    println!();

    let choice = prompt("Choose [1-5]: ")?;
    let choice = choice.trim();

    let (provider_type, provider_name, default_base_url, needs_api_key) = match choice {
        "1" => (ProviderType::Anthropic, "anthropic", None, true),
        "2" => (ProviderType::Openai, "openai", None, true),
        "3" => (
            ProviderType::Openrouter,
            "openrouter",
            Some("https://openrouter.ai/api/v1"),
            true,
        ),
        "4" => (
            ProviderType::Ollama,
            "ollama",
            Some("http://localhost:11434"),
            false,
        ),
        "5" => (
            ProviderType::Openai,
            "lmstudio",
            Some("http://localhost:1234/v1"),
            false,
        ),
        _ => {
            println!("Invalid choice, defaulting to Anthropic.");
            (ProviderType::Anthropic, "anthropic", None, true)
        }
    };

    println!();

    // ── Step 2: Connection Details ──────────────────────────────────────
    println!("Step 2/5: Connection Details");

    let api_key = if needs_api_key {
        let key = prompt("API Key: ")?;
        let key = key.trim().to_string();
        if key.is_empty() {
            anyhow::bail!("API key is required for this provider.");
        }
        Some(key)
    } else if choice == "5" {
        // LM Studio / OpenAI-compatible gets a dummy key
        Some("lm-studio".to_string())
    } else {
        None
    };

    let base_url = if let Some(default_url) = default_base_url {
        let input = prompt(&format!("Base URL (press Enter for {default_url}): "))?;
        let input = input.trim().to_string();
        if input.is_empty() {
            Some(default_url.to_string())
        } else {
            Some(input)
        }
    } else {
        None
    };

    println!();

    // ── Step 3: Model Selection ─────────────────────────────────────────
    println!("Step 3/5: Model Selection");

    let default_model_input = prompt("Default model name: ")?;
    let default_model = default_model_input.trim().to_string();

    let extra_models_input = prompt("Add more models? (comma-separated, or Enter to skip): ")?;
    let extra_models: Vec<String> = extra_models_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut all_models = Vec::new();
    if !default_model.is_empty() {
        all_models.push(default_model.clone());
    }
    for m in &extra_models {
        if !all_models.contains(m) {
            all_models.push(m.clone());
        }
    }

    println!();

    // ── Step 4: Preferences ─────────────────────────────────────────────
    println!("Step 4/5: Preferences");

    let theme_input =
        prompt("Theme [dark/light/dracula/nord/gruvbox/tokyo_night/solarized] (default: dark): ")?;
    let theme = {
        let t = theme_input.trim().to_string();
        if t.is_empty() { "dark".to_string() } else { t }
    };

    let layout_input = prompt("Layout [loose/tight] (default: loose): ")?;
    let layout = {
        let l = layout_input.trim().to_string();
        if l.is_empty() { "loose".to_string() } else { l }
    };

    let editor_input = prompt("Editor command (default: vim): ")?;
    let editor = {
        let e = editor_input.trim().to_string();
        if e.is_empty() { "vim".to_string() } else { e }
    };

    println!();

    // ── Step 5: Save ────────────────────────────────────────────────────
    let config_path = Config::default_path();
    println!("Step 5/5: Save");
    println!("Config will be saved to: {}", config_path.display());

    let confirm = prompt("Save and start tuillem? [Y/n]: ")?;
    let confirm = confirm.trim().to_lowercase();
    if confirm == "n" || confirm == "no" {
        anyhow::bail!("Setup cancelled by user.");
    }

    // Build the Config
    let provider_config = ProviderConfig {
        name: provider_name.to_string(),
        provider_type,
        api_key,
        base_url,
        default_model: if default_model.is_empty() {
            None
        } else {
            Some(default_model.clone())
        },
        models: all_models,
    };

    let config = Config {
        editor,
        theme,
        providers: vec![provider_config],
        defaults: DefaultsConfig {
            provider: Some(provider_name.to_string()),
            model: if default_model.is_empty() {
                None
            } else {
                Some(default_model)
            },
            system_prompt: None,
        },
        ui: UiConfig {
            layout,
            ..UiConfig::default()
        },
        ..Config::default()
    };

    // Create directory and write config
    write_config(&config_path, &config)?;

    println!();
    println!("Config saved! Starting tuillem...");
    println!();

    Ok(config)
}

fn prompt(message: &str) -> Result<String> {
    print!("{message}");
    io::stdout().flush().context("failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read input")?;
    Ok(input
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string())
}

fn write_config(path: &PathBuf, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory: {}", parent.display()))?;
    }
    let yaml = serde_yaml::to_string(config).context("failed to serialize config")?;
    std::fs::write(path, yaml)
        .with_context(|| format!("failed to write config file: {}", path.display()))?;
    Ok(())
}
