# Projects & Tasks

## Projects

```text
tmkpr project add <NAME> [-d DESCRIPTION] [--color HEX]
tmkpr project list [--archived]
tmkpr project edit <NAME|ID> [--name NAME] [-d DESCRIPTION] [--color HEX]
tmkpr project delete <NAME|ID> [--hard]
```

Projects are listed with a numeric ID. Both the name and numeric ID are accepted wherever `NAME|ID` appears.

Deleting a project archives it by default. Use `--hard` to permanently remove it. Pass `-` to `--description` or `--color` to clear those fields.

## Tasks

```text
tmkpr task add        <NAME> -p PROJECT [-d DESCRIPTION]
tmkpr task list       -p PROJECT [--archived]
tmkpr task edit       <NAME|ID> -p PROJECT [--name NAME] [-d DESCRIPTION] [--move-to PROJECT]
tmkpr task delete     <NAME|ID> -p PROJECT [--hard]
tmkpr task done       <NAME|ID> -p PROJECT
tmkpr task reactivate <NAME|ID> -p PROJECT
```

`-p` identifies the project the task currently belongs to. Use `--move-to` to reassign it to a different project.

`task delete` archives by default; use `--hard` to permanently remove. `task done` marks a task completed; `task reactivate` reverses that.
