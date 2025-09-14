#!/bin/bash
# Format Rust files after Claude Code edits them

jq -r '.tool_input.file_path | select(endswith(".rs"))' | xargs -r rustfmt