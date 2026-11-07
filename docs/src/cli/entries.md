# Entries

```text
tmkpr list     [--from TIME] [--until TIME] [-p PROJECT] [-t TASK] [-l LIMIT] [--tag TAG] [--gaps] [--min-gap MINUTES]
tmkpr report   [--from TIME] [--until TIME] [-p PROJECT] [--tag TAG]
tmkpr report   --week [N] [--year YEAR] [--tag TAG]
tmkpr report   --wweek [N] [--year YEAR] [--tag TAG]
tmkpr edit     <ID> [-p PROJECT] [-t TASK] [-n NOTE] [--start TIME] [--end TIME] [--tags t1,t2 | --add-tag TAG --remove-tag TAG]
tmkpr delete   <ID> [-y]
tmkpr merge    <ID> [--prev]
tmkpr fill-gap [ID]
```

## Reports

`--week` shows a full 7-day ISO week (Mon–Sun). `--wweek` shows only Mon–Fri. Both accept an optional week number and a `--year` flag:

```bash
tmkpr report --week           # current full week
tmkpr report --wweek          # current working week (Mon–Fri)
tmkpr report --wweek 12       # working week 12 of the current year
tmkpr report --week 12 --year 2025
```

## Listing

`list` with no `--from`/`--until` shows today's entries only. Pass `--from` to widen the range:

```bash
tmkpr list --from "last monday"
tmkpr list --from "2025-01-01"
```

## Merging and gap filling

`merge` (alias: `m`) joins an entry into the chronologically next entry that shares its project and task. Pass `--prev` to merge into the previous entry instead.

`fill-gap` (alias: `fg`) extends an entry's start and/or end times to abut adjacent entries on the same day. Omit `ID` to operate on the active entry.

## Editing

Entry IDs can be abbreviated to any unambiguous prefix (8+ chars).

`edit --tags` replaces all tags on an entry. To add or remove individual tags without specifying the full set, use `--add-tag` and `--remove-tag` (each can be given multiple times; both conflict with `--tags`):

```bash
tmkpr edit abc12345 --add-tag billable
tmkpr edit abc12345 --remove-tag wip --add-tag done
```
