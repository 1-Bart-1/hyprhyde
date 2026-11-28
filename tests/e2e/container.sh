# Inside the hyprhyde-e2e image: build hyprhyde, boot a guest from this image's own files, and
# copy what the guest wrote to tests/e2e/out.
set -eu
cd /repo
cargo build --locked
out=/repo/tests/e2e/out

mkinitcpio -k "$(ls /usr/lib/modules)" -S autodetect -g /tmp/initramfs.img >/dev/null 2>&1
mkdir /tmp/root
tar -C / --one-file-system --exclude=./tmp -cf - . | tar -C /tmp/root -xf -
mkdir -p /tmp/root/proc /tmp/root/sys /tmp/root/dev /tmp/root/run /tmp/root/tmp
install -m755 tests/e2e/guest/init.sh /tmp/root/init.sh
echo 0123456789abcdef0123456789abcdef > /tmp/root/etc/machine-id
mke2fs -q -t ext4 -d /tmp/root /tmp/root.img 8G

mkdir /tmp/payload
cp tests/e2e/guest/* target/debug/hyprhyde /tmp/payload/
mke2fs -q -t ext4 -d /tmp/payload /tmp/payload.img 512M

accel=tcg,thread=multi
if [ -e /dev/kvm ]; then accel=kvm; fi
# The virtio mouse keeps a pointer on the seat; without one, wlrctl's clicks reach no client.
timeout 1200 qemu-system-x86_64 -machine q35 -accel "$accel" -cpu max -smp 8 -m 6G \
    -kernel /boot/vmlinuz-linux -initrd /tmp/initramfs.img \
    -append "root=/dev/vda rw console=ttyS0 init=/init.sh quiet" \
    -drive file=/tmp/root.img,if=virtio,format=raw \
    -drive file=/tmp/payload.img,if=virtio,format=raw \
    -device virtio-mouse-pci -nographic -no-reboot > "$out/serial.log" 2>&1

mkdir /tmp/results
debugfs -R "rdump /results /tmp/results" /tmp/payload.img >/dev/null 2>&1
cp -r /tmp/results/results/. "$out/"
