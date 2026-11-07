# Quick Start

### 1. Create a project

```bash
tmkpr project add "Client Work"
```

### 2. Add tasks to the project

```bash
tmkpr task add "Feature Development" -p "Client Work"
tmkpr task add "Bug Fixes" -p "Client Work"
```

### 3. Start tracking

```bash
# Start timing
tmkpr start -p "Client Work" -t "Feature Development"

# Do some work...

# Stop timing
tmkpr stop

# Or log directly, without a start/stop cycle
tmkpr log -s "9:00am" -e "11:30am" -p "Client Work" -t "Feature Development" -n "implemented auth flow"
```

### 4. View your work

```bash
# Today's entries
tmkpr list

# This week's report
tmkpr report --wweek

# All entries for a project
tmkpr list -p "Client Work"
```

### 5. Or use the interactive dashboard

```bash
# Launch the terminal UI
tmkpr-ui
```

### 6. Or work with Pomodoro sessions

```bash
# Launch the Pomodoro timer
tmkpr-pomodoro
```

All three tools read and write the same database, so you can mix and match freely — see the [Configuration](./configuration.md) chapter for where that database lives and how to override it.
