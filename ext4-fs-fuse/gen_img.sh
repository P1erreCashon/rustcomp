cd ../user/target/riscv64gc-unknown-none-elf/release/
rm -rf fs.img
dd if=/dev/zero of=fs.img bs=1M count=8192
mkfs.ext4 ./fs.img