use config::{MAX_FD,RLimit};
use alloc::vec::Vec;
use alloc::sync::Arc;
use vfs_defs::File;


pub struct FdTable{
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub fd_table_rlimit:RLimit,
}

impl FdTable{
    pub fn new()->Self{
        
    }
}