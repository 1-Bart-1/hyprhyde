# Start Hyprland on the guest's display, then run the scenarios against it.
R=/e/results
/usr/lib/systemd/systemd-udevd --daemon
udevadm trigger
udevadm settle
seatd -g root > $R/seatd.log 2>&1 &
export LIBSEAT_BACKEND=seatd XDG_RUNTIME_DIR=/run/user/0 HOME=/root
mkdir -p -m700 $XDG_RUNTIME_DIR /root/.config/gtk-4.0
printf '[Settings]\ngtk-icon-theme-name=Papirus-Dark\n' > /root/.config/gtk-4.0/settings.ini
DBUS_SESSION_BUS_ADDRESS=$(dbus-daemon --session --fork --print-address)
export DBUS_SESSION_BUS_ADDRESS
Hyprland --i-am-really-stupid -c /e/hyprland.lua > $R/hyprland-stdout.log 2>&1 &
for _ in $(seq 240); do
    timeout 5 hyprctl -i 0 version > /dev/null 2>&1 && break
    sleep 0.5
done
HYPRLAND_INSTANCE_SIGNATURE=$(ls $XDG_RUNTIME_DIR/hypr)
export HYPRLAND_INSTANCE_SIGNATURE WAYLAND_DISPLAY=wayland-1
. /e/lib.sh
. /e/scenarios.sh
cp $XDG_RUNTIME_DIR/hypr/*/hyprland.log $R/
