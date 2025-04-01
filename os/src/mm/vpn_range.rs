use arch::addr::{VirtPage,VirtAddr};

#[derive(Copy, Clone, Debug)]
pub struct VPNRange {
    pub l: VirtPage,
    pub r: VirtPage,
    pub start:VirtAddr,
    pub end:VirtAddr
}
impl VPNRange {
    pub fn new(start: VirtPage, end: VirtPage,start_addr:VirtAddr,end_addr:VirtAddr) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end ,start:start_addr,end:end_addr}
    }
    pub fn get_start(&self) -> VirtPage {
        self.l
    }
    pub fn get_end(&self) -> VirtPage {
        self.r
    }
    pub fn get_start_addr(&self) ->VirtAddr {
        self.start
    }
    pub fn get_end_addr(&self) ->VirtAddr {
        self.end
    }
}
impl IntoIterator for VPNRange {
    type Item = VirtPage;
    type IntoIter = SimpleRangeIterator;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
pub struct SimpleRangeIterator {
    current: VirtPage,
    end: VirtPage,
}
impl SimpleRangeIterator {
    pub fn new(l: VirtPage, r: VirtPage) -> Self {
        Self { current: l, end: r }
    }
}
impl Iterator for SimpleRangeIterator {
    type Item = VirtPage;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current = self.current + 1;
            // let t = self.current;
            // self.current.step();
            Some(t)
        }
    }
}
