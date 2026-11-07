# Comments

Add free-form notes to any entry.

```text
tmkpr comment add [-e ENTRY-ID] <TEXT...>
tmkpr comment list              # comments on active entry
tmkpr comment list <ENTRY-ID>   # comments on a specific entry
tmkpr comment edit <ID> <TEXT...>
tmkpr comment delete <ID> [-y]
```

`comment add` targets the active entry by default; use `-e` to add to any finished entry:

```bash
tmkpr comment add just deployed the fix
tmkpr comment add -e abc12345 actually deployed to staging
```

Aliases: `c` for the subcommand, `a` / `ls` / `e` / `d` for the actions. Comment IDs can be abbreviated to any unambiguous prefix (8+ chars).
