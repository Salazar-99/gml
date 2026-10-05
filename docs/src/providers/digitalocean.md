# DigitalOcean

The DigitalOcean provider supports **creating and deleting a node** (Droplet) and defaults to the **`ubuntu-22-04-x64`** image.

Add a `digitalocean` block to `~/.gml/config.toml`:

```toml
[digitalocean]
api-key = "..."
ssh-key-name = "..."
region = "..."
```

- `api-key` is a DigitalOcean [personal access token](https://docs.digitalocean.com/reference/api/create-personal-access-token/) with write access.
- `ssh-key-name` is the **ID or fingerprint** of an SSH key already registered on your DigitalOcean account (not its display name) — see `doctl compute ssh-key list`.
- `region` is a DigitalOcean region slug, e.g. `nyc3`.

Use `gml node list-types --provider digitalocean` to see available GPU Droplet sizes in your configured region. Set `GML_DIGITALOCEAN_IMAGE` to override the default image if your chosen size/region requires a different one.
