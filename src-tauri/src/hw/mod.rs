pub mod battery;
pub mod board;
pub mod codenames;
pub mod cpu;
#[cfg(windows)]
pub mod devnode;
pub mod edid;
pub mod gpu;
pub mod memory;
#[cfg(all(windows, target_arch = "x86_64"))]
pub mod pawnio;
pub mod pci;
pub mod peripherals;
pub mod privileged;
pub mod spd;
pub mod storage;
pub mod upgrade;
#[cfg(windows)]
pub mod wmi;
