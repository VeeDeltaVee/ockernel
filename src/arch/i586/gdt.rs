//! i586 global descriptor table (GDT) and task state segment (TSS)

use aligned::{Aligned, A16};
use x86::dtables::{DescriptorTablePointer, lgdt};
use bitmask_enum::bitmask;
use core::{
    arch::asm,
    mem::size_of,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// entry in GDT
#[repr(transparent)]
#[derive(Copy, Clone)]
struct GDTEntry(u64);

impl GDTEntry {
    /// create a new GDT entry. honestly just yoinked code from osdev wiki because the entry structure is batshit insane
    #[allow(clippy::needless_late_init)]
    pub fn new(base: u32, limit: u32, flags: GDTFlags) -> Self {
        let mut descriptor: u64;

        // Create the high 32 bit segment
        descriptor  = ( limit         & 0x000F0000) as u64; // set limit bits 19:16
        descriptor |= ((flags.0 << 8) & 0x00F0FF00) as u64; // set type, p, dpl, s, g, d/b, l and avl fields
        descriptor |= ((base   >> 16) & 0x000000FF) as u64; // set base bits 23:16
        descriptor |= ( base          & 0xFF000000) as u64; // set base bits 31:24

        // Shift by 32 to allow for low part of segment
        descriptor <<= 32;

        // Create the low 32 bit segment
        descriptor |= (base  << 16) as u64;         // set base bits 15:0
        descriptor |= (limit  & 0x0000FFFF) as u64; // set limit bits 15:0

        Self(descriptor)
    }
}

/// GDT flags
#[bitmask(u32)]
enum GDTFlags {
    //  system descriptor type (default)
    //DescTypeSys       = Self(0 << 0x04),

    /// code/data descriptor type
    DescTypeCodeData    = Self(1 << 0x04),

    /// present
    Present             = Self(1 << 0x07),

    /// available for system use
    //SysAvail          = Self(1 << 0x0c),

    //  long mode (lmao why)
    //LongMode          = Self(1 << 0x0d),

    //  16 bit (default)
    //Size16            = Self(0 << 0x0e),

    /// 32 bit
    Size32              = Self(1 << 0x0e),

    //  granularity (1b - 1mb, default)
    //GranSmall         = Self(0 << 0x0f),

    /// granularity (4kb - 4gb)
    GranLarge           = Self(1 << 0x0f),

    //  privilege level 0
    //Priv0             = Self(0 << 0x05),

    /// privilege level 1
    Priv1               = Self(1 << 0x05),

    /// privilege level 2
    Priv2               = Self(2 << 0x05),

    /// privilege level 3
    Priv3               = Self(3 << 0x05),

    //DataReadOnly      = Self(0x00), // 0b0000 (default)
    DataAccessed        = Self(0x01), // 0b0001
    DataReadWrite       = Self(0x02), // 0b0010
    DataExpandDown      = Self(0x04), // 0b0100
    DataConform         = Self(0x04), // 0b0100 (duplicate because wording was different on wiki so maybe this'll be more readable?)
    DataExecute         = Self(0x08), // 0b1000

    CodePriv0           = Self(Self::DescTypeCodeData.0 | Self::Present.0 | Self::Size32.0 | Self::GranLarge.0 | Self::DataExecute.0 | Self::DataReadWrite.0),
    DataPriv0           = Self(Self::DescTypeCodeData.0 | Self::Present.0 | Self::Size32.0 | Self::GranLarge.0 | Self::DataReadWrite.0),
    CodePriv3           = Self(Self::DescTypeCodeData.0 | Self::Present.0 | Self::Size32.0 | Self::GranLarge.0 | Self::Priv3.0 | Self::DataExecute.0 | Self::DataReadWrite.0),
    DataPriv3           = Self(Self::DescTypeCodeData.0 | Self::Present.0 | Self::Size32.0 | Self::GranLarge.0 | Self::Priv3.0 | Self::DataReadWrite.0),
    TaskStateSegment    = Self(Self::Present.0 | Self::DataExecute.0 | Self::DataAccessed.0),
}

/// TSS
#[repr(C, packed(16))]
pub struct TaskStateSegment {
    pub link: u32,
    pub esp0: u32,
    pub ss0: u32,
    pub esp1: u32,
    pub ss1: u32,
    pub esp2: u32,
    pub ss2: u32,
    pub cr3: u32,
    pub eip: u32,
    pub eflags: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub es: u32,
    pub cs: u32,
    pub ss: u32,
    pub ds: u32,
    pub fs: u32,
    pub gs: u32,
    pub ldtr: u32,
    pub iopb: u16,
    pub ssp: u16,
}

impl TaskStateSegment {
    // 💀
    pub const fn new() -> Self {
        Self {
            link: 0,
            esp0: 0,
            ss0: 0,
            esp1: 0,
            ss1: 0,
            esp2: 0,
            ss2: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0,
            cs: 0,
            ss: 0,
            ds: 0,
            fs: 0,
            gs: 0,
            ldtr: 0,
            iopb: 0,
            ssp: 0,
        }
    }
}

/// how many entries do we want in our GDT
const GDT_ENTRIES: usize = 5;

/// the GDT itself (aligned to 16 bits for performance)
static mut GDT: Aligned<A16, [GDTEntry; GDT_ENTRIES + 1]> = Aligned([GDTEntry(0); GDT_ENTRIES + 1]);

/// the TSS lmao
static mut TSS: Aligned<A16, TaskStateSegment> = Aligned(TaskStateSegment::new());

/// size of kernel stack
const STACK_SIZE: usize = 4096 * 5; // 20k

/// kernel stack
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

/// flush TSS
unsafe fn flush_tss() {
    let index = (5 * 8) | 3;
    asm!("ltr ax", in("ax") index);
}

/// initialize GDT and TSS
pub unsafe fn init() {
    // populate TSS
    TSS.ss0 = 0x10; // kernel data segment descriptor
    TSS.esp0 = (&STACK as *const _) as u32 + STACK_SIZE as u32 - 1;
    TSS.cs = 0x0b;
    TSS.ds = 0x13;
    TSS.es = 0x13;
    TSS.fs = 0x13;
    TSS.gs = 0x13;
    //TSS.esp0 = 0xc03fffff;
    //TSS.iopb = size_of::<TaskStateSegment>() as u16; // size of TSS

    // populate GDT
    GDT[1] = GDTEntry::new(0, 0x000fffff, GDTFlags::CodePriv0);
    GDT[2] = GDTEntry::new(0, 0x000fffff, GDTFlags::DataPriv0);
    GDT[3] = GDTEntry::new(0, 0x000fffff, GDTFlags::CodePriv3);
    GDT[4] = GDTEntry::new(0, 0x000fffff, GDTFlags::DataPriv3);
    
    let base = (&TSS as *const _) as u32;
    GDT[5] = GDTEntry::new(base, base + size_of::<TaskStateSegment>() as u32, GDTFlags::TaskStateSegment);

    // load GDT
    let gdt_desc = DescriptorTablePointer::new(&GDT);
    lgdt(&gdt_desc);

    flush_tss();
}
