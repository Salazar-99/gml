# Introduction

`gml` is a command-line tool for creating and managing ephemeral GPU compute. It supports **nodes** today; **clusters** are a work in progress. Resources are tracked locally, and optional automatic shutdown is handled by a small companion daemon, `gmld`.

This guide covers installing `gml`, configuring providers, day-to-day commands, and running the daemon.
