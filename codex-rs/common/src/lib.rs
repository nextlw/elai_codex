#[cfg(feature = "cli")]
mod approval_mode_cli_arg;

#[cfg(feature = "elapsed")]
pub mod elapsed;

#[cfg(feature = "cli")]
pub use approval_mode_cli_arg::ApprovalModeCliArg;

#[cfg(feature = "cli")]
mod sandbox_mode_cli_arg;

#[cfg(feature = "cli")]
pub use sandbox_mode_cli_arg::SandboxModeCliArg;

#[cfg(feature = "cli")]
pub mod format_env_display;

#[cfg(any(feature = "cli", test))]
mod config_override;

#[cfg(feature = "cli")]
pub use config_override::CliConfigOverrides;

mod sandbox_summary;

#[cfg(feature = "sandbox_summary")]
pub use sandbox_summary::summarize_sandbox_policy;

mod config_summary;

pub use config_summary::create_config_summary_entries;
// Shared fuzzy matcher (used by TUI selection popups and other UI filtering)
pub mod fuzzy_match;
// Shared model presets used by TUI and MCP server
pub mod model_presets;
// Shared approval presets (AskForApproval + Sandbox) used by TUI and MCP server
// Not to be confused with AskForApproval, which we should probably rename to EscalationPolicy.
pub mod approval_presets;

#[must_use]
pub fn add_numbers(lhs: i64, rhs: i64) -> i64 {
    lhs + rhs
}

#[cfg(test)]
mod tests {
    use super::add_numbers;
    use pretty_assertions::assert_eq;

    #[test]
    fn adds_two_numbers() {
        assert_eq!(add_numbers(2, 3), 5);
    }
}
