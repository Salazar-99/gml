# Lambda

The Lambda provider supports **creating and deleting a node** and defaults to the **latest Lambda Stack** image.

Add a `lambda` block to `~/.gml/config.toml`:

```toml
[lambda]
api-key = "..."
ssh-key-name = "..."
region = "..."
```

`ssh-key-name` is the name of an SSH public key already registered in your Lambda account.
