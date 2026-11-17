#!/bin/sh
# Mock Antigravity CLI (`agy`) for offline development and automated tests.
#
# Emulates the CLI contract consumed by `CliAiClient` (crates/kamish-core/src/ai/client.rs):
#   agy --version                        -> prints a version line
#   agy -p <prompt> --output-format json -> prints a JSON response with tool calls
#
# This mock performs no network access and mutates no real state.

for arg in "$@"; do
    if [ "$arg" = "--version" ]; then
        echo "Antigravity CLI v1.0.0 (mock)"
        exit 0
    fi
done

# Fixed deterministic response: a single create_alarm tool call for 07:30.
echo '{"response":"Antigravity CLI: alarm set for 07:30.","tool_calls":[{"name":"create_alarm","arguments":{"time":"07:30"}}],"status":"SUCCESS"}'
exit 0
