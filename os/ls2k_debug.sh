#!/bin/bash 

# 通常情况下该文件应当放在项目的根目录下

RUNENV_PREFIX=/home/pierrecashon/qemu
KERNEL_PREFIX=`pwd`

cd $RUNENV_PREFIX

./bin/qemu-system-loongarch64 \
	-M ls2k \
	-serial stdio \
	-k ./share/qemu/keymaps/en-us \
	-kernel /home/pierrecashon/rustcomp/os/target/loongarch64-unknown-none/release/os\
	-serial vc \
	-m 1G \
	-vnc :0 \
	-drive file=~/rustcomp/user/target/riscv64gc-unknown-none-elf/release/fs.img,if=none,format=raw,id=x0 \
	-device virtio-blk-pci,drive=x0,bus=virtio-mmio-bus.0 -no-reboot \
	-netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
	 
	#-S -s 	 
		#-hdb ~/rcore-tutorial-v3-with-hal-component/user/target/loongarch64-unknown-none/release/fs.img
