# PolkaBTC Kubernetes Helm Chart

This Helm Chart can be used to deploy a containerized PolkaBTC Parachain to a Kubernetes cluster.

## Install

To install the chart with the release name `my-release` into namespace `my-namespace` from within this directory:

```bash
helm install --namespace my-namespace --name my-release --values values.yaml ./
```

## Uninstall

To uninstall/delete the `my-release` deployment:

```bash
helm delete --namespace my-namespace my-release
```

## Upgrade

To upgrade the `my-release` deployment:

```bash
helm upgrade --namespace my-namespace --values values.yaml my-release ./
```
