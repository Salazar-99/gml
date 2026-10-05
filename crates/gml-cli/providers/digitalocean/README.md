# DigitalOcean gml Provider

Supports creating and deleting a node (Droplet). Defaults to the `ubuntu-22-04-x64`
image; override with the `GML_DIGITALOCEAN_IMAGE` env var if your size/region needs
a different image.

Requires an SSH key already added to your DigitalOcean account; `ssh-key-name` in
config takes that key's ID or fingerprint (as shown by `doctl compute ssh-key list`
or the API), not the key's name.
