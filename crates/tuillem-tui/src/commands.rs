use tuillem_core::actions::Action;

/// Result of parsing a slash command.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Action to send to the coordinator.
    pub action: Option<Action>,
    /// Status message to display.
    pub message: Option<String>,
    /// Error message to display.
    pub error: Option<String>,
    /// Whether to show the commands help overlay.
    pub show_help: bool,
    /// Whether to toggle thinking on.
    pub set_thinking: Option<bool>,
    /// Whether to set system prompt for this session.
    pub set_system_prompt: Option<String>,
    /// Whether to request clearing the conversation (needs confirmation).
    pub request_clear: bool,
}

impl CommandResult {
    fn ok(message: impl Into<String>) -> Self {
        Self {
            action: None,
            message: Some(message.into()),
            error: None,
            show_help: false,
            set_thinking: None,
            set_system_prompt: None,
            request_clear: false,
        }
    }

    fn action(action: Action, message: impl Into<String>) -> Self {
        Self {
            action: Some(action),
            message: Some(message.into()),
            error: None,
            show_help: false,
            set_thinking: None,
            set_system_prompt: None,
            request_clear: false,
        }
    }

    fn err(error: impl Into<String>) -> Self {
        Self {
            action: None,
            message: None,
            error: Some(error.into()),
            show_help: false,
            set_thinking: None,
            set_system_prompt: None,
            request_clear: false,
        }
    }

    fn help() -> Self {
        Self {
            action: None,
            message: None,
            error: None,
            show_help: true,
            set_thinking: None,
            set_system_prompt: None,
            request_clear: false,
        }
    }
}

/// Context needed by some commands to produce results.
pub struct CommandContext<'a> {
    pub current_provider: &'a str,
    pub current_model: &'a str,
    pub active_session_id: Option<&'a str>,
    pub message_count: usize,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub available_models: &'a [(String, Vec<String>)],
}

/// Parse input as a slash command. Returns `None` if input doesn't start with
/// the command prefix (or if the prefix is empty, meaning commands are disabled).
pub fn parse_command(input: &str, prefix: &str, ctx: &CommandContext) -> Option<CommandResult> {
    if prefix.is_empty() {
        return None;
    }

    let trimmed = input.trim();
    if !trimmed.starts_with(prefix) {
        return None;
    }

    let rest = &trimmed[prefix.len()..];
    if rest.is_empty() {
        return Some(CommandResult::err(
            "Empty command. Type /help for available commands.",
        ));
    }

    let mut parts = rest.splitn(2, ' ');
    let cmd = parts.next().unwrap_or("").to_lowercase();
    let args = parts.next().unwrap_or("").trim();

    Some(match cmd.as_str() {
        "help" => CommandResult::help(),
        "new" => CommandResult::action(
            Action::CreateSession {
                title: "New Chat".to_string(),
            },
            "New conversation created",
        ),
        "export" => CommandResult::action(Action::SaveTranscript, "Exporting transcript..."),
        "clear" => {
            let mut r = CommandResult::ok("");
            r.request_clear = true;
            r
        }
        "model" => CommandResult::ok(format!(
            "Model: {} (provider: {})",
            ctx.current_model, ctx.current_provider
        )),
        "stats" => CommandResult::ok(format!(
            "Messages: {} | Tokens in: {} | Tokens out: {} | Total: {}",
            ctx.message_count,
            ctx.total_tokens_in,
            ctx.total_tokens_out,
            ctx.total_tokens_in + ctx.total_tokens_out,
        )),
        "set" => parse_set_command(args, ctx),
        "tag" => parse_tag_command(args, ctx),
        "untag" => parse_untag_command(args, ctx),
        "rename" => parse_rename_command(args, ctx),
        _ => CommandResult::err(format!(
            "Unknown command: {}{}. Type {}help for available commands.",
            prefix, cmd, prefix
        )),
    })
}

fn parse_set_command(args: &str, ctx: &CommandContext) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err(
            "Usage: /set <think|nothink|model <name>|provider <name>|system <prompt>>",
        );
    }

    let mut parts = args.splitn(2, ' ');
    let sub = parts.next().unwrap_or("").to_lowercase();
    let sub_args = parts.next().unwrap_or("").trim();

    match sub.as_str() {
        "think" => {
            let mut r = CommandResult::ok("Thinking enabled");
            r.set_thinking = Some(true);
            r
        }
        "nothink" => {
            let mut r = CommandResult::ok("Thinking disabled");
            r.set_thinking = Some(false);
            r
        }
        "model" => {
            if sub_args.is_empty() {
                return CommandResult::err("Usage: /set model <name>");
            }
            // Fuzzy match against available models for the current provider
            let provider_models: Vec<&str> = ctx
                .available_models
                .iter()
                .find(|(name, _)| name == ctx.current_provider)
                .map(|(_, models)| models.iter().map(|s| s.as_str()).collect())
                .unwrap_or_default();

            let query = sub_args.to_lowercase();
            // Try exact match first, then contains match
            let matched = provider_models
                .iter()
                .find(|m| m.to_lowercase() == query)
                .or_else(|| provider_models.iter().find(|m| m.to_lowercase().contains(&query)));

            match matched {
                Some(model) => CommandResult::action(
                    Action::SwitchModel {
                        provider: ctx.current_provider.to_string(),
                        model: model.to_string(),
                    },
                    format!("Switched to model: {}", model),
                ),
                None => {
                    let available: Vec<&str> = provider_models.iter().take(5).copied().collect();
                    CommandResult::err(format!(
                        "No model matching '{}'. Available: {}",
                        sub_args,
                        if available.is_empty() {
                            "(none)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ))
                }
            }
        }
        "provider" => {
            if sub_args.is_empty() {
                return CommandResult::err("Usage: /set provider <name>");
            }
            let query = sub_args.to_lowercase();
            let matched = ctx
                .available_models
                .iter()
                .find(|(name, _)| name.to_lowercase() == query)
                .or_else(|| {
                    ctx.available_models
                        .iter()
                        .find(|(name, _)| name.to_lowercase().contains(&query))
                });

            match matched {
                Some((provider, models)) => {
                    let model = models.first().cloned().unwrap_or_default();
                    CommandResult::action(
                        Action::SwitchModel {
                            provider: provider.clone(),
                            model: model.clone(),
                        },
                        format!("Switched to provider: {} (model: {})", provider, model),
                    )
                }
                None => {
                    let available: Vec<&str> = ctx
                        .available_models
                        .iter()
                        .take(5)
                        .map(|(n, _)| n.as_str())
                        .collect();
                    CommandResult::err(format!(
                        "No provider matching '{}'. Available: {}",
                        sub_args,
                        if available.is_empty() {
                            "(none)".to_string()
                        } else {
                            available.join(", ")
                        }
                    ))
                }
            }
        }
        "system" => {
            if sub_args.is_empty() {
                return CommandResult::err("Usage: /set system <prompt>");
            }
            let mut r = CommandResult::ok(format!(
                "System prompt set ({}chars)",
                sub_args.len()
            ));
            r.set_system_prompt = Some(sub_args.to_string());
            r
        }
        _ => CommandResult::err(format!(
            "Unknown set option: '{}'. Options: think, nothink, model, provider, system",
            sub
        )),
    }
}

fn parse_tag_command(args: &str, ctx: &CommandContext) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Usage: /tag <tagname>");
    }
    match ctx.active_session_id {
        Some(session_id) => CommandResult::action(
            Action::AddTag {
                session_id: session_id.to_string(),
                tag: args.to_string(),
            },
            format!("Tag added: #{}", args),
        ),
        None => CommandResult::err("No active conversation to tag"),
    }
}

fn parse_untag_command(args: &str, ctx: &CommandContext) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Usage: /untag <tagname>");
    }
    match ctx.active_session_id {
        Some(session_id) => CommandResult::action(
            Action::RemoveTag {
                session_id: session_id.to_string(),
                tag: args.to_string(),
            },
            format!("Tag removed: #{}", args),
        ),
        None => CommandResult::err("No active conversation to untag"),
    }
}

fn parse_rename_command(args: &str, ctx: &CommandContext) -> CommandResult {
    if args.is_empty() {
        return CommandResult::err("Usage: /rename <title>");
    }
    match ctx.active_session_id {
        Some(session_id) => CommandResult::action(
            Action::RenameSession {
                id: session_id.to_string(),
                title: args.to_string(),
            },
            format!("Renamed to: {}", args),
        ),
        None => CommandResult::err("No active conversation to rename"),
    }
}

/// Render function for the commands help overlay.
pub fn render_commands_help(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    theme: &crate::theme::Theme,
    prefix: &str,
    scroll: u16,
) {
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    };

    let popup_width = 64u16.min(area.width.saturating_sub(6));
    let popup_height = 30u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let accent = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.thinking_fg);
    let normal = Style::default().fg(theme.fg);

    let p = prefix;

    let lines = vec![
        Line::from(Span::styled("Settings", accent)),
        Line::from(vec![
            Span::styled(format!("  {}set think      ", p), normal),
            Span::styled("Enable thinking/reasoning mode", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}set nothink    ", p), normal),
            Span::styled("Disable thinking mode", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}set model <n>  ", p), normal),
            Span::styled("Switch to model (fuzzy match)", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}set provider <n>", p), normal),
            Span::styled("Switch provider", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}set system <p> ", p), normal),
            Span::styled("Set system prompt for session", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Conversation", accent)),
        Line::from(vec![
            Span::styled(format!("  {}tag <name>     ", p), normal),
            Span::styled("Add tag to conversation", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}untag <name>   ", p), normal),
            Span::styled("Remove tag from conversation", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}rename <title> ", p), normal),
            Span::styled("Rename current conversation", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}new            ", p), normal),
            Span::styled("Create new conversation", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}clear          ", p), normal),
            Span::styled("Clear conversation messages", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}export         ", p), normal),
            Span::styled("Save transcript to file", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Info", accent)),
        Line::from(vec![
            Span::styled(format!("  {}model          ", p), normal),
            Span::styled("Show current model info", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}stats          ", p), normal),
            Span::styled("Show session statistics", dim),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}help           ", p), normal),
            Span::styled("Show this help screen", dim),
        ]),
    ];

    let total_lines = lines.len() as u16;
    let inner_height = popup_height.saturating_sub(2);

    let scroll_hint = if total_lines > inner_height {
        " j/k:scroll  Esc:close "
    } else {
        " Esc:close "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(Span::styled(" Slash Commands ", accent)))
        .title_bottom(Line::from(Span::styled(scroll_hint, dim)))
        .style(Style::default().bg(theme.bg));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, popup_area);

    // Scrollbar if content exceeds popup
    if total_lines > inner_height {
        let inner_area = ratatui::layout::Rect::new(
            popup_area.x + 1,
            popup_area.y + 1,
            popup_area.width.saturating_sub(2),
            inner_height,
        );
        let mut scrollbar_state = ScrollbarState::new(total_lines as usize)
            .position(scroll as usize)
            .viewport_content_length(inner_height as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        frame.render_stateful_widget(scrollbar, inner_area, &mut scrollbar_state);
    }
}

/// Returns the maximum scroll offset for the commands help overlay at the given area size.
pub fn commands_help_max_scroll(area: ratatui::layout::Rect) -> u16 {
    let popup_height = 30u16.min(area.height.saturating_sub(4));
    let inner_height = popup_height.saturating_sub(2);
    let total_lines: u16 = 20; // number of lines in the commands help content
    total_lines.saturating_sub(inner_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> CommandContext<'static> {
        CommandContext {
            current_provider: "anthropic",
            current_model: "claude-sonnet-4-20250514",
            active_session_id: Some("sess-123"),
            message_count: 10,
            total_tokens_in: 5000,
            total_tokens_out: 3000,
            available_models: &[],
        }
    }

    #[test]
    fn test_not_a_command() {
        let ctx = test_ctx();
        assert!(parse_command("hello world", "/", &ctx).is_none());
        assert!(parse_command("", "/", &ctx).is_none());
    }

    #[test]
    fn test_empty_prefix_disables() {
        let ctx = test_ctx();
        assert!(parse_command("/help", "", &ctx).is_none());
    }

    #[test]
    fn test_help_command() {
        let ctx = test_ctx();
        let result = parse_command("/help", "/", &ctx).unwrap();
        assert!(result.show_help);
    }

    #[test]
    fn test_new_command() {
        let ctx = test_ctx();
        let result = parse_command("/new", "/", &ctx).unwrap();
        assert!(result.action.is_some());
        assert!(result.message.as_ref().unwrap().contains("New conversation"));
    }

    #[test]
    fn test_model_command() {
        let ctx = test_ctx();
        let result = parse_command("/model", "/", &ctx).unwrap();
        assert!(result.message.as_ref().unwrap().contains("claude-sonnet-4-20250514"));
    }

    #[test]
    fn test_stats_command() {
        let ctx = test_ctx();
        let result = parse_command("/stats", "/", &ctx).unwrap();
        let msg = result.message.unwrap();
        assert!(msg.contains("10"));
        assert!(msg.contains("5000"));
    }

    #[test]
    fn test_set_think() {
        let ctx = test_ctx();
        let result = parse_command("/set think", "/", &ctx).unwrap();
        assert_eq!(result.set_thinking, Some(true));
    }

    #[test]
    fn test_set_nothink() {
        let ctx = test_ctx();
        let result = parse_command("/set nothink", "/", &ctx).unwrap();
        assert_eq!(result.set_thinking, Some(false));
    }

    #[test]
    fn test_set_system() {
        let ctx = test_ctx();
        let result = parse_command("/set system You are a pirate", "/", &ctx).unwrap();
        assert_eq!(
            result.set_system_prompt.as_deref(),
            Some("You are a pirate")
        );
    }

    #[test]
    fn test_tag_command() {
        let ctx = test_ctx();
        let result = parse_command("/tag research", "/", &ctx).unwrap();
        assert!(result.action.is_some());
        assert!(result.message.as_ref().unwrap().contains("#research"));
    }

    #[test]
    fn test_untag_command() {
        let ctx = test_ctx();
        let result = parse_command("/untag research", "/", &ctx).unwrap();
        assert!(result.action.is_some());
    }

    #[test]
    fn test_rename_command() {
        let ctx = test_ctx();
        let result = parse_command("/rename My Cool Chat", "/", &ctx).unwrap();
        assert!(result.action.is_some());
        assert!(result.message.as_ref().unwrap().contains("My Cool Chat"));
    }

    #[test]
    fn test_clear_command() {
        let ctx = test_ctx();
        let result = parse_command("/clear", "/", &ctx).unwrap();
        assert!(result.request_clear);
    }

    #[test]
    fn test_unknown_command() {
        let ctx = test_ctx();
        let result = parse_command("/foobar", "/", &ctx).unwrap();
        assert!(result.error.is_some());
        assert!(result.error.as_ref().unwrap().contains("Unknown command"));
    }

    #[test]
    fn test_case_insensitive() {
        let ctx = test_ctx();
        let result = parse_command("/HELP", "/", &ctx).unwrap();
        assert!(result.show_help);
    }

    #[test]
    fn test_custom_prefix() {
        let ctx = test_ctx();
        let result = parse_command("!help", "!", &ctx).unwrap();
        assert!(result.show_help);
        assert!(parse_command("/help", "!", &ctx).is_none());
    }

    #[test]
    fn test_tag_no_session() {
        let ctx = CommandContext {
            active_session_id: None,
            ..test_ctx()
        };
        let result = parse_command("/tag test", "/", &ctx).unwrap();
        assert!(result.error.is_some());
    }

    #[test]
    fn test_set_model_fuzzy() {
        let models: Vec<(String, Vec<String>)> = vec![(
            "anthropic".to_string(),
            vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
        )];
        let ctx = CommandContext {
            available_models: &models,
            ..test_ctx()
        };
        let result = parse_command("/set model haiku", "/", &ctx).unwrap();
        assert!(result.action.is_some());
        assert!(result.message.as_ref().unwrap().contains("haiku"));
    }

    #[test]
    fn test_export_command() {
        let ctx = test_ctx();
        let result = parse_command("/export", "/", &ctx).unwrap();
        assert!(result.action.is_some());
    }

    #[test]
    fn test_empty_command() {
        let ctx = test_ctx();
        let result = parse_command("/", "/", &ctx).unwrap();
        assert!(result.error.is_some());
    }
}
