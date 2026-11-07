# Themes

`tmkpr-ui` and `tmkpr-pomodoro` ship 23 built-in colour themes:

`default`, `ayu_dark`, `catppuccin_frappe`, `catppuccin_latte`, `catppuccin_macchiato`, `catppuccin_mocha`, `cobalt`, `dracula`, `everforest`, `github_dark`, `github_light`, `gruvbox_dark`, `gruvbox_light`, `high_contrast`, `kanagawa`, `matrix`, `monokai`, `nord`, `onedark`, `rose_pine`, `solarized_dark`, `solarized_light`, `tokyonight`

Select a theme via `--theme <name>`, the `TMKPR_THEME` environment variable, or the `theme` key in `config.toml`. In the TUI you can also type `:theme <name>` and use Tab to cycle through themes with a live preview — run `:config-write` to persist the choice.

## Custom themes

Define your own theme in `config.toml` using hex colour values:

```toml
[themes.my_theme]
bg        = "#1e1e2e"
fg        = ""          # leave empty for dark themes (uses terminal default); set a hex colour for light themes
active    = "#a6e3a1"
accent    = "#cba6f7"
dim       = "#7f849c"
error     = "#f38ba8"
warning   = "#f9e2af"
selection = "#313244"
border    = "#45475a"
```

For light themes, set `fg` to a dark hex colour so text is readable against the light background:

```toml
[themes.my_light]
bg        = "#ffffff"
fg        = "#1f2328"   # dark text for light background
active    = "#0969da"
accent    = "#8250df"
dim       = "#656d76"
error     = "#cf222e"
warning   = "#9a6700"
selection = "#ddf4ff"
border    = "#d0d7de"
```

Then set `theme = "my_theme"` under `[display]`.
