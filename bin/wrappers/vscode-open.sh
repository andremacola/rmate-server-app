#!/bin/bash

# VS Code wrapper script using open -a command
# This script allows rmate-server to use "open -a Visual Studio Code"
# instead of the direct "code" binary (works better with file arguments)

# Check if VS Code is installed
if ! osascript -e 'id of application "Visual Studio Code"' &>/dev/null; then
    echo "Error: VS Code is not installed or not accessible" >&2
    exit 1
fi

# Pass all arguments to VS Code using open -a (most reliable method)
exec open -a "Visual Studio Code" "$@"
