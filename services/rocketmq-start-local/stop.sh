#!/bin/sh
# RocketMQ 停止脚本
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}"

echo "---------------------------------------"
echo "  RocketMQ 停止中..."
echo "---------------------------------------"
docker compose stop

echo "---------------------------------------"
echo "  RocketMQ 已停止"
echo "---------------------------------------"
