use crate::monitor::{CpuInfo, DiskInfo, MemInfo, Monitorable, NetInfo};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Hash, PartialEq, Eq)]
pub enum MonitorKind {
    Mem,
    Cpu,
    Disk,
    Net,
}

impl MonitorKind {
    pub fn variants() -> Vec<&'static str> {
        vec!["mem", "cpu", "disk", "net"]
    }
}

impl TryFrom<&str> for MonitorKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "mem" => Ok(MonitorKind::Mem),
            "cpu" => Ok(MonitorKind::Cpu),
            "disk" => Ok(MonitorKind::Disk),
            "net" => Ok(MonitorKind::Net),
            _ => Err(format!("unknown monitor kind: {}", value)),
        }
    }
}

#[derive(Debug)]
pub enum MonitorPayload {
    Mem(MemInfo),
    Cpu(CpuInfo),
    Disk(DiskInfo),
    Net(NetInfo),
    None,
}

impl From<&MonitorKind> for MonitorPayload {
    fn from(value: &MonitorKind) -> Self {
        match value {
            MonitorKind::Mem => MonitorPayload::Mem(MemInfo::default()),
            MonitorKind::Cpu => MonitorPayload::Cpu(CpuInfo::default()),
            MonitorKind::Disk => MonitorPayload::Disk(DiskInfo::default()),
            MonitorKind::Net => MonitorPayload::Net(NetInfo::default()),
        }
    }
}

impl Default for MonitorPayload {
    fn default() -> Self {
        MonitorPayload::None
    }
}

impl Monitorable for MonitorPayload {
    fn exec_cmd(&self) -> &'static str {
        match self {
            MonitorPayload::Mem(info) => info.exec_cmd(),
            MonitorPayload::Cpu(info) => info.exec_cmd(),
            MonitorPayload::Disk(info) => info.exec_cmd(),
            MonitorPayload::Net(info) => info.exec_cmd(),
            MonitorPayload::None => "",
        }
    }

    fn parse_from_str(&mut self, s: &str) -> anyhow::Result<()> {
        match self {
            MonitorPayload::Mem(info) => info.parse_from_str(s),
            MonitorPayload::Cpu(info) => info.parse_from_str(s),
            MonitorPayload::Disk(info) => info.parse_from_str(s),
            MonitorPayload::Net(info) => info.parse_from_str(s),
            MonitorPayload::None => Ok(()),
        }
    }

    fn common_display(&self) -> String {
        match self {
            MonitorPayload::Mem(info) => info.common_display(),
            MonitorPayload::Cpu(info) => info.common_display(),
            MonitorPayload::Disk(info) => info.common_display(),
            MonitorPayload::Net(info) => info.common_display(),
            MonitorPayload::None => String::from("No Data"),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum MonitorEvent {
    Sample {
        server: String,
        kind: MonitorKind,
        payload: MonitorPayload,
        timestamp: DateTime<Utc>,
    },
    Error {
        server: String,
        kind: Option<MonitorKind>,
        error: String,
        timestamp: DateTime<Utc>,
    },
}
