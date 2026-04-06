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

## Delete a node

```bash
gml node delete <node-id>
```

## Manage node timeouts

```bash
gml node timeout reset --id <node-id> --duration 1h30m
gml node timeout remove --id <node-id>
```
