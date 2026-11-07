# Shell Completion

## Dynamic completion (recommended)

Includes project and task name suggestions:

```bash
# bash — add to ~/.bashrc
source <(COMPLETE=bash tmkpr)

# zsh — add to ~/.zshrc
source <(COMPLETE=zsh tmkpr)

# fish — add to ~/.config/fish/completions/tmkpr.fish
COMPLETE=fish tmkpr | source
```

## Static completion

Flags and subcommands only, no dynamic project/task lookups:

```bash
tmkpr completion bash   >> ~/.bashrc
tmkpr completion zsh    >> ~/.zshrc
tmkpr completion fish   > ~/.config/fish/completions/tmkpr.fish
```
