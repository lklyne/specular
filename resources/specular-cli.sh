#!/bin/bash
# Resolve the real path of this script (follows symlink chains)
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd -P "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE"
done
DIR="$(cd -P "$(dirname "$SOURCE")" && pwd)"

# Point the CLI at the bundled agent-browser binary when the caller hasn't
# already set one. Covers both the packaged app (Contents/Resources/bin/)
# and dev mode (resources/bin/ after `pnpm fetch:agent-browser`). An
# existing env var always wins — this only fills in the default.
if [ -z "$AGENT_BROWSER_PATH" ] && [ -x "$DIR/bin/agent-browser" ]; then
  export AGENT_BROWSER_PATH="$DIR/bin/agent-browser"
fi

# Packaged app: cli.js is next to this script in Contents/Resources/
# Dev mode: cli.js is in out/main/ relative to the project root
if [ -f "$DIR/cli.js" ]; then
  CLI="$DIR/cli.js"
else
  CLI="$DIR/../out/main/cli.js"
fi
# Use the grandparent PID as the stable session anchor.
# Claude Code spawns a fresh subshell ($PPID) per bash command,
# but its own PID (the grandparent) stays constant across the conversation.
if [ -z "$SPECULAR_PARENT_PID" ]; then
  SPECULAR_PARENT_PID=$(ps -o ppid= -p $PPID 2>/dev/null | tr -d ' ')
  SPECULAR_PARENT_PID="${SPECULAR_PARENT_PID:-$PPID}"
fi
export SPECULAR_PARENT_PID
exec node "$CLI" "$@"
