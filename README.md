# coden

An HTTP API for running code within a virtual machine.

## Development Status

- [x] kernel & rootfs
- [x] spawn a Firecracker microVM
- [ ] guest agent to accept & execute commands sent by the host
- [ ] HTTP API to spawn a VM and execute the given code

## How to run?

The easiest way to run this is via Docker:
```
docker run -it --rm \
    --device /dev/kvm \
    --cap-add SYS_ADMIN \
    --security-opt seccomp=unconfined \
    -e RUST_LOG=debug \
    ghcr.io/lukadev-0/coden
```

This repository contains a Nix flake containing a devShell that contains the
required environment variables. After installing Nix, you can run it like this:
```sh
nix develop
nix build .#coden-server
sudo -E RUST_LOG=debug ./result/bin/coden-server
```

You can also acquire the required files manually, refer to
[Firecracker's documentation](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md)
for how to get a copy of the Linux kernel and a rootfs image. Then, set these
environment variables:

- `VMLINUX` - path to the uncompressed Linux kernel
- `ROOTFS` - path to the rootfs image
- `FIRECRACKER` - path to the `firecracker` executable
- `FIRECRACKER_JAILER` - path to the `jailer` executable (comes with Firecracker)

You can download the server binary from the [releases](https://github.com/lukadev-0/coden/releases).

## Future Plans

Integrate this into a website with a code editor to let users write & run code
from the browser.
