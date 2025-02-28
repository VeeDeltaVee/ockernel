//! x86 non-PAE paging
// warning: this code is terrible. do not do anything like this

use core::{
    arch::asm,
    default::Default,
    fmt,
    mem::size_of,
};
use bitmask_enum::bitmask;
use crate::{
    util::array::BitSet,
    mm::KHEAP_INITIAL_SIZE,
};
use super::{MEM_SIZE, LINKED_BASE, KHEAP_START, PAGE_SIZE};

extern "C" {
    /// located at end of kernel, used for calculating placement address
    static kernel_end: u32;
}

/// entry in a page table
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PageTableEntry(u32);

impl PageTableEntry {
    /// create new page table entry
    pub fn new(addr: u32, flags: PageTableFlags) -> Self {
        Self((addr & 0xfffff000) | (flags.0 & 0x0fff) as u32)
    }

    /// create an unused page table entry
    pub fn new_unused() -> Self {
        Self(0)
    }

    /// set address of page table entry
    pub fn set_address(&mut self, addr: u32) {
        self.0 = (self.0 & 0x00000fff) | (addr & 0xfffff000);
    }

    /// set flags of page table entry
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.0 = (self.0 & 0xfffff000) | (flags.0 & 0x0fff) as u32;
    }

    /// checks if this page table entry is unused
    pub fn is_unused(&self) -> bool {
        self.0 == 0 // lol. lmao
    }

    /// set page as unused and clear its fields
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    /// gets address of page table entry
    pub fn get_address(&self) -> u32 {
        self.0 & 0xfffff000
    }

    /// gets flags of page table entry
    pub fn get_flags(&self) -> u16 {
        (self.0 & 0x00000fff) as u16
    }
}

impl fmt::Display for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "PageTableEntry {{")?;
        writeln!(f, "    address: {:#x},", self.0 & 0xfffff000)?;
        writeln!(f, "    flags: {}", PageTableFlags((self.0 & 0x0fff) as u16))?;
        write!(f, "}}")
    }
}

/// page table entry flags
#[bitmask(u16)]
#[repr(transparent)]
pub enum PageTableFlags {
    /// no flags?
    None                = Self(0),

    /// page is present in memory and can be accessed
    Present             = Self(1 << 0),

    /// code can read and write to page
    /// absence of this flag forces read only
    ReadWrite           = Self(1 << 1),

    /// page is accessible in user mode
    /// absence of this flag only allows supervisor access
    UserSupervisor      = Self(1 << 2),

    /// enables write-through caching instead of write-back
    /// requires page attribute table
    PageWriteThru       = Self(1 << 3),

    /// disables caching for this page
    /// requires page attribute table
    PageCacheDisable    = Self(1 << 4),

    /// set if page has been accessed during address translation
    Accessed            = Self(1 << 5),

    /// set if page has been written to
    Dirty               = Self(1 << 6),

    /// can be set if page attribute table is supported, allows setting cache disable and write thru bits
    PageAttributeTable  = Self(1 << 7),

    /// tells cpu to not invalidate this page table entry in cache when page tables are reloaded
    Global              = Self(1 << 8),

    /// if this bit is set and the present bit is not, the page will be copied into a new page when written to
    CopyOnWrite         = Self(1 << 9),
}

impl fmt::Display for PageTableFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageTableFlags {{")?;

        if self.0 & (1 << 0) > 0 {
            write!(f, " present,")?;
        }

        if self.0 & (1 << 1) > 0 {
            write!(f, " read/write")?;
        } else {
            write!(f, " read only")?;
        }

        if self.0 & (1 << 2) > 0 {
            write!(f, ", user + supervisor mode")?;
        } else {
            write!(f, ", supervisor mode")?;
        }

        if self.0 & (1 << 3) > 0 {
            write!(f, ", write thru")?;
        }

        if self.0 & (1 << 4) > 0 {
            write!(f, ", cache disable")?;
        }

        if self.0 & (1 << 5) > 0 {
            write!(f, ", accessed")?;
        }

        if self.0 & (1 << 6) > 0 {
            write!(f, ", dirty")?;
        }

        if self.0 & (1 << 7) > 0 {
            write!(f, ", page attribute table")?;
        }

        if self.0 & (1 << 8) > 0 {
            write!(f, ", global")?;
        }

        if self.0 & (1 << 9) > 0 {
            write!(f, ", copy on write")?;
        }

        write!(f, " }}")
    }
}

/// entry in a page directory
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PageDirEntry(u32);

impl PageDirEntry {
    /// create new page directory entry
    pub fn new(addr: u32, flags: PageTableFlags) -> Self {
        Self((addr & 0xfffff000) | (flags.0 & 0x0fff) as u32)
    }

    /// create an unused page directory entry
    pub fn new_unused() -> Self {
        Self(0)
    }

    /// set address of page directory entry
    pub fn set_address(&mut self, addr: u32) {
        self.0 = (self.0 & 0x00000fff) | (addr & 0xfffff000);
    }

    /// set flags of page directory entry
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.0 = (self.0 & 0xfffff000) | (flags.0 & 0x0fff) as u32;
    }

    /// checks if this page dir entry is unused
    pub fn is_unused(&self) -> bool {
        self.0 == 0 // lol. lmao
    }

    /// set page dir as unused and clear its fields
    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    /// gets address of page directory entry
    pub fn get_address(&self) -> u32 {
        self.0 & 0xfffff000
    }

    /// gets flags of page directory entry
    pub fn get_flags(&self) -> u16 {
        (self.0 & 0x00000fff) as u16
    }
}

impl fmt::Display for PageDirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "PageDirEntry {{")?;
        writeln!(f, "    address: {:#x},", self.0 & 0xfffff000)?;
        writeln!(f, "    flags: {}", PageDirFlags((self.0 & 0x0fff) as u16))?;
        write!(f, "}}")
    }
}

/// page directory entry flags
/// all absent flags override flags of children, i.e. not having the read write bit set prevents
/// all page table entries in the page directory from being writable
#[bitmask(u16)]
#[repr(transparent)]
pub enum PageDirFlags {
    /// no flags?
    None                = Self(0),

    /// pages are present in memory and can be accessed
    Present             = Self(1 << 0),

    /// code can read/write to pages
    /// absence of this flag forces read only
    ReadWrite           = Self(1 << 1),

    /// pages are accessible in user mode
    /// absence of this flag only allows supervisor access
    UserSupervisor      = Self(1 << 2),

    /// enables write-through caching instead of write-back
    /// requires page attribute table
    PageWriteThru       = Self(1 << 3),

    /// disables caching for this page
    /// requires page attribute table
    PageCacheDisable    = Self(1 << 4),

    /// set if page has been accessed during address translation
    Accessed            = Self(1 << 5),

    /// set if page has been written to
    /// only available if page is large
    Dirty               = Self(1 << 6),

    /// enables large (4mb) pages
    /// no support currently
    PageSize            = Self(1 << 7),

    /// tells cpu to not invalidate this page table entry in cache when page tables are reloaded
    Global              = Self(1 << 8),

    /// can be set if page attribute table is supported, allows setting cache disable and write thru bits
    PageAttributeTable  = Self(1 << 12),
}

impl fmt::Display for PageDirFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageDirFlags {{")?;

        if self.0 & (1 << 0) > 0 {
            write!(f, " present,")?;
        }

        if self.0 & (1 << 1) > 0 {
            write!(f, " read/write")?;
        } else {
            write!(f, " read only")?;
        }

        if self.0 & (1 << 2) > 0 {
            write!(f, ", user + supervisor mode")?;
        } else {
            write!(f, ", supervisor mode")?;
        }

        if self.0 & (1 << 3) > 0 {
            write!(f, ", write thru")?;
        }

        if self.0 & (1 << 4) > 0 {
            write!(f, ", cache disable")?;
        }

        if self.0 & (1 << 5) > 0 {
            write!(f, ", accessed")?;
        }

        if self.0 & (1 << 6) > 0 {
            write!(f, ", dirty")?;
        }

        if self.0 & (1 << 7) > 0 {
            write!(f, ", large")?;
        }

        if self.0 & (1 << 8) > 0 {
            write!(f, ", global")?;
        }

        if self.0 & (1 << 12) > 0 {
            write!(f, ", page attribute table")?;
        }

        write!(f, " }}")
    }
}

// based on http://www.jamesmolloy.co.uk/tutorial_html/6.-Paging.html

/// where to allocate memory
static mut PLACEMENT_ADDR: usize = 0; // to be filled in with end of kernel on init

/// result of kmalloc calls
pub struct MallocResult<T> {
    pub pointer: *mut T,
    pub phys_addr: usize,
}

/// extremely basic malloc- doesn't support free, only useful for allocating effectively static data
unsafe fn kmalloc<T>(size: usize, align: bool) -> MallocResult<T> {
    /*if let Some(heap) = KERNEL_HEAP.as_mut() {
        let pointer = heap.alloc::<T>(size, if align { PAGE_SIZE } else { 0 });
        let phys_addr = virt_to_phys(pointer as usize).unwrap();

        MallocResult {
            pointer, phys_addr,
        }
    } else {*/
        if align && (PLACEMENT_ADDR & 0xfffff000) > 0 { // if alignment is requested and we aren't already aligned
            PLACEMENT_ADDR &= 0xfffff000; // round down to nearest 4k block
            PLACEMENT_ADDR += 0x1000; // increment by 4k- we don't want to overwrite things
        }

        // increment address to make room for area of provided size, return pointer to start of area
        let tmp = PLACEMENT_ADDR;
        PLACEMENT_ADDR += size;

        if PLACEMENT_ADDR >= 0x400000 { // prolly won't happen but might as well
            panic!("out of memory (kmalloc)");
        }

        MallocResult {
            pointer: (tmp + LINKED_BASE) as *mut T,
            phys_addr: tmp,
        }
    //}
}

/// struct for page table
/// basically just a wrapper for the array lmao
#[repr(C)]
pub struct PageTable {
    pub entries: [PageTableEntry; 1024],
}

/// struct for page directory
/// could be laid out better, but works fine for now
#[repr(C)] // im pretty sure this guarantees the order and size of this struct
pub struct PageDirectory {
    /// pointers to page tables
    pub tables: [*mut PageTable; 1024], // FIXME: maybe we want references here? too lazy to deal w borrow checking rn

    /// physical addresses of page tables (raw pointer bc references are Annoying and shit breaks without it)
    pub tables_physical: *mut [u32; 1024],

    /// physical address of tables_physical lmao
    pub tables_physical_addr: u32,

    /// bitset to speed up allocation of page frames
    pub frame_set: BitSet,

    /// counter of how many times the page directory has been updated
    /// can be used to check if partial copies of this page directory elsewhere are out of date
    pub page_updates: usize,
}

impl PageDirectory {
    /// creates a new page directory, allocating memory for it in the process
    pub fn new() -> Self {
        let num_frames = unsafe { MEM_SIZE >> 12 };
        let tables_physical = unsafe { kmalloc::<[u32; 1024]>(1024 * size_of::<u32>(), true) };

        debug!("tables_physical alloc @ {:#x}", tables_physical.pointer as usize);

        for phys in (unsafe { *tables_physical.pointer }).iter_mut() {
            *phys = 0;
        }

        PageDirectory {
            tables: [core::ptr::null_mut(); 1024],
            tables_physical: tables_physical.pointer, // shit breaks without this lmao
            tables_physical_addr: tables_physical.phys_addr as u32,
            frame_set: BitSet::place_at(unsafe { kmalloc::<u32>(num_frames / 32 * size_of::<u32>(), false).pointer }, num_frames), // BitSet::new uses the global allocator, which isn't initialized yet!
            page_updates: 0,
        }
    }

    /// gets a page from the directory if one exists, makes one if requested
    pub fn get_page(&mut self, mut addr: u32, make: bool) -> Option<*mut PageTableEntry> {
        addr >>= 12;
        let table_idx = (addr / 1024) as usize;
        if !self.tables[table_idx].is_null() { // page table already exists
            let table_ref = unsafe { &mut (*self.tables[table_idx]) };

            Some(&mut table_ref.entries[(addr % 1024) as usize])
        } else if make { // page table doesn't exist, create it
            unsafe {
                let ptr = kmalloc(1024 * 4, true); // page table entries are 32 bits (4 bytes) wide
                self.tables[table_idx] = ptr.pointer;
                let table_ref = &mut (*self.tables[table_idx]);
                for entry in table_ref.entries.iter_mut() {
                    entry.0 = 0;
                }
                (*self.tables_physical)[table_idx] = (ptr.phys_addr | 0x7) as u32; // present, read/write, user/supervisor
                
                Some(&mut table_ref.entries[(addr % 1024) as usize])
            }
        } else { // page table doesn't exist
            None
        }
    }
    
    /// allocates a frame for specified page
    pub unsafe fn alloc_frame(&mut self, page: *mut PageTableEntry, is_kernel: bool, is_writeable: bool) -> Option<u32> { // TODO: consider passing in flags?
        let page2 = &mut *page; // pointer shenanigans to get around the borrow checker lmao
        if page2.is_unused() {
            if let Some(idx) = self.frame_set.first_unset() {
                let mut flags = PageTableFlags::Present;
                if !is_kernel {
                    flags |= PageTableFlags::UserSupervisor;
                }
                if is_writeable {
                    flags |= PageTableFlags::ReadWrite;
                }

                self.frame_set.set(idx);
                page2.set_flags(flags);
                page2.set_address((idx << 12) as u32);
                self.page_updates = self.page_updates.wrapping_add(1); // we want this to be able to overflow

                Some((idx << 12) as u32)
            } else {
                panic!("out of memory (no free frames)");
            }
        } else {
            None
        }
    }

    /// frees a frame, allowing other things to use it
    pub unsafe fn free_frame(&mut self, page: *mut PageTableEntry) -> Option<u32> {
        let page2 = &mut *page; // pointer shenanigans
        if !page2.is_unused() {
            let addr = page2.get_address();
            self.frame_set.clear((addr >> 12) as usize);
            page2.set_unused();
            self.page_updates = self.page_updates.wrapping_add(1);

            Some(addr)
        } else {
            None
        }
    }

    /// switch global page directory to this page directory
    pub fn switch_to(&self) {
        unsafe {
            debug!("switching to page table @ phys {:#x}", self.tables_physical_addr);

            asm!(
                "mov cr3, {0}",
                "mov {1}, cr0",
                "or {1}, 0x80000000",
                "mov cr0, {1}",

                in(reg) self.tables_physical_addr,
                out(reg) _,
            );
        }
    }

    /// transform a virtual address to a physical address
    pub fn virt_to_phys(&mut self, addr: u32) -> Option<u32> {
        let page = self.get_page(addr, false)?;

        Some((unsafe { *page }).get_address() | (addr & (PAGE_SIZE as u32 - 1)))
    }
}

impl Default for PageDirectory {
    fn default() -> Self {
        Self::new()
    }
}

/// allocate region of memory
unsafe fn alloc_region(dir: &mut PageDirectory, start: u32, size: u32) {
    let end = start + size;

    for i in (start..end).step_by(PAGE_SIZE) {
        let page = dir.get_page(i, true).unwrap();
        dir.alloc_frame(page, false, true); // FIXME: switch to kernel mode when user tasks don't run in the kernel's address space
    }

    debug!("mapped {:#x} - {:#x}", start, end);
}

/// our page directory
pub static mut PAGE_DIR: Option<PageDirectory> = None;

/// initializes paging
pub unsafe fn init() {
    // calculate placement addr for kmalloc calls
    PLACEMENT_ADDR = (&kernel_end as *const _) as usize - LINKED_BASE; // we need a physical address for this

    debug!("kernel end @ {:#x}, linked @ {:#x}", (&kernel_end as *const _) as usize, LINKED_BASE);
    debug!("placement @ {:#x} (phys {:#x})", PLACEMENT_ADDR + LINKED_BASE, PLACEMENT_ADDR);

    // set up page directory struct
    let mut dir = PageDirectory::new();

    // FIXME: map initial kernel memory allocations as global so they won't be invalidated from TLB flushes

    debug!("mapping kernel memory");

    // map first 4mb of memory to LINKED_BASE
    alloc_region(&mut dir, LINKED_BASE as u32, 0x400000);

    debug!("mapping heap memory");

    // map initial memory for kernel heap
    alloc_region(&mut dir, KHEAP_START as u32, KHEAP_INITIAL_SIZE as u32);

    debug!("creating page table");

    // holy fuck we need maybeuninit so bad
    PAGE_DIR = Some(dir);

    debug!("switching to page table");

    // switch to our new page directory
    PAGE_DIR.as_ref().unwrap().switch_to();

    if let Some(dir) = PAGE_DIR.as_ref() {
        let bits_used = dir.frame_set.bits_used;
        log!("{}mb total, {}/{} mapped ({}mb), {}% usage", MEM_SIZE / 1024 / 1024, bits_used, dir.frame_set.size, bits_used / 256, (bits_used * 100) / dir.frame_set.size);
    }
}

/// allocate page and map to given address
pub fn alloc_page(addr: usize, is_kernel: bool, is_writeable: bool) {
    assert!(addr % PAGE_SIZE == 0, "address is not page aligned");

    let dir = unsafe { PAGE_DIR.as_mut().unwrap() };

    let page = dir.get_page(addr.try_into().unwrap(), true).unwrap();

    unsafe {
        if dir.alloc_frame(page, is_kernel, is_writeable).is_some() {
            asm!("invlpg [{0}]", in(reg) addr); // invalidate this page in the TLB
        }
    }
}

/// free page at given address
pub fn free_page(addr: usize) {
    assert!(addr % PAGE_SIZE == 0, "address is not page aligned");

    let dir = unsafe { PAGE_DIR.as_mut().unwrap() };

    if let Some(page) = dir.get_page(addr.try_into().unwrap(), false) {
        unsafe {
            if dir.free_frame(page).is_some() {
                asm!("invlpg [{0}]", in(reg) addr); // invalidate this page in the TLB
            }
        }
    }
}

/// convert virtual to physical address
pub fn virt_to_phys(addr: usize) -> Option<usize> {
    let dir = unsafe { PAGE_DIR.as_mut()? };

    let addr = if let Ok(res) = addr.try_into() { res } else { return None };

    match dir.virt_to_phys(addr) {
        Some(res) => match res.try_into() {
            Ok(ult) => Some(ult),
            Err(..) => None,
        },
        None => None,
    }
}

/// bump allocate some memory
pub unsafe fn bump_alloc<T>(size: usize, alignment: usize) -> *mut T {
    let offset: usize = 
        if PLACEMENT_ADDR % alignment != 0 {
            alignment - (PLACEMENT_ADDR % alignment)
        } else {
            0
        };
    
    PLACEMENT_ADDR += offset;

    let tmp = PLACEMENT_ADDR;
    PLACEMENT_ADDR += size;

    if PLACEMENT_ADDR >= 0x400000 { // prolly won't happen but might as well
        panic!("out of memory (kmalloc)");
    }

    (tmp + LINKED_BASE) as *mut T
}
