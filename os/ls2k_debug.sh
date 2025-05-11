#!/bin/bash 

# 通常情况下该文件应当放在项目的根目录下

RUNENV_PREFIX=~/qemu-bin-9.2.1/bin
KERNEL_PREFIX=`pwd`

cd $RUNENV_PREFIX

./qemu-system-loongarch64 \
	-M virt, \
	-kernel /home/pierrecashon/rustcomp/os/target/loongarch64-unknown-none/release/os -m 1G -nographic -smp 1 -drive file=~/testsuits-for-oskernel-pre-20250506/sdcard-la.img,if=none,format=raw,id=x0  \
    -device virtio-blk-pci,drive=x0 \
    -rtc base=utc
#	-S -s
#-drive file=~/rustcomp/user/target/riscv64gc-unknown-none-elf/release/fs.img,if=none,format=raw,id=x0  \
	 #~/testsuits-for-oskernel-pre-20250506/sdcard-la.img
	#-S -s 	 
		#-hdb ~/rcore-tutorial-v3-with-hal-component/user/target/loongarch64-unknown-none/release/fs.img
