use screeps::Creep;
use screeps::Part;
use screeps::Position;
use screeps::ResourceType;
use screeps::prelude::*;

use crate::transport::BidirTaskHandle;
use crate::transport::ExportTaskHandle;
use crate::transport::ImportTaskHandle;
use crate::transport::Priority;
use crate::transport::TransportTask;
use crate::transport::resource::ResourceExport;
use crate::transport::resource::ResourceImport;
use crate::transport::solver::solve;
use crate::transport::task::TransportAllocation;

pub struct TransportAllocator {
    pub(super) id_counter: u16,
    pub(super) creeps: Vec<Info>,
    pub(super) imports: Vec<ImportInfo>,
    pub(super) exports: Vec<ExportInfo>,
}

#[derive(Debug)]
pub(super) enum TaskType {
    Bidir,
    Import,
    Export,
}

#[derive(Debug)]
pub(super) struct Info {
    pub task: Option<TransportTask>,
    pub size: u8,
    pub used_capacity: u32,
    pub free_capacity: u32,
    pub pos: Position,
    pub spawning: bool,
    pub task_type: TaskType,
}

#[derive(Debug)]
pub(super) struct ImportInfo {
    pub target: ResourceImport,
    pub priority: Priority,
    pub amount: u32,
    pub pos: Position,
}

#[derive(Debug)]
pub(super) struct ExportInfo {
    pub target: ResourceExport,
    pub priority: Priority,
    pub amount: u32,
    pub pos: Position,
}

impl TransportAllocator {
    pub fn new() -> Self {
        TransportAllocator {
            id_counter: 0,
            creeps: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    fn next_id(&mut self) -> u16 {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_hauler(
        &mut self,
        creep: &Creep,
        task: Option<TransportTask>,
    ) -> BidirTaskHandle {
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.creeps.push(Info {
            task,
            size,
            used_capacity: creep.store().get_used_capacity(None),
            free_capacity: creep.store().get_free_capacity(None) as u32,
            pos: creep.pos(),
            spawning: creep.spawning(),
            task_type: TaskType::Bidir,
        });

        BidirTaskHandle::new(self.next_id())
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_import(
        &mut self,
        creep: &Creep,
        task: Option<TransportTask>,
    ) -> ImportTaskHandle {
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.creeps.push(Info {
            task,
            size,
            used_capacity: creep.store().get_used_capacity(None),
            free_capacity: creep.store().get_free_capacity(None) as u32,
            pos: creep.pos(),
            spawning: creep.spawning(),
            task_type: TaskType::Import,
        });

        ImportTaskHandle::new(self.next_id())
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_export(
        &mut self,
        creep: &Creep,
        task: Option<TransportTask>,
    ) -> ExportTaskHandle {
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.creeps.push(Info {
            task,
            size,
            used_capacity: creep.store().get_used_capacity(None),
            free_capacity: creep.store().get_free_capacity(None) as u32,
            pos: creep.pos(),
            spawning: creep.spawning(),
            task_type: TaskType::Export,
        });

        ExportTaskHandle::new(self.next_id())
    }

    pub fn register_export(&mut self, export: impl Into<ResourceExport>, priority: Priority) {
        let target = export.into();
        let amount = target.store().get(ResourceType::Energy).unwrap_or(0);
        let pos = target.pos();
        if amount > 0 {
            self.exports.push(ExportInfo {
                target,
                priority,
                amount,
                pos,
            });
        }
    }

    pub fn register_import(&mut self, import: impl Into<ResourceImport>, priority: Priority) {
        let target = import.into();
        let amount = target.store().get_free_capacity(Some(ResourceType::Energy)) as u32;
        let pos = target.pos();
        if amount > 0 {
            self.imports.push(ImportInfo {
                target,
                priority,
                amount,
                pos,
            });
        }
    }

    pub fn allocate(self) -> TransportAllocation {
        solve(self)
    }
}
