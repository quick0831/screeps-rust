use serde::{Deserialize, Serialize};

use crate::transport::resource::ResourceExportId;
use crate::transport::resource::ResourceImportId;

pub struct TransportAllocation {
    alloc: Vec<Option<TransportTask>>,
}

impl TransportAllocation {
    pub(super) fn new(alloc: Vec<Option<TransportTask>>) -> Self {
        Self { alloc }
    }
}

#[derive(Debug)]
pub struct BidirTaskHandle(u16);
#[derive(Debug)]
pub struct ImportTaskHandle(u16);
#[derive(Debug)]
pub struct ExportTaskHandle(u16);

impl BidirTaskHandle {
    pub(super) fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn resolve(&self, transport: &TransportAllocation) -> Option<TransportTask> {
        transport.alloc[self.0 as usize]
    }
}

impl ImportTaskHandle {
    pub(super) fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn resolve(&self, transport: &TransportAllocation) -> Option<TransportTask> {
        transport.alloc[self.0 as usize]
    }
}

impl ExportTaskHandle {
    pub(super) fn new(id: u16) -> Self {
        Self(id)
    }

    #[allow(unused)]
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<TransportTask> {
        transport.alloc[self.0 as usize]
    }
}

impl From<ImportTaskHandle> for BidirTaskHandle {
    fn from(value: ImportTaskHandle) -> Self {
        BidirTaskHandle(value.0)
    }
}

impl From<ExportTaskHandle> for BidirTaskHandle {
    fn from(value: ExportTaskHandle) -> Self {
        BidirTaskHandle(value.0)
    }
}

// Priority::High > Priority::Low
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Passive,
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task")]
pub enum TransportTask {
    Pickup(ResourceExportId),
    Supply(ResourceImportId),
}
