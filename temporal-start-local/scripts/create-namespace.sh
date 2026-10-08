#!/bin/sh
set -eu

: "${TEMPORAL_ADDRESS:?TEMPORAL_ADDRESS is required}"
namespace=${TEMPORAL_NAMESPACE:-default}

# A listening gRPC port does not guarantee that the cluster is ready.
attempt=1
until temporal operator cluster health --address "$TEMPORAL_ADDRESS"; do
  if [ "$attempt" -ge 30 ]; then
    echo "Temporal cluster did not become ready." >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 2
done

attempt=1
while :; do
  if temporal operator namespace describe --address "$TEMPORAL_ADDRESS" \
    --namespace "$namespace" >/dev/null 2>&1; then
    echo "Namespace '$namespace' already exists; keeping its settings."
    exit 0
  fi
  if temporal operator namespace create --address "$TEMPORAL_ADDRESS" \
    --namespace "$namespace" --retention 3d; then
    echo "Namespace '$namespace' created."
    exit 0
  fi
  if [ "$attempt" -ge 30 ]; then
    echo "Unable to create namespace '$namespace'." >&2
    exit 1
  fi
  attempt=$((attempt + 1))
  sleep 2
done
