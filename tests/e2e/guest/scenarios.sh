# The end-to-end scenarios. The corner bubble's centre on the 1280x800 output, bottom-right with a
# 16 px margin, is at x, y below.
X=1242
Y=762

$H daemon > $R/daemon.log 2>&1 &
daemon=$!
pause 5
check "nothing on screen without hidden windows" is "$(bubbles)" 0

# Hide, hover and click on a numbered workspace.
alpha=$(open_window)
wait_for 10 focused "$alpha"
shot 01-window
$H hide
wait_for 20 bubble_count 1
check "hide parks the window in special:hyprhyde-1" is "$(workspace_of "$alpha")" special:hyprhyde-1
check "one bubble after hiding" is "$(bubbles)" 1
shot 02-bubble
pointer_to $X $Y
pause 2
shot 03-hover
click
wait_for 20 on "$alpha" 1
check "a click restores the window to workspace 1" is "$(workspace_of "$alpha")" 1
check "a click focuses the restored window" is "$(active)" "$alpha"
wait_for 5 bubble_count 0
check "the bubble goes once its window is back" is "$(bubbles)" 0
shot 04-restored

# Bubbles stack and belong to their workspace.
beta=$(open_window)
gamma=$(open_window)
$H hide
pause 3
$H hide
wait_for 10 bubble_count 2
check "two hidden windows stack two bubbles" is "$(bubbles)" 2
shot 05-stacked
dispatch "hl.dsp.focus({ workspace = 2 })"
wait_for 10 bubble_count 0
check "another workspace shows none of them" is "$(bubbles)" 0
dispatch "hl.dsp.focus({ workspace = 1 })"
wait_for 10 bubble_count 2
check "they come back with their workspace" is "$(bubbles)" 2

# A restarted daemon picks up what was hidden before; a second one leaves the first running.
kill $daemon
wait $daemon
$H daemon > $R/daemon-restarted.log 2>&1 &
daemon=$!
wait_for 20 bubble_count 2
check "a restarted daemon shows the bubbles left from before" is "$(bubbles)" 2
timeout 30 $H daemon > $R/daemon-second.log 2>&1
check "a second daemon exits at once" is "$?" 0
check "the first daemon keeps running" kill -0 $daemon
check "the bubbles stay after a second daemon" is "$(bubbles)" 2

# Clicking the corner twice without moving restores both (focus warps the pointer by default).
hc eval 'hl.config({ cursor = { no_warps = true } })'
pointer_to $X $Y
click
wait_for 10 bubble_count 1
click
wait_for 10 bubble_count 0
check "two clicks on the corner restore both stacked windows" \
    is "$(workspace_of "$beta") $(workspace_of "$gamma")" "1 1"
hc eval 'hl.config({ cursor = { no_warps = false } })'

# A named workspace.
dispatch "hl.dsp.focus({ workspace = 'name:foo' })"
named=$(open_window)
$H hide
wait_for 10 bubble_count 1
check "a window hidden from workspace foo waits in special:hyprhyde-foo" \
    is "$(workspace_of "$named")" special:hyprhyde-foo
shot 06-named
pointer_to $X $Y
click
wait_for 10 on "$named" foo
check "a click restores it to workspace foo" is "$(workspace_of "$named")" foo
check "and focuses it" is "$(active)" "$named"

# Fullscreen windows.
dispatch "hl.dsp.focus({ workspace = 3 })"
full=$(open_window)
dispatch "hl.dsp.window.fullscreen()"
pause 3
$H hide
wait_for 10 bubble_count 1
check "a fullscreen window hides" is "$(workspace_of "$full")" special:hyprhyde-3
shot 07-fullscreen-hidden
pointer_to $X $Y
click
wait_for 10 on "$full" 3
check "it comes back to workspace 3" is "$(workspace_of "$full")" 3
check "it comes back focused" is "$(active)" "$full"
check "it comes back fullscreen" is "$(hc -j activewindow | jq .fullscreen)" 2
shot 08-fullscreen-restored
other=$(open_window)
wait_for 10 focused "$other"
$H hide
wait_for 10 bubble_count 1
dispatch "hl.dsp.focus({ window = [[address:$full]] })"
[ "$(hc -j activewindow | jq .fullscreen)" = 2 ] || dispatch "hl.dsp.window.fullscreen()"
pause 3
shot 09-bubble-under-fullscreen
pointer_to $X $Y
click
pause 3
check "a fullscreen window covers the bubble" is "$(workspace_of "$other")" special:hyprhyde-3
dispatch "hl.dsp.window.fullscreen()"
pause 3
pointer_to $X $Y
click
wait_for 10 on "$other" 3
check "the bubble restores once nothing covers it" is "$(workspace_of "$other")" 3

# Two outputs: bubbles follow the focused one.
hc output create headless
pause 5
second=$(hc -j monitors | jq -r '.[] | select(.name != "Virtual-1") | .name')
dispatch "hl.dsp.focus({ monitor = '$second' })"
pause 3
open_window > /dev/null
$H hide
wait_for 10 bubble_count 1
check "the bubble shows on the focused second output" is "$(bubbles_on "$second")" 1
shot 10-second-output "$second"
dispatch "hl.dsp.focus({ monitor = 'Virtual-1' })"
wait_for 10 bubble_count 0
check "and leaves it when focus moves to the first" is "$(bubbles_on "$second")" 0
