#!/bin/zsh
# The simulator test loop, one xcodebuild each:
#   scripts/test.sh        fast: DrusskyTests + LearningCoreTests (the ordinary loop)
#   scripts/test.sh slow   DrusskySlowTests (Vision over hundreds of ink samples; when ink or
#                          recognition code changes)
#   scripts/test.sh all    the three unit-test targets (DrusskyUITests stay a separate run)
# DRUSSKY_SIM names the simulator (default "Drussky iPad 13"); it is found by name, then addressed
# by udid, since `name=` alone resolves to the newest runtime and misses a device on an older one.
# The full log is .build/logs/test-<mode>.log; the summary and any failures are printed.
set -u
cd "$(dirname "$0")/.." || exit 1
mode="${1:-fast}"
case "$mode" in
  fast) only=(-only-testing:DrusskyTests -only-testing:LearningCoreTests) ;;
  slow) only=(-only-testing:DrusskySlowTests) ;;
  all) only=(-only-testing:DrusskyTests -only-testing:LearningCoreTests -only-testing:DrusskySlowTests) ;;
  *) echo "usage: $0 [fast|slow|all]" >&2; exit 2 ;;
esac
sim="${DRUSSKY_SIM:-Drussky iPad 13}"
udid=$(xcrun simctl list devices available -j | python3 -c '
import json, sys
name = sys.argv[1]
found = [d for devices in json.load(sys.stdin)["devices"].values() for d in devices if d["name"] == name]
if not found: sys.exit("no available simulator named %r (xcrun simctl list devices)" % name)
print(found[0]["udid"])' "$sim") || exit 1
mkdir -p .build/logs
log=".build/logs/test-$mode.log"
start=$(date +%s)
# -collect-test-diagnostics never: otherwise a run can end with `simctl diagnose`, ten minutes of
# log collection, after the tests have finished.
xcodebuild -project Drussky.xcodeproj -scheme Drussky \
  -destination "platform=iOS Simulator,id=$udid" -derivedDataPath .build/dd \
  -collect-test-diagnostics never "${only[@]}" test > "$log" 2>&1
rc=$?
wall=$(( $(date +%s) - start ))
grep -E '^\s*Executed [0-9]+ tests' "$log" | tail -1
if [ "$rc" -ne 0 ]; then
  grep -E 'error:|Test Case .* failed|\*\* TEST FAILED' "$log" | head -30
fi
echo "test.sh $mode: exit $rc, ${wall}s wall (build + test), log $log"
exit "$rc"
