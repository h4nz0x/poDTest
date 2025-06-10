#!/bin/bash
set -e

# Build the poDTest command with arguments
CMD="${{ github.action_path }}/podtest"

if [[ -n "$INPUT_DOCKERFILE_PATH" ]]; then
  CMD="$CMD --dockerfile-path '$INPUT_DOCKERFILE_PATH'"
fi
if [[ -n "$INPUT_PORT" ]]; then
  CMD="$CMD --port $INPUT_PORT"
fi
if [[ -n "$INPUT_HEALTH_CHECK_PATH" ]]; then
  CMD="$CMD --health-check-path '$INPUT_HEALTH_CHECK_PATH'"
fi
if [[ -n "$INPUT_HEALTH_CHECK_TIMEOUT" ]]; then
  CMD="$CMD --health-check-timeout $INPUT_HEALTH_CHECK_TIMEOUT"
fi
if [[ -n "$INPUT_HEALTH_CHECK_INTERVAL" ]]; then
  CMD="$CMD --health-check-interval $INPUT_HEALTH_CHECK_INTERVAL"
fi
if [[ -n "$INPUT_BUILD_TIMEOUT" ]]; then
  CMD="$CMD --build-timeout $INPUT_BUILD_TIMEOUT"
fi
if [[ -n "$INPUT_HOT_FIX" ]]; then
  CMD="$CMD --hot-fix $INPUT_HOT_FIX"
fi

# Execute the command
eval "$CMD"