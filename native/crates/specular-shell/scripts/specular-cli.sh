#!/bin/bash
# The `specular` command. A link on the PATH points here, at
# Specular.app/Contents/Resources/specular-cli.sh, and the CLI itself is a
# service of the helper in Contents/MacOS.

# This script's real folder, through however many links lead to it.
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd -P "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE"
done
DIR="$(cd -P "$(dirname "$SOURCE")" && pwd)"

# The browse verbs run the bundled agent-browser unless the caller names one.
if [ -z "$AGENT_BROWSER_PATH" ] && [ -x "$DIR/bin/agent-browser" ]; then
  export AGENT_BROWSER_PATH="$DIR/bin/agent-browser"
fi

# The grandparent is what stays the same across one agent's commands: Claude
# Code starts a new shell ($PPID) for each.
if [ -z "$SPECULAR_PARENT_PID" ]; then
  SPECULAR_PARENT_PID=$(ps -o ppid= -p $PPID 2>/dev/null | tr -d ' ')
  SPECULAR_PARENT_PID="${SPECULAR_PARENT_PID:-$PPID}"
fi
export SPECULAR_PARENT_PID
exec "$DIR/../MacOS/specular-helper" cli "$@"
