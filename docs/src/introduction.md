# Introduction

`gml` is a command-line tool for creating, managing, and using GPU/TPU instances across public clouds. 

I built `gml` because I wanted to be able to easily run my own AI research experiments on rented GPUs/TPUs across a variety of public clouds.
There are a few problems with renting accelerated compute that `gml` looks to solve:
- Compute is somewhat scarce so your favorite cloud might not have the type of instance you want at any given time leading you to have to find it somewhere else
- Compute is expensive and the market is dynamic so you want to be able to pick the cheapest option across clouds (or pick whatever you got credits for at a hackathon!)
- Each cloud has its own quirks for setting up an account and provisioning compute that make doing it manually a hassle
- If you are renting compute on-demand than you do not want to keep it running any longer than you need it otherwise you are burning money
- Existing solutions that aggregate cloud access seem too complicated and geared towards enterprises

To solve this `gml` does the following:
- It's a single CLI tool to access any cloud in the providers list (adding a provider is easy as long as they have an API)
- It provides a `--timeout` feature to automatically shutdown your instances after a certain amount of time
- It has a `connect` command to automatically SSH into an instance, copy over your workspace, and open a remote IDE session with git credentials automatically configured
- It all runs locally

Today it supports **single nodes**, **Kubernetes clusters** are a work in progress. The ideal workflow I am building towards is:
- Spin up a single node with `gml node create ...` and connect to it with `gml connect ...` to do quick development and testing
- Spin up a multi-node cluster with `gml cluster create ...` to prepare for running larger scale experiments
- Submit training jobs to the cluster via container images using `gml run ...` leveraging CRDs provided by the `gml-operator` and workload placement decided by the `gml-scheduler` 

This guide covers installing `gml`, configuring providers, and running day-to-day commands.
