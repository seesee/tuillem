use std::collections::HashMap;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Parse(#[from] serde_yaml::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingPreset {
    Vim,
    Emacs,
    #[default]
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    Openai,
    Openrouter,
    Ollama,
}

// ---------------------------------------------------------------------------
// ThemeColors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ThemeColors {
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub sidebar_bg: Option<String>,
    pub sidebar_fg: Option<String>,
    pub sidebar_selected: Option<String>,
    pub user_msg_bg: Option<String>,
    pub assistant_msg_bg: Option<String>,
    pub thinking_fg: Option<String>,
    pub accent: Option<String>,
    pub error: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub border: Option<String>,
    pub code_bg: Option<String>,
    pub code_fg: Option<String>,
    pub heading: Option<String>,
    pub link: Option<String>,
    pub tag: Option<String>,
}

// ---------------------------------------------------------------------------
// ProviderConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
}

// ---------------------------------------------------------------------------
// DefaultsConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DefaultsConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// ToolConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub command: String,
    pub input_schema: Option<serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout: String,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_timeout() -> String {
    "30s".to_string()
}

// ---------------------------------------------------------------------------
// DatabaseConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_path")]
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_database_path(),
        }
    }
}

fn default_database_path() -> String {
    ProjectDirs::from("com", "tuillem", "tuillem")
        .map(|dirs| {
            dirs.data_dir()
                .join("tuillem.db")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "tuillem.db".to_string())
}

// ---------------------------------------------------------------------------
// UiConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,
    #[serde(default)]
    pub show_thinking: bool,
    #[serde(default = "default_true")]
    pub show_token_usage: bool,
    #[serde(default = "default_true")]
    pub mouse: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar_width: 30,
            show_thinking: false,
            show_token_usage: true,
            mouse: true,
        }
    }
}

fn default_sidebar_width() -> u16 {
    30
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Config (top-level)
// ---------------------------------------------------------------------------

fn default_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

fn default_theme() -> String {
    "dark".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default)]
    pub keybindings: KeybindingPreset,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub themes: HashMap<String, ThemeColors>,

    #[serde(default)]
    pub providers: Vec<ProviderConfig>,

    #[serde(default)]
    pub defaults: DefaultsConfig,

    #[serde(default)]
    pub tools: Vec<ToolConfig>,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: default_editor(),
            keybindings: KeybindingPreset::Default,
            theme: "dark".to_string(),
            themes: HashMap::new(),
            providers: Vec::new(),
            defaults: DefaultsConfig::default(),
            tools: Vec::new(),
            database: DatabaseConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Config {
    /// Parse a YAML string into a `Config`, then validate it.
    pub fn from_yaml(yaml: &str) -> Result<Config, ConfigError> {
        let config: Config = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }

    /// Read a file and parse it as YAML config.
    pub fn from_file(path: &Path) -> Result<Config, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_yaml(&contents)
    }

    /// Return the default XDG config path for the config file.
    pub fn default_path() -> PathBuf {
        ProjectDirs::from("com", "tuillem", "tuillem")
            .map(|dirs| dirs.config_dir().join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from("config.yaml"))
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // API-based providers must have an api_key.
        for provider in &self.providers {
            let needs_key = matches!(
                provider.provider_type,
                ProviderType::Anthropic | ProviderType::Openai | ProviderType::Openrouter
            );
            if needs_key && provider.api_key.is_none() {
                return Err(ConfigError::Validation(format!(
                    "Provider '{}' requires an api_key",
                    provider.name
                )));
            }
        }

        // Default provider must exist in the providers list.
        if let Some(ref default_provider) = self.defaults.provider {
            let exists = self.providers.iter().any(|p| &p.name == default_provider);
            if !exists {
                return Err(ConfigError::Validation(format!(
                    "Default provider '{}' not found in providers list",
                    default_provider
                )));
            }
        }

        Ok(())
    }
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let config = Config::from_yaml("{}").expect("should parse empty config");
        assert_eq!(config.theme, "dark");
        assert_eq!(config.keybindings, KeybindingPreset::Default);
        assert!(config.providers.is_empty());
        assert!(config.tools.is_empty());
        assert_eq!(config.ui.sidebar_width, 30);
        assert!(!config.ui.show_thinking);
        assert!(config.ui.show_token_usage);
        assert!(config.ui.mouse);
    }

    #[test]
    fn test_full_config() {
        let yaml = r##"
editor: nvim
keybindings: vim
theme: dark
themes:
  dark:
    bg: "#1e1e2e"
    fg: "#cdd6f4"
providers:
  - name: anthropic
    provider_type: anthropic
    api_key: "sk-ant-test"
    default_model: claude-sonnet-4-20250514
    models:
      - claude-sonnet-4-20250514
      - claude-3-haiku-20240307
  - name: local
    provider_type: ollama
    base_url: "http://localhost:11434"
    models:
      - llama3
defaults:
  provider: anthropic
  model: claude-sonnet-4-20250514
  system_prompt: "You are a helpful assistant."
tools:
  - name: grep_tool
    description: "Search files"
    command: "grep -rn"
    timeout: "10s"
    confirm: true
    env:
      LANG: "en_US.UTF-8"
database:
  path: "/tmp/test.db"
ui:
  sidebar_width: 40
  show_thinking: true
  show_token_usage: false
  mouse: false
"##;
        let config = Config::from_yaml(yaml).expect("should parse full config");
        assert_eq!(config.editor, "nvim");
        assert_eq!(config.keybindings, KeybindingPreset::Vim);
        assert_eq!(config.theme, "dark");
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.providers[0].name, "anthropic");
        assert_eq!(config.providers[0].api_key.as_deref(), Some("sk-ant-test"));
        assert_eq!(config.providers[1].provider_type, ProviderType::Ollama);
        assert_eq!(config.defaults.provider.as_deref(), Some("anthropic"));
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].timeout, "10s");
        assert!(config.tools[0].confirm);
        assert_eq!(config.database.path, "/tmp/test.db");
        assert_eq!(config.ui.sidebar_width, 40);
        assert!(config.ui.show_thinking);
        assert!(!config.ui.show_token_usage);
        assert!(!config.ui.mouse);

        // Theme check
        let dark = config.themes.get("dark").expect("dark theme should exist");
        assert_eq!(dark.bg.as_deref(), Some("#1e1e2e"));
    }

    #[test]
    fn test_validation_missing_api_key() {
        let yaml = "
providers:
  - name: anthropic
    provider_type: anthropic
";
        let result = Config::from_yaml(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("requires an api_key"),
            "Expected api_key error, got: {err}",
        );
    }

    #[test]
    fn test_validation_invalid_default_provider() {
        let yaml = "
providers:
  - name: anthropic
    provider_type: anthropic
    api_key: sk-test
defaults:
  provider: nonexistent
";
        let result = Config::from_yaml(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found in providers list"),
            "Expected provider-not-found error, got: {err}",
        );
    }

    #[test]
    fn test_ollama_no_api_key_required() {
        let yaml = "
providers:
  - name: local
    provider_type: ollama
    base_url: http://localhost:11434
    models:
      - llama3
";
        let config = Config::from_yaml(yaml).expect("ollama should not require api_key");
        assert_eq!(config.providers.len(), 1);
        assert_eq!(config.providers[0].provider_type, ProviderType::Ollama);
        assert!(config.providers[0].api_key.is_none());
    }
}
