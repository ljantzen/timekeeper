# Events

Point-in-time events are entries with no duration — useful for marking moments (deploys, meetings, etc.).

```text
tmkpr event add    [--at TIME] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
tmkpr event list   [--from TIME] [--until TIME] [-p PROJECT] [--tag TAG] [-l LIMIT]
tmkpr event edit   <ID> [--at TIME] [-p PROJECT] [-t TASK] [-n NOTE] [--tags t1,t2]
tmkpr event delete <ID> [-y]
```

`--at` accepts natural language or ISO 8601 and defaults to now. Events also appear in `tmkpr list` alongside regular entries.
