# Building
ARCH := riscv64
ifeq ($(ARCH), x86_64)
  TARGET := x86_64-unknown-none
  QEMU_EXEC += qemu-system-x86_64 \
				-machine q35 \
				-kernel $(KERNEL_ELF) \
				-cpu IvyBridge-v2
  BUS := pci
else ifeq ($(ARCH), riscv64)
  TARGET := riscv64gc-unknown-none-elf
  QEMU_EXEC += qemu-system-$(ARCH) \
				-machine virt \
				-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), aarch64)
  TARGET := aarch64-unknown-none-softfloat
  QEMU_EXEC += qemu-system-$(ARCH) \
				-cpu cortex-a72 \
				-machine virt \
				-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
				-kernel $(KERNEL_BIN)
else ifeq ($(ARCH), loongarch64)
  TARGET := loongarch64-unknown-none
  QEMU_EXEC += qemu-system-$(ARCH) -kernel $(KERNEL_ELF)
  BUS := pci
else
  $(error "ARCH" must be one of "x86_64", "riscv64", "aarch64" or "loongarch64")
endif

MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img
APPS := ../user/src/bin/*

# BOARD
BOARD := qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# Building mode argument
ifeq ($(MODE), release)
	MODE_ARG := --release
endif

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

# Run usertests or usershell
TEST ?=

build: env $(KERNEL_BIN) fs-img 

testbuild: env $(KERNEL_BIN) test-fs-img

env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils
	rustup component add rust-src
	rustup component add llvm-tools-preview

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

fs-img: $(APPS)
	@cd ../user && make build TEST=$(TEST)
	@rm -f $(FS_IMG)
	@dd if=/dev/zero of=$(FS_IMG) bs=1M count=256
	@mkfs.ext4 $(FS_IMG)
	@cd ../ext4-fs-fuse && cargo run -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/
#	@cd ../ext4-fs-fuse && cargo run -- -s ../testcase/basic/riscv64/ -t ../user/target/riscv64gc-unknown-none-elf/release/
#	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

test-fs-img: $(APPS)
	@cd ../user && make build TEST=$(TEST)
	@rm -f $(FS_IMG)
	@dd if=/dev/zero of=$(FS_IMG) bs=1M count=512
	@mkfs.ext4 $(FS_IMG)
#	@cd ../ext4-fs-fuse && cargo run -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/
	@cd ../ext4-test-fuse && cargo run -- -s ../testcase/basic/riscv64/ -t ../user/target/riscv64gc-unknown-none-elf/release/
#	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin/ -t ../user/target/riscv64gc-unknown-none-elf/release/

$(APPS):

#kernel:
#	@echo Platform: $(BOARD)
#	@cp src/linker-$(BOARD).ld src/linker.ld
#	@cargo build --release
#	@rm src/linker.ld
kernel:
	@echo Platform: $(BOARD)
	@cargo build -Z build-std --release --target $(TARGET)

clean:
	@cargo clean

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: run-inner

test: test-run-inner

#QEMU_ARGS := -machine virt \
#			 -nographic \
#			 -bios $(BOOTLOADER) \
#			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
#			 -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
#			 -device virtio-blk-device,drive=x0
QEMU_EXEC += -nographic \
				-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
				-smp 1 \
				-D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors

#QEMU_NAME := qemu-system-riscv64
#qemu-version-check:
#	@sh scripts/qemu-ver-check.sh $(QEMU_NAME)

#run-inner: qemu-version-check build
#	@qemu-system-riscv64 $(QEMU_ARGS)

run-inner: build
	$(QEMU_EXEC)

test-run-inner: testbuild
	$(QEMU_EXEC)

#debug: qemu-version-check build
#	@tmux new-session -d \
		"qemu-system-riscv64 $(QEMU_ARGS) -s -S" && \
		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

#gdbserver: qemu-version-check build
#	@qemu-system-riscv64 $(QEMU_ARGS) -s -S

#gdbclient:
#	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

debug: build
	@tmux new-session -d \
		"$(QEMU_EXEC) -s -S" && \
		tmux split-window -h "gdb -ex 'file $(KERNEL_ELF)' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d


gdbserver: build
	@$(QEMU_EXEC) -s -S

test-gdbserver: testbuild
	@$(QEMU_EXEC) -s -S

gdbclient:
	@riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'

.PHONY: build env kernel clean disasm disasm-vim run-inner fs-img gdbserver gdbclient qemu-version-check test-run-inner testbuild test-fs-img
