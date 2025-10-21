#[test]
fn it_works() -> anyhow::Result<()> {
    use crate::monitor::{CpuInfo, DiskInfo, MemInfo, Monitorable, NetInfo};
    use crate::ssh::SSHClient;
    let client = SSHClient::with_pswd("10.210.126.58", "harkerhand", "harkerhand")?;
    let mem = client.exec(MemInfo::default())?;
    println!("{}", mem.common_display());
    let disk = client.exec(DiskInfo::default())?;
    println!("{}", disk.common_display());
    let cpu = client.exec(CpuInfo::default())?;
    println!("{}", cpu.common_display());
    let net = client.exec(NetInfo::default())?;
    println!("{}", net.common_display());

    panic!()
}

