#!/bin/sh
# PID 1 of the guest: mount the basics, run the session from the payload disk, power off.
mount -t proc proc /proc
mount -t sysfs sys /sys
mount -t devtmpfs dev /dev
mkdir -p /dev/pts /dev/shm
mount -t devpts devpts /dev/pts
mount -t tmpfs tmpfs /dev/shm
mount -t tmpfs tmpfs /run
mount -t tmpfs tmpfs /tmp
mkdir -p /e
mount /dev/vdb /e
mkdir -p /e/results
sh /e/session.sh > /e/results/session.log 2>&1
sync
umount /e
poweroff -f
