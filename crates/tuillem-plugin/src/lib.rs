use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tuillem_config::ToolConfig;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Tool timed out after {0:?}")]
    Timeout(Duration),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// ToolInput / ToolOutput
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub output: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// PluginHost
// ---------------------------------------------------------------------------

pub struct PluginHost {
    tools: HashMap<String, ToolConfig>,
}

impl PluginHost {
    pub fn new(tools: Vec<ToolConfig>) -> Self {
        let tools = tools
            .into_iter()
            .map(|t| (t.name.clone(), t))
            .collect();
        Self { tools }
    }

    pub fn list_tools(&self) -> Vec<&ToolConfig> {
        self.tools.values().collect()
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolConfig> {
        self.tools.get(name)
    }

    pub fn requires_confirmation(&self, name: &str) -> bool {
        self.tools.get(name).map(|t| t.confirm).unwrap_or(false)
    }

    pub async fn invoke(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<ToolOutput, PluginError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let timeout = parse_duration(&tool.timeout);

        // Split command into program + args
        let parts: Vec<&str> = tool.command.split_whitespace().collect();
        let (program, args) = parts
            .split_first()
            .ok_or_else(|| PluginError::Execution("Empty command".to_string()))?;

        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Set environment variables
        for (k, v) in &tool.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;

        // Write JSON to stdin
        let tool_input = ToolInput {
            name: name.to_string(),
            input,
        };
        let input_json = serde_json::to_string(&tool_input)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input_json.as_bytes()).await?;
            // Drop stdin to signal EOF
            drop(stdin);
        }

        // Wait with timeout
        let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Try to parse stdout as ToolOutput JSON, fall back to raw stdout.
                // Only accept it if at least one of output/error is present.
                if let Ok(tool_output) = serde_json::from_str::<ToolOutput>(&stdout) {
                    if tool_output.output.is_some() || tool_output.error.is_some() {
                        return Ok(tool_output);
                    }
                }
                {
                    Ok(ToolOutput {
                        output: if stdout.is_empty() { None } else { Some(stdout) },
                        error: if stderr.is_empty() { None } else { Some(stderr) },
                    })
                }
            }
            Ok(Err(e)) => Err(PluginError::Io(e)),
            Err(_) => Err(PluginError::Timeout(timeout)),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        Duration::from_secs(secs.parse::<u64>().unwrap_or(30))
    } else if let Some(mins) = s.strip_suffix('m') {
        Duration::from_secs(mins.parse::<u64>().unwrap_or(1) * 60)
    } else {
        Duration::from_secs(s.parse::<u64>().unwrap_or(30))
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
    use std::collections::HashMap;

    fn make_tool(name: &str, command: &str, timeout: &str, confirm: bool) -> ToolConfig {
        ToolConfig {
            name: name.to_string(),
            description: format!("{name} tool"),
            command: command.to_string(),
            input_schema: None,
            timeout: timeout.to_string(),
            confirm,
            env: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_invoke_tool() {
        // `cat` reads stdin and writes to stdout
        let tool = make_tool("cat_tool", "cat", "10s", false);
        let host = PluginHost::new(vec![tool]);

        let input = serde_json::json!({"message": "hello"});
        let result = host.invoke("cat_tool", input).await;
        assert!(result.is_ok(), "invoke should succeed: {result:?}");

        let output = result.unwrap();
        // cat echoes the JSON input back as raw stdout
        assert!(output.output.is_some(), "output should be present");
        let text = output.output.unwrap();
        assert!(text.contains("hello"), "output should contain 'hello': {text}");
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let host = PluginHost::new(vec![]);
        let result = host.invoke("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), PluginError::NotFound(name) if name == "nonexistent")
        );
    }

    #[test]
    fn test_requires_confirmation() {
        let tool = make_tool("dangerous", "rm -rf", "10s", true);
        let host = PluginHost::new(vec![tool]);
        assert!(host.requires_confirmation("dangerous"));
        assert!(!host.requires_confirmation("nonexistent"));
    }

    #[test]
    fn test_list_tools() {
        let tools = vec![
            make_tool("a", "echo", "10s", false),
            make_tool("b", "cat", "10s", false),
            make_tool("c", "ls", "10s", false),
        ];
        let host = PluginHost::new(tools);
        assert_eq!(host.list_tools().len(), 3);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Duration::from_secs(30));
        assert_eq!(parse_duration("2m"), Duration::from_secs(120));
        assert_eq!(parse_duration("45"), Duration::from_secs(45));
    }

    #[tokio::test]
    async fn test_timeout() {
        let tool = make_tool("sleeper", "sleep 60", "1s", false);
        let host = PluginHost::new(vec![tool]);

        let result = host.invoke("sleeper", serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), PluginError::Timeout(d) if d == Duration::from_secs(1))
        );
    }
}
