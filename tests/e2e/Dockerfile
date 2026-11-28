# Arch with Rust, GTK, Hyprland and a kernel for the QEMU guest the end-to-end run boots.
FROM archlinux:latest
RUN pacman -Syu --noconfirm --needed base-devel git rust gtk4 gtk4-layer-shell hyprland grim foot \
        jq meson scdoc wayland-protocols papirus-icon-theme ttf-dejavu linux mkinitcpio seatd \
        qemu-system-x86 e2fsprogs \
    && git clone https://git.sr.ht/~brocellous/wlrctl /tmp/wlrctl \
    && git -C /tmp/wlrctl checkout c6bc60820bb8786c7509e651bcae9393a738f180 \
    && meson setup /tmp/wlrctl/build /tmp/wlrctl --prefix=/usr \
    && ninja -C /tmp/wlrctl/build install \
    && rm -rf /tmp/wlrctl \
    && pacman -Scc --noconfirm
