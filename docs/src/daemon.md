# Daemon (gmld)

`gmld` is a small daemon that enforces timeouts by periodically reading `~/.gml/state.json` and deleting expired resources. The check granularity is **one minute**. Logs are written to `~/.gml/gmld.log`.

When you run `gml node create`, `gml` tries to start `gmld` automatically if it finds a `gmld` binary **next to** the `gml` executable. You can also run the daemon yourself:

```bash
gmld
```
