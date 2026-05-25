# Quickstart

## Install

**macOS (Homebrew)**

```bash
brew tap Salazar-99/gml
brew install gml
```

**Linux (shell installer)**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Salazar-99/gml/releases/latest/download/gml-installer.sh | sh
```

Both install the `gml` CLI and the `gmld` background daemon.

## Configure a provider

`gml` needs at least one cloud provider configured before you can provision nodes. Follow the setup guide for the provider you want to use:

- [Lambda Labs](providers/lambda.md)
- [Google Cloud](providers/google.md)

## Verify

```bash
gml --help
```
