# gml

`gml` is a CLI for creating and managing ephemeral GPU compute (nodes today; clusters are WIP) across pluggable cloud providers. It tracks created resources locally and can enforce automatic shutdown via a companion daemon (`gmld`).

## Install

Build and install both binaries:

```bash
cargo install --path crates/cli --locked
cargo install --path crates/daemon --locked
```

Or build from source and use `target/release/{gml,gmld}`:

```bash
cargo build -p gml -p gml-daemon --release
```

## Configure

`gml` reads provider config from `~/.gml/config.toml`.

## Usage

- **Create a node**:

```bash
gml node create --provider lambda --instance-type <type> --timeout 2h
```

- **List nodes / clusters**:

```bash
gml ls
```

- **Connect to a node** (syncs your current folder to the node and opens Cursor over SSH):

```bash
gml connect <node-id>
```

- **Delete a node**:

```bash
gml node delete <node-id>
```

- **Manage node timeouts**:

```bash
gml node timeout reset --id <node-id> --duration 1h30m
gml node timeout remove --id <node-id>
```

## Providers

### Lambda provider

The Lambda provider currently supports **creating and deleting a node** and defaults to the **latest Lambda Stack** image.

Add a `lambda` block to `~/.gml/config.toml`:

```toml
[lambda]
api-key = "..."
ssh-key-name = "..."
region = "..."
```

The `ssh-key-name` field is the name of an SSH public key which you have already added to your Lambda account.

## gmld (the daemon)

`gmld` is a small daemon that enforces timeouts by periodically reading `~/.gml/state.json` and deleting any expired resources (granularity: **1 minute**). Logs are written to `~/.gml/gmld.log`.

`gml node create` will try to auto-start `gmld` if it can find a `gmld` binary **next to** the `gml` executable; you can also run it yourself:

```bash
gmld
```
