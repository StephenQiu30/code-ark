#!/bin/sh
set -eu
# PARAMS is the legacy Spring CLI input; renderer rejects whitespace within each value.
# No eval: values containing quotes, $, # and = stay literal arguments.
set -f
# shellcheck disable=SC2086
exec java ${JAVA_OPTS:-} -jar /app/xxl-job-admin.jar ${PARAMS:-}
