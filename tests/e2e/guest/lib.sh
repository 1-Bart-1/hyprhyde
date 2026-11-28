# Helpers for scenarios.sh: Hyprland queries, waiting, checks and screenshots.
H=/e/hyprhyde

hc() { timeout 20 hyprctl "$@"; }
dispatch() { echo "dispatch $1 -> $(hc dispatch "$1")"; }

# check <name> <command...>: record whether the command holds.
check() {
    name=$1
    shift
    if "$@"; then result=PASS; else result=FAIL; fi
    echo "$result $name" | tee -a $R/results.txt
}

# wait_for <seconds> <command...>: retry until the command holds or the time is up.
wait_for() {
    tries=$(($1 * 4))
    shift
    for _ in $(seq $tries); do
        "$@" && return 0
        sleep 0.25
    done
    return 1
}
pause() { sleep "$1"; }

shot() { timeout 60 grim ${2:+-o "$2"} "$R/$1.png"; }

workspace_of() { hc -j clients | jq -r --arg a "$1" '.[] | select(.address == $a) | .workspace.name'; }
active() { hc -j activewindow | jq -r '.address // empty'; }
bubbles() { hc -j layers | jq '[.. | objects | select(.namespace? == "hyprhyde")] | length'; }
bubbles_on() {
    hc -j layers | jq --arg m "$1" '[.[$m].levels[][] | select(.namespace == "hyprhyde")] | length'
}
addresses() { hc -j clients | jq -r '.[].address' | sort; }

is() { [ "$1" = "$2" ] || { echo "  expected '$2', got '$1'"; return 1; }; }
on() { [ "$(workspace_of "$1")" = "$2" ]; }
bubble_count() { [ "$(bubbles)" = "$1" ]; }
focused() { [ "$(active)" = "$1" ]; }
changed() { [ "$(addresses)" != "$1" ]; }

# Open a foot window and print its address once Hyprland lists it.
open_window() {
    before=$(addresses)
    foot > /dev/null 2>&1 &
    wait_for 60 changed "$before"
    addresses | grep -vxF "$before" | head -1
}

# Put the pointer at layout position x y: clamp to the top-left corner, then move.
pointer_to() {
    wlrctl pointer move -10000 -10000
    wlrctl pointer move "$1" "$2"
}
click() { wlrctl pointer click left; }
