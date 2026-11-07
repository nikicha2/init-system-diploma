# init-system-diploma

## QEMU Testing

### Dependencies

The following tools are required:

- Rust and Cargo
- `x86_64-unknown-linux-musl` Rust target
- musl toolchain
- QEMU (`qemu-system-x86_64`)
- `cpio`
- `gzip`

KVM must be enabled and available on the host to run QEMU with hardware acceleration.

Install the required packages using your distribution's package manager.

For Arch Linux:

```bash
sudo pacman -S qemu-system-x86 cpio gzip musl
```

For Ubuntu/Debian:

```bash
sudo apt install qemu-system-x86 cpio gzip musl-tools
```

Add the required Rust target:

```bash
rustup target add x86_64-unknown-linux-musl
```

### Linux Kernel

The QEMU environment uses the Linux kernel installed on the host.

Copy the kernel image to `qemu/vmlinuz`.

For Arch Linux:

```bash
cp /boot/vmlinuz-linux qemu/vmlinuz
```

For Ubuntu/Debian, check the available kernel images in `/boot/` and copy the appropriate one:

```bash
cp /boot/vmlinuz-<version> qemu/vmlinuz
```

### Build and Run

Build the Rust init process and copy it into the root filesystem:

```bash
make build
```

Build the initramfs:

```bash
make initramfs
```

Run the environment in QEMU:

```bash
make run
```

The `make run` command automatically builds the init process and initramfs before starting QEMU.

To exit QEMU, press `Ctrl+A`, then `X`.

**Note:** KVM must also be accessible to your current user. On some distributions, you may need to add your user to the `kvm` group and log in again for the changes to take effect.