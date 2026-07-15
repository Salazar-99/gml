# Usage

## Create a node

```bash
gml node create --provider <provider> --instance-type <type> --timeout 2h
```

## List nodes and clusters

```bash
gml ls
```

## Connect to a node

Syncs your current folder to the node and opens Cursor over SSH:

```bash
gml connect <node-id>
```

## Run a command on a node

Runs a command on the node over SSH without opening an editor. Output streams
live and `gml` exits with the remote command's exit code, so it composes in
scripts and CI.

```bash
gml run <node-id> "python train.py --epochs 10"   # stream output, exit with the job's code
gml run <node-id> --sync "python train.py"          # rsync the current folder first, then run
gml run <node-id> --detach "python train.py"        # fire-and-forget; logs to ~/gml-run.log
```

The command is a **single quoted string**, passed through verbatim to the remote
shell (just like `ssh host "<command>"`). Whatever you quote is exactly what runs,
so nested quoting works — e.g. `gml run <node-id> "sh -c 'exit 42'"` exits 42.

## Delete a node

```bash
gml node delete <node-id>
```

## Manage node timeouts

```bash
gml node timeout reset --id <node-id> --duration 1h30m
gml node timeout remove --id <node-id>
```
