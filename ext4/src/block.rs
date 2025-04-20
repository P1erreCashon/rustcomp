use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use buffer::{get_block_cache,DataBlock};
use config::{BLOCK_SZ,DISK_BLOCK_SZ};

pub struct Ext4Disk{
    dev:Arc<dyn device::BlockDevice>
}

impl Ext4Disk{
    pub fn new(dev:Arc<dyn device::BlockDevice>)->Self{
        Self{
            dev
        }
    }
}

impl ext4_rs::BlockDevice for Ext4Disk{
    fn read_offset(&self, offset: usize) -> Vec<u8> {
        let mut dst = vec![0u8; BLOCK_SZ];
        let start_block_id = offset / DISK_BLOCK_SZ;
        let mut offset_in_block = offset % DISK_BLOCK_SZ;
        let mut total_bytes_read = 0;
        while total_bytes_read < dst.len() {
            let current_block_id = start_block_id + (total_bytes_read / DISK_BLOCK_SZ);
            let bytes_to_copy =
            (dst.len() - total_bytes_read).min(DISK_BLOCK_SZ - offset_in_block);

            get_block_cache(current_block_id, self.dev.clone())
            .lock()
            .read(0, |data_block: &DataBlock|{
                dst[total_bytes_read..total_bytes_read + bytes_to_copy]
                .copy_from_slice(&data_block[offset_in_block..offset_in_block + bytes_to_copy]);
            });
            total_bytes_read += bytes_to_copy;
            offset_in_block = 0; 
        }
        dst
    }
    fn write_offset(&self, offset: usize, data: &[u8]) {
        let start_block_id = offset / DISK_BLOCK_SZ;
        let mut offset_in_block = offset % DISK_BLOCK_SZ;

        let bytes_to_write = data.len();
        let mut total_bytes_written = 0;

        while total_bytes_written < bytes_to_write {
            let current_block_id = start_block_id + (total_bytes_written / DISK_BLOCK_SZ);
            let bytes_to_copy =
                (bytes_to_write - total_bytes_written).min(DISK_BLOCK_SZ - offset_in_block);
            get_block_cache(current_block_id, self.dev.clone())
            .lock()
            .modify(0, |data_block: &mut DataBlock|{
                data_block[offset_in_block..offset_in_block + bytes_to_copy]
                .copy_from_slice(&data[total_bytes_written..total_bytes_written + bytes_to_copy]);
            });
            total_bytes_written += bytes_to_copy;
            offset_in_block = 0; // After the first block, subsequent blocks start at the beginning
        }
    }
}