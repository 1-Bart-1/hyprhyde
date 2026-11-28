#!/bin/sh
# Run hyprhyde end to end against a real Hyprland in a QEMU guest; results go to tests/e2e/out.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
docker build -q -t hyprhyde-e2e "$here" >/dev/null
rm -rf "$here/out"
mkdir -p "$here/out"
kvm=""
if [ -w /dev/kvm ]; then kvm="--device /dev/kvm"; fi
# shellcheck disable=SC2086
docker run --rm $kvm -v "$repo:/repo" -v hyprhyde-e2e-cargo:/root/.cargo/registry hyprhyde-e2e \
    sh /repo/tests/e2e/container.sh
cat "$here/out/results.txt"
grep -q '^PASS' "$here/out/results.txt" && ! grep -q '^FAIL' "$here/out/results.txt"
