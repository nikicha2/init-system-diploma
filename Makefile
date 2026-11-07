
.PHONY: build initramfs run clean

build:
	cargo build --release --target x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/init-system-diploma qemu/rootfs/bin/my-init

initramfs: build
	./qemu/build-initramfs.sh

run: initramfs
	qemu-system-x86_64 \
		-enable-kvm \
		-m 1024 \
		-smp 2 \
		-kernel qemu/vmlinuz \
		-initrd qemu/initramfs.cpio.gz \
		-append "console=ttyS0 rdinit=/init" \
		-nographic

clean:
	cargo clean
	rm -f qemu/rootfs/bin/my-init
	rm -f qemu/initramfs.cpio.gz
