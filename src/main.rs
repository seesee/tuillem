use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Logging to file (not terminal, since we own the screen)
    let log_dir = directories::ProjectDirs::from("com", "tuillem", "tuillem")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("tuillem.log");
    let log_file = std::fs::File::create(&log_path)?;

    // Enable DEBUG level for tuillem crates, INFO for everything else
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("tuillem=debug,tuillem_core=debug,tuillem_db=debug,tuillem_provider=debug,tuillem_plugin=debug,tuillem_tui=debug,tuillem_config=debug,info")
            }),
        )
        .init();

    info!("tuillem starting — log file: {}", log_path.display());

    // 2. Load config
    let config_path = tuillem_config::Config::default_path();
    debug!("Config path: {}", config_path.display());
    let config = if config_path.exists() {
        debug!("Config file found, loading...");
        tuillem_config::Config::from_file(&config_path)?
    } else {
        info!("No config found at {}. Using defaults.", config_path.display());
        eprintln!(
            "No config found at {}. Using defaults.",
            config_path.display()
        );
        tuillem_config::Config::from_file(&config_path)?
    };
    debug!("Config loaded: {} providers, default provider={:?}, default model={:?}",
        config.providers.len(), config.defaults.provider, config.defaults.model);

    // 3. Expand ~ in database path
    let db_path = shellexpand::tilde(&config.database.path).to_string();

    // 4. Open the SQLite database (creating parent directory if needed)
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = tuillem_db::Db::open(&db_path)?;

    // 5. Initialize providers from config; log warnings for failures but don't abort
    let mut providers: HashMap<String, Box<dyn tuillem_provider::Provider>> = HashMap::new();
    for pc in &config.providers {
        debug!("Initializing provider '{}' (type={:?}, base_url={:?})", pc.name, pc.provider_type, pc.base_url);
        match tuillem_provider::create_provider(pc) {
            Ok(p) => {
                info!("Provider '{}' initialized with {} models", pc.name, p.models().len());
                providers.insert(pc.name.clone(), p);
            }
            Err(e) => {
                error!("Failed to initialize provider '{}': {}", pc.name, e);
            }
        }
    }
    debug!("Total providers initialized: {}", providers.len());

    // 6. Initialize PluginHost from config tools
    let plugin_host = tuillem_plugin::PluginHost::new(config.tools.clone());

    // 7. Determine default provider and model from config
    let default_provider = config.defaults.provider.clone().unwrap_or_else(|| {
        config
            .providers
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_default()
    });
    let default_model = config.defaults.model.clone().unwrap_or_else(|| {
        config
            .providers
            .first()
            .and_then(|p| {
                p.default_model
                    .clone()
                    .or_else(|| p.models.first().cloned())
            })
            .unwrap_or_default()
    });

    // 8. Create mpsc channels (unbounded) for actions and events
    let (action_tx, action_rx) = mpsc::unbounded_channel();
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // 9. Build Theme from config
    let theme = tuillem_tui::theme::Theme::from_config(&config.theme, &config.themes);

    // 10. Build AppState with default provider/model
    let state = tuillem_core::AppState::new(default_provider.clone(), default_model.clone());

    // 11. Build App with state, theme, action_tx, editor command from config
    let available_models: Vec<(String, Vec<String>)> = config
        .providers
        .iter()
        .map(|p| (p.name.clone(), p.models.clone()))
        .collect();
    let app = tuillem_tui::app::App::new(
        state,
        theme,
        action_tx,
        config.editor.clone(),
        available_models,
    );

    // 12. Spawn coordinator on a dedicated thread (rusqlite::Connection is !Sync)
    debug!("Starting coordinator with provider='{}', model='{}'", default_provider, default_model);
    let coordinator = tuillem_core::Coordinator::new(
        db,
        providers,
        plugin_host,
        default_provider,
        default_model,
        config.defaults.system_prompt.clone(),
    );
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build coordinator runtime");
        info!("Coordinator thread started");
        rt.block_on(coordinator.run(action_rx, event_tx));
        info!("Coordinator thread exiting");
    });

    // 13. Run TUI (blocks until quit)
    tuillem_tui::run(app, event_rx, config.ui.mouse).await?;

    Ok(())
}
