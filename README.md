# coden

An HTTP API for running code within a virtual machine.

## Development Status

- [x] kernel & rootfs
- [x] spawn a Firecracker microVM
- [x] guest agent to accept & execute commands sent by the host
- [ ] HTTP API to spawn a VM and execute the given code

## How to run?

The easiest way to run this is via Docker:
```
docker run -it --rm \
    --device /dev/kvm \
    --cap-add SYS_ADMIN \
    --security-opt seccomp=unconfined \
    -p 8080:8080 \
    -e RUST_LOG=debug \
    ghcr.io/lukadev-0/coden
```

This repository contains a Nix flake that exposes a package that includes all
the required dependencies.
```sh
sudo RUST_LOG=debug nix run github:lukadev-0/coden
```

You can also acquire the required files manually, refer to
[Firecracker's documentation](https://github.com/firecracker-microvm/firecracker/blob/main/docs/getting-started.md)
for how to download Firecracker and get a copy of the Linux kernel. Head to the
[releases](https://github.com/lukadev-0/coden/releases) to download the
`coden-server` binary and the `rootfs`.

```sh
sudo VMLINUX=<vmlinux> ROOTFS=<rootfs> FIRECRACKER=<firecracker> FIRECRACKER_JAILER=<jailer> \
    ./coden-server
```

- `<vmlinux>` is the path to the uncompressed Linux kernel
- `<rootfs>` is the path to the rootfs image
- `<firecracker>` is the path to the `firecracker` executable
- `<jailer>` is the path to the `jailer` executable (comes with Firecracker)

## Future Plans

Integrate this into a website with a code editor to let users write & run code
from the browser.
