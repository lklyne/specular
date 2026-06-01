#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCENARIOS_DIR="$SCRIPT_DIR/scenarios"
RESULTS_DIR="$SCRIPT_DIR/results"
SYSTEM_PROMPT="$SCRIPT_DIR/SYSTEM_PROMPT.md"
CDP_PORT=9333
SMOKE_PORT=29950
SANDBOX_DIR=""
ELECTRON_PID=""
KEEP_OPEN=false

# Max spend per scenario as a guardrail
MAX_BUDGET_PER_SCENARIO=1.00
MODEL="${AGENT_TEST_MODEL:-sonnet}"

# Parse flags
FILTER=""
for arg in "$@"; do
  case "$arg" in
    --review) KEEP_OPEN=true ;;
    *) FILTER="$arg" ;;
  esac
done

cleanup() {
  echo ""
  if [[ "$KEEP_OPEN" == true ]]; then
    echo "App left open for review (PID $ELECTRON_PID)."
    echo "Kill manually: kill $ELECTRON_PID"
    return
  fi
  echo "Cleaning up..."
  if [[ -n "$ELECTRON_PID" ]] && kill -0 "$ELECTRON_PID" 2>/dev/null; then
    kill "$ELECTRON_PID" 2>/dev/null || true
    sleep 1
    kill -9 "$ELECTRON_PID" 2>/dev/null || true
  fi
  if [[ -n "$SANDBOX_DIR" && -d "$SANDBOX_DIR" ]]; then
    rm -rf "$SANDBOX_DIR"
  fi
  rm -f "$DISCOVERY_FILE" 2>/dev/null || true
  agent-browser close 2>/dev/null || true
}
trap cleanup EXIT

# Private discovery file (macOS tmpdir is not always /tmp) so this run never
# reads or clobbers the canonical specular-mcp.json a dev app + CLI share.
DISCOVERY_FILE="$(python3 -c "import tempfile, os; print(os.path.join(tempfile.gettempdir(), 'specular-mcp-agent-$SMOKE_PORT.json'))")"
export SPECULAR_DISCOVERY_FILE="$DISCOVERY_FILE"

# Read secret once (populated after server is ready)
API_SECRET=""
load_secret() {
  if [[ -z "$API_SECRET" ]]; then
    API_SECRET=$(python3 -c "import json; print(json.load(open('$DISCOVERY_FILE'))['secret'])")
  fi
}

# Helper: call the app control server
api() {
  local method="$1" path="$2"
  shift 2
  load_secret
  local secret="$API_SECRET"
  if [[ "$method" == "GET" ]]; then
    curl -s "http://127.0.0.1:$SMOKE_PORT$path" \
      -H "X-Specular-Secret: $secret"
  else
    curl -s "http://127.0.0.1:$SMOKE_PORT$path" \
      -X "$method" \
      -H "Content-Type: application/json" \
      -H "X-Specular-Secret: $secret" \
      "$@"
  fi
}

# --- Build ---
echo "Building app..."
cd "$PROJECT_DIR"
npm run build --silent 2>/dev/null

# --- Launch Electron ---
SANDBOX_DIR=$(mktemp -d -t specular-agent-test)
echo "Launching Electron (CDP port $CDP_PORT, sandbox $SANDBOX_DIR)..."

SPECULAR_PORT=$SMOKE_PORT \
  npx electron ./out/main/index.js \
  --remote-debugging-port=$CDP_PORT \
  "--user-data-dir=$SANDBOX_DIR" \
  &>/dev/null &
ELECTRON_PID=$!

# Wait for app control server
echo -n "Waiting for app..."
for i in $(seq 1 30); do
  if curl -s "http://127.0.0.1:$SMOKE_PORT/health" >/dev/null 2>&1; then
    echo " ready."
    break
  fi
  if [[ $i -eq 30 ]]; then
    echo " TIMEOUT"
    exit 1
  fi
  sleep 0.5
  echo -n "."
done

# --- Clear default frames so we start with a clean canvas ---
if [[ "$KEEP_OPEN" == true ]]; then
  frame_ids=$(api GET /workspace | python3 -c "
import sys, json
data = json.load(sys.stdin)
ids = [f['id'] for f in data.get('frames', [])]
print(json.dumps(ids))
" 2>/dev/null)
  if [[ "$frame_ids" != "[]" ]]; then
    api POST /frames/delete -d "{\"frameIds\": $frame_ids}" >/dev/null
    sleep 0.5
  fi
fi

# --- Prepare results ---
rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR"

# --- Run scenarios ---
PASS=0
FAIL=0
TOTAL=0
declare -a SCENARIO_NAMES=()
declare -a SCENARIO_RESULTS=()

for scenario in "$SCENARIOS_DIR"/*.md; do
  name=$(basename "$scenario" .md)

  # Skip if filter is set and doesn't match
  if [[ -n "$FILTER" && "$name" != "$FILTER" ]]; then
    continue
  fi

  TOTAL=$((TOTAL + 1))
  result_dir="$RESULTS_DIR/$name"
  mkdir -p "$result_dir"

  echo ""
  echo "━━━ Running: $name ━━━"

  # Extract timeout from frontmatter (default 120s)
  scenario_timeout=$(sed -n 's/^timeout: *\([0-9]*\)s*/\1/p' "$scenario" 2>/dev/null)
  scenario_timeout=${scenario_timeout:-120}

  # Build the prompt
  prompt="$(cat "$SYSTEM_PROMPT")

---

## Scenario to execute

$(cat "$scenario")

---

## Instructions

- The app is running on CDP port $CDP_PORT.
- The app control server is on http://127.0.0.1:$SMOKE_PORT (read $DISCOVERY_FILE for the secret).
- Save screenshots to: $result_dir/
- Write your result report to: $result_dir/result.md
- You have ${scenario_timeout}s to complete this scenario."

  # Run Claude with the scenario
  if claude --print --dangerously-skip-permissions --no-session-persistence --model "$MODEL" --max-budget-usd "$MAX_BUDGET_PER_SCENARIO" -p "$prompt" 2>/dev/null; then
    # Check if result file reports PASS
    if grep -qi "PASS" "$result_dir/result.md" 2>/dev/null; then
      echo "  ✓ PASS"
      PASS=$((PASS + 1))
      SCENARIO_NAMES+=("$name")
      SCENARIO_RESULTS+=("PASS")
    else
      echo "  ✗ FAIL"
      FAIL=$((FAIL + 1))
      SCENARIO_NAMES+=("$name")
      SCENARIO_RESULTS+=("FAIL")
    fi
  else
    echo "  ✗ FAIL (timeout or error)"
    FAIL=$((FAIL + 1))
    echo "## Result: FAIL

### Notes
Scenario timed out or Claude exited with an error." > "$result_dir/result.md"
    SCENARIO_NAMES+=("$name")
    SCENARIO_RESULTS+=("FAIL")
  fi
done

# --- Summary ---
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Results: $PASS/$TOTAL passed"
if [[ $FAIL -gt 0 ]]; then
  echo "  $FAIL failed — see tests/agent/results/ for details"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Write summary file
cat > "$RESULTS_DIR/summary.md" <<EOF
# Agent Test Results

**Run:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Passed:** $PASS / $TOTAL

## Scenarios
EOF

for scenario in "$SCENARIOS_DIR"/*.md; do
  sname=$(basename "$scenario" .md)
  if [[ -n "$FILTER" && "$sname" != "$FILTER" ]]; then
    continue
  fi
  if grep -qi "PASS" "$RESULTS_DIR/$sname/result.md" 2>/dev/null; then
    echo "- ✓ $sname" >> "$RESULTS_DIR/summary.md"
  else
    echo "- ✗ $sname" >> "$RESULTS_DIR/summary.md"
  fi
done

# --- Build review canvas ---
if [[ "$KEEP_OPEN" == true && "$TOTAL" -gt 0 ]]; then
  echo ""
  echo "Building review canvas..."

  # Clear any leftover entities from test runs
  frame_ids=$(api GET /workspace | python3 -c "
import sys, json
data = json.load(sys.stdin)
ids = [f['id'] for f in data.get('frames', [])]
print(json.dumps(ids))
" 2>/dev/null)
  if [[ "$frame_ids" != "[]" ]]; then
    api POST /frames/delete -d "{\"frameIds\": $frame_ids}" >/dev/null 2>&1
    sleep 0.3
  fi

  text_ids=$(api GET /text-entities | python3 -c "
import sys, json
data = json.load(sys.stdin)
ids = [e['id'] for e in data.get('textEntities', [])]
print(json.dumps(ids))
" 2>/dev/null)
  if [[ "$text_ids" != "[]" ]]; then
    api POST /text-entities/delete -d "{\"ids\": $text_ids}" >/dev/null 2>&1
    sleep 0.3
  fi

  # Add title
  api POST /text-entities/create -d "{
    \"canvasX\": 40, \"canvasY\": 20,
    \"text\": \"Agent Test Results — $PASS/$TOTAL passed — $(date '+%Y-%m-%d %H:%M')\",
    \"width\": 800, \"height\": 60
  }" >/dev/null

  # Layout: each scenario is a row
  #   Left: text entity with name + result + summary (240px wide)
  #   Right: screenshots laid out left-to-right
  #   Vertical gap between rows
  INFO_WIDTH=260
  SCREENSHOT_W=400
  SCREENSHOT_H=260
  SCREENSHOT_GAP=20
  ROW_GAP=40
  y=100

  for i in $(seq 0 $((TOTAL - 1))); do
    name="${SCENARIO_NAMES[$i]}"
    result="${SCENARIO_RESULTS[$i]}"
    result_dir="$RESULTS_DIR/$name"

    # Scenario info on the left
    if [[ "$result" == "PASS" ]]; then
      color="green"
      label="✓ PASS"
    else
      color="red"
      label="✗ FAIL"
    fi

    summary=""
    if [[ -f "$result_dir/result.md" ]]; then
      summary=$(sed -n '/^###\|^##/!p' "$result_dir/result.md" | head -8 | sed 's/"/\\"/g' | tr '\n' ' ' | cut -c1-200)
    fi

    api POST /text-entities/create -d "{
      \"canvasX\": 40, \"canvasY\": $y,
      \"text\": \"$label: $name\n\n$summary\",
      \"color\": \"$color\",
      \"width\": $INFO_WIDTH, \"height\": 200
    }" >/dev/null

    # Screenshots laid out left-to-right, preserving aspect ratio
    # Each screenshot gets a small label annotation above it
    LABEL_H=30
    sx=$((40 + INFO_WIDTH + SCREENSHOT_GAP))
    max_img_h=0
    for screenshot in "$result_dir"/*.png; do
      if [[ -f "$screenshot" ]]; then
        # Read image dimensions and compute height to preserve aspect ratio
        img_h=$(python3 -c "
from struct import unpack
with open('$screenshot','rb') as f:
  f.read(16); w,h = unpack('>II', f.read(8))
print(int($SCREENSHOT_W * h / w))
" 2>/dev/null)
        img_h=${img_h:-$SCREENSHOT_H}
        if [[ $img_h -gt $max_img_h ]]; then max_img_h=$img_h; fi

        # Label above screenshot (filename without extension as caption)
        caption=$(basename "$screenshot" .png | sed 's/-/ /g; s/_/ /g')
        api POST /text-entities/create -d "{
          \"canvasX\": $sx, \"canvasY\": $y,
          \"text\": \"$caption\",
          \"width\": $SCREENSHOT_W, \"height\": $LABEL_H
        }" >/dev/null

        # Screenshot below label
        api POST /file-entities/create -d "{
          \"canvasX\": $sx, \"canvasY\": $((y + LABEL_H)),
          \"file\": \"$screenshot\",
          \"width\": $SCREENSHOT_W, \"height\": $img_h
        }" >/dev/null
        sx=$((sx + SCREENSHOT_W + SCREENSHOT_GAP))
      fi
    done

    # Use the tallest image height for the row
    row_h=$((max_img_h + LABEL_H))
    row_h=${row_h:-$SCREENSHOT_H}
    y=$((y + row_h + ROW_GAP))
  done

  # Focus camera on the results
  api POST /camera/focus -d "{
    \"bounds\": {\"x\": 0, \"y\": 0, \"width\": $((40 + INFO_WIDTH + 5 * (SCREENSHOT_W + SCREENSHOT_GAP))), \"height\": $y}
  }" >/dev/null

  echo "Review canvas ready. Check the Specular window."
fi

if [[ "$KEEP_OPEN" != true ]]; then
  exit $FAIL
fi
