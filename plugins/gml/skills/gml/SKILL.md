---
name: gml
description: Use when the user wants to provision an ephemeral cloud GPU node and run jobs on it with the `gml` CLI — spinning up a node (e.g. on Lambda), running training/inference/shell commands on it over SSH, syncing local code to it, and tearing it down. Triggers on "gml", "spin up a GPU", "run this on a GPU node", "train on an A10/A100/H100", "rent a GPU box".
---

# Running jobs on GPU nodes with `gml`

`gml` provisions **ephemeral** GPU nodes across cloud providers (Lambda today) and runs
commands on them over SSH. Nodes are billed by the hour, so the golden rule is: **always
create with a timeout, and delete the node the moment the work is done.**

## The workflow (always in this order)

1. Check the provider is configured.
2. Pick an instance type and confirm capacity + price.
3. Create the node **with a `--timeout`** (safety net) and a `--name`.
4. Wait for it to be reachable.
5. Run jobs with `gml run`.
6. **Delete the node** (`gml node delete`) when finished.

## 0. Prerequisites

`gml` reads provider config from `~/.gml/config.toml`. For Lambda it needs an `api-key`,
an `ssh-key-name` already registered on the Lambda account, and a default `region`:

```toml
[lambda]
api-key = "..."
ssh-key-name = "Macbook"   # name of an SSH key on your Lambda account
region = "us-west-1"
```

SSH auth for `gml run`/`gml connect` uses your **default SSH identity / agent** — the
private key matching `ssh-key-name` must be loaded locally. If a `gml run` hangs or fails
auth, that key isn't available to your SSH agent.

The `gmld` daemon enforces timeouts (it deletes expired nodes). `gml node create`
auto-starts it if a `gmld` binary sits next to `gml`; otherwise run `gmld` yourself.

## 1. Pick an instance type

List what a provider offers, with **price** and **which regions have capacity**:

```bash
gml node list-types --provider lambda
```

This returns JSON keyed by instance type. Each entry has `price_cents_per_hour` and
`regions_with_capacity_available`. Pick a `name` (e.g. `gpu_1x_a10`) whose region list
includes the region you'll use. Common Lambda types: `gpu_1x_a10` (~129¢/hr),
`gpu_1x_a100_sxm4` (~199¢/hr).

**Before creating anything billable, tell the user the instance type and hourly price and
confirm** — unless they've already told you exactly what to launch.

## 2. Create the node

```bash
gml node create --provider lambda --instance-type gpu_1x_a10 --timeout 1h --name my-job
```

- `--timeout` accepts durations like `30m`, `1h`, `1h30m`, `2h`. **Never omit it** — it's
  the backstop that stops a forgotten node from billing forever.
- `--name` gives you a friendly handle to use with `ls`, `run`, `connect`, and `delete`
  instead of the generated ID.
- `--region` overrides the config default if needed (must be a region with capacity from
  step 1).

## 3. Wait until it's reachable

```bash
gml ls   # shows ID/name, IP, instance type, and time remaining
```

The node needs ~30–60s to boot and accept SSH after it appears. Poll the SSH port before
running the first job:

```bash
IP=$(...from `gml ls`...)
until nc -z -w 3 "$IP" 22; do sleep 5; done
```

## 4. Run jobs with `gml run`

`gml run` executes a command on the node over SSH, **streams output live**, and **exits
with the remote command's exit code** — so you can detect success/failure normally.

```bash
gml run my-job "nvidia-smi"                       # confirm the GPU
gml run my-job "python train.py --epochs 10"      # run a job
```

**The command is a single quoted string**, passed verbatim to the remote shell (exactly
like `ssh host "<command>"`). Quote the whole thing; whatever you quote is what runs, so
nested quoting and shell operators work:

```bash
gml run my-job "cd data && ./prepare.sh && python train.py"
gml run my-job "sh -c 'exit 42'"                  # really exits 42
```

If you forget the quotes, `gml` errors immediately (it won't silently run the wrong thing).

### Sync local code to the node: `--sync`

`--sync` rsyncs your **current directory** to the node first (respecting `.gitignore`),
then runs the command **inside** that synced directory:

```bash
gml run my-job --sync "python train.py"           # push cwd, then run it there
```

### Long-running jobs: `--detach`

`--detach` starts the command in the background so it survives disconnects; output goes to
`~/gml-run.log` on the node. The call returns immediately.

```bash
gml run my-job --detach "python train.py --epochs 100"
gml run my-job "tail -n 50 gml-run.log"           # check on it later
```

Note `~/gml-run.log` is a single fixed file — a second detached job overwrites the first
job's log.

## 5. Manage the timeout while working

If a job needs longer than the original timeout, extend it — don't let the node get reaped
mid-run:

```bash
gml node timeout reset --id my-job --duration 2h
gml node timeout remove --id my-job    # remove the auto-shutdown entirely (use with care)
```

## 6. Delete the node when done

**This is not optional.** As soon as the work is finished (results copied off, etc.),
delete the node so billing stops:

```bash
gml node delete my-job
gml ls                 # confirm it's gone
```

The `--timeout` from step 2 is only a backstop for when you forget — deleting promptly is
the real cost control.

## Quick reference

| Goal | Command |
| --- | --- |
| See instance types + prices + capacity | `gml node list-types --provider lambda` |
| Create a node (always with a timeout) | `gml node create --provider lambda --instance-type <type> --timeout 1h --name <name>` |
| List nodes / IPs / time remaining | `gml ls` |
| Run a command (single quoted string) | `gml run <name> "<command>"` |
| Sync cwd then run | `gml run <name> --sync "<command>"` |
| Run detached (logs to ~/gml-run.log) | `gml run <name> --detach "<command>"` |
| Extend the timeout | `gml node timeout reset --id <name> --duration 2h` |
| Open the node in Cursor over SSH | `gml connect <name>` |
| **Delete the node (stop billing)** | `gml node delete <name>` |
