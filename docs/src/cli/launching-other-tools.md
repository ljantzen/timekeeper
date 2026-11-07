# Launching Other Tools

`tmkpr ui` and `tmkpr pomodoro` launch the sibling terminal apps without you needing to remember their binary names. All flags are forwarded, including `--db`:

```bash
tmkpr ui                        # launch tmkpr-ui
tmkpr u                         # alias
tmkpr ui --theme dracula        # pass flags through

tmkpr pomodoro                  # launch tmkpr-pomodoro
tmkpr pomo                      # alias
tmkpr p25                       # alias

tmkpr --db /path/to/other.db ui         # --db is forwarded automatically
tmkpr --db /path/to/other.db pomodoro
```
