pub mod cpu;
pub use cpu::CpuInfo;
pub mod mem;
pub use mem::MemInfo;
pub mod disk;
pub use disk::DiskInfo;
pub mod net;
pub use net::NetInfo;

pub trait Monitorable: Default {
    fn exec_cmd(&self) -> &'static str;

    fn parse_from_str(&mut self, s: &str) -> anyhow::Result<()>;

    fn common_display(&self) -> String;
}
