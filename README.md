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

- **Run a command on a node** (non-interactive over SSH; streams output and exits with the remote command's exit code):

```bash
gml run <node-id> "python train.py --epochs 10"   # run a job (command is a single quoted string)
gml run <node-id> --sync "python train.py"          # rsync current folder first, then run
gml run <node-id> --detach "python train.py"        # detached; logs to ~/gml-run.log
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

### DigitalOcean provider

The DigitalOcean provider currently supports **creating and deleting a node** (Droplet) and defaults to the **`ubuntu-22-04-x64`** image; override with the `GML_DIGITALOCEAN_IMAGE` env var if your size/region needs a different image.

Add a `digitalocean` block to `~/.gml/config.toml`:

```toml
[digitalocean]
api-key = "..."
ssh-key-name = "..."
region = "..."
```

The `ssh-key-name` field is the ID or fingerprint of an SSH key which you have already added to your DigitalOcean account (not the key's display name).

## gmld (the daemon)

`gmld` is a small daemon that enforces timeouts by periodically reading `~/.gml/state.json` and deleting any expired resources (granularity: **1 minute**). Logs are written to `~/.gml/gmld.log`.

`gml node create` will try to auto-start `gmld` if it can find a `gmld` binary **next to** the `gml` executable; you can also run it yourself:

```bash
gmld
```

## Documentation

The user guide is an [mdBook](https://rust-lang.github.io/mdBook/) project under [`docs/`](./docs/). Pushes to `main` run the **Docs** workflow ([`.github/workflows/docs.yml`](./.github/workflows/docs.yml)), which runs `mdbook build docs` and deploys the static output to **GitHub Pages** (set the repository’s Pages source to **GitHub Actions** in Settings).

To preview changes locally, install mdBook (`cargo install mdbook --locked` is fine), then from the repo root:

```bash
mdbook serve docs
```

Open the URL mdBook prints (by default `http://127.0.0.1:3000`); it reloads when you edit the Markdown under `docs/src/`.
