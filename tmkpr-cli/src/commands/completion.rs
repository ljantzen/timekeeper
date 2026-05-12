use anyhow::Result;
use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionArgs};

/// Generates a static completion script for the given shell.
///
/// For dynamic completion (including project and task name suggestions), source
/// the completions directly from the binary instead:
///
///   bash:  source <(COMPLETE=bash tmkpr)
///   zsh:   source <(COMPLETE=zsh tmkpr)
///   fish:  COMPLETE=fish tmkpr | source
pub fn run(args: CompletionArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(args.shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}
