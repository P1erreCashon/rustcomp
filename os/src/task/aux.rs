///
pub const AT_NULL: usize = 0;
///
pub const AT_PHDR: usize = 3;
///
pub const AT_PHENT: usize = 4;
///
pub const AT_PHNUM: usize = 5;
///
pub const AT_PAGESZ: usize = 6;
///
pub const AT_RANDOM: usize = 25;


#[derive(Copy, Clone)]
#[repr(C)]
///
pub struct AuxvT{
    /// Type
    pub a_type: usize,
    /// Value
    pub a_val: usize,

}

impl AuxvT {
    ///
    pub fn new(a_type: usize, a_val: usize) -> Self {
        Self { a_type, a_val }
    }
}

