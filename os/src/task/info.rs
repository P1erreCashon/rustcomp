use lazy_static::lazy_static;

#[derive(Clone, Copy)]
#[repr(C)]
/// System information structure.
pub struct SysInfo {
    /// Seconds since boot.
    pub uptime: i64,
    /// 1, 5, and 15 minute load averages.
    pub loads: [u64; 3],
    /// Total usable main memory size.
    pub totalram: u64,
    /// Available memory size.
    pub freeram: u64,
    /// Amount of shared memory.
    pub sharedram: u64,
    /// Memory used by buffers.
    pub bufferram: u64,
    /// Total swap space size.
    pub totalswap: u64,
    /// Swap space still available.
    pub freeswap: u64,
    /// Number of current processes.
    pub procs: u16,
    /// Explicit padding for m68k.
    pub pad: u16,
    /// Total high memory size.
    pub totalhigh: u64,
    /// Available high memory size.
    pub freehigh: u64,
    /// Memory unit size in bytes.
    pub mem_uint: u32,
    /// Pads structure to 64 bytes.
    pub _f: [u8; 12], // 假设 _F_SIZE 是 12，使得结构体总大小为 64 字节, o错误？？
}

impl Default for SysInfo {
    fn default() -> Self {
        SysInfo {
            uptime: 0,
            loads: [0; 3],
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            pad: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_uint: 0,
            _f: [0; 12],
        }
    }
}

lazy_static!{
    pub static ref UNAME: Utsname = Utsname::default();
}

///
#[repr(C)]
pub struct Utsname {
    ///
    pub sysname: [u8; 65],
    ///
    pub nodename: [u8; 65],
    ///
    pub release: [u8; 65],
    ///
    pub version: [u8; 65],
    ///
    pub machine: [u8; 65],
    ///
    pub domainname: [u8; 65],
}

// Helper function to convert a string to a fixed-size array of u8
fn string_to_array(s: &str) -> [u8; 65] {
    let mut array = [0u8; 65];
    let bytes = s.as_bytes();
    let len = bytes.len().min(64); // Ensure we don't overflow the array
    array[..len].copy_from_slice(&bytes[..len]);
    array
}

impl Default for Utsname {
    fn default() -> Self {
        Utsname {
            sysname: string_to_array("Linux"),
            nodename: string_to_array("Linux"),
            release: string_to_array("5.19.0-42-generic"),
            version: string_to_array("#43~22.04.1-Ubuntu SMP PREEMPT_DYNAMIC Fri Apr 21 16:51:08 UTC 2"),
            machine: string_to_array("risc-v"),
            domainname: string_to_array("user"),
        }
    }
}
impl Utsname {
    /// Copy the contents of another Utsname instance into this instance
    pub fn copy_from(&mut self, other: &Utsname) {
        self.sysname.copy_from_slice(&other.sysname);
        self.nodename.copy_from_slice(&other.nodename);
        self.release.copy_from_slice(&other.release);
        self.version.copy_from_slice(&other.version);
        self.machine.copy_from_slice(&other.machine);
        self.domainname.copy_from_slice(&other.domainname);
    }
}