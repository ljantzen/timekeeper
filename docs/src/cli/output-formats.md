# Output Formats

All list-producing commands accept `--format table` (default), `--format json`, `--format csv`, or `--format markdown`.

```bash
tmkpr list --format json
tmkpr list --format markdown
tmkpr report --format csv
tmkpr report --format markdown
tmkpr project list --format json
tmkpr task list -p myproject --format csv
```

`project list --format json` includes a `tasks` array on each project object, making it suitable for scripting:

```bash
tmkpr project list --format json \
  | jq -r '.[] | . as $p | .tasks[] | "\($p.name) / \(.name)"'
```

CSV output uses raw seconds for duration fields to keep it machine-readable.

See [Configuration](../configuration.md) for the `time_format` setting that controls how times are displayed in table output.
