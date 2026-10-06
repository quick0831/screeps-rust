use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;

use enum_dispatch::enum_dispatch;
use screeps::CARRY_CAPACITY;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::ResourceType;
use screeps::RoomObject;
use screeps::Store;
use screeps::StructureContainer;
use screeps::StructureExtension;
use screeps::StructureSpawn;
use screeps::StructureStorage;
use screeps::StructureTower;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::KeyCmp;

pub struct TransportAllocator {
    haulers: HashMap<ObjectId<Creep>, Info>,
    imports: Vec<ImportInfo>,
    exports: Vec<ExportInfo>,
}

struct Info {
    task: Option<Task>,
    size: u8,
    carrying: u32,
}

struct ImportInfo {
    target: ResourceImport,
    priority: Priority,
    amount: u32,
}

struct ExportInfo {
    target: ResourceExport,
    priority: Priority,
    amount: u32,
}

impl TransportAllocator {
    pub fn new() -> Self {
        TransportAllocator {
            haulers: HashMap::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    pub fn register_hauler(&mut self, creep: &Creep, task: Option<Task>) {
        let Some(creep_id) = creep.try_id() else {
            return;
        };
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        let carrying = creep.store().get_used_capacity(None);
        self.haulers.insert(
            creep_id,
            Info {
                task,
                size,
                carrying,
            },
        );
    }

    pub fn register_export(&mut self, export: impl Into<ResourceExport>, priority: Priority) {
        let target = export.into();
        let amount = target.store().get(ResourceType::Energy).unwrap_or(0);
        if amount > 0 {
            self.exports.push(ExportInfo {
                target,
                priority,
                amount,
            });
        }
    }

    pub fn register_import(&mut self, import: impl Into<ResourceImport>, priority: Priority) {
        let target = import.into();
        let amount = target.store().get_free_capacity(Some(ResourceType::Energy)) as u32;
        if amount > 0 {
            self.imports.push(ImportInfo {
                target,
                priority,
                amount,
            });
        }
    }

    pub fn allocate(&mut self) {
        let being_served: HashSet<_> = self.haulers.values().filter_map(|info| info.task).collect();
        let mut pending_export: BinaryHeap<_> = self
            .exports
            .iter()
            .map(|info| (info.target.id(), info))
            .filter(|(id, _)| !being_served.contains(&Task::Pickup(*id)))
            .map(|(id, info)| KeyCmp {
                key: (info.priority, info.amount),
                value: id,
            })
            .collect();
        let mut pending_import: Vec<_> = self
            .imports
            .iter()
            .map(|info| (info.target.id(), info))
            .filter(|(id, _)| !being_served.contains(&Task::Supply(*id)))
            .map(|(id, info)| (id, info.target.pos(), info.priority, info.amount))
            .collect();
        let mut idle_haulers: BinaryHeap<_> = self
            .haulers
            .iter()
            .filter(|(_, info)| info.task.is_none())
            .map(|(creep_id, info)| KeyCmp {
                key: info.size,
                value: (*creep_id, info.carrying),
            })
            .collect();

        while let Some(KeyCmp {
            key: size,
            value: (hauler_id, hauler_carrying),
        }) = idle_haulers.pop()
        {
            let carriable = size as u32 * CARRY_CAPACITY;
            let task = if hauler_carrying > 20 {
                let creep = hauler_id.resolve();
                let Some(creep) = creep else { continue };
                let creep_pos = creep.pos();
                let idx = pending_import
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, (_, pos, priority, _))| {
                        (*priority, Reverse(pos.get_range_to(creep_pos)))
                    })
                    .map(|(idx, _)| idx);
                let Some(idx) = idx else { break };
                let (import_id, pos, priority, amount) = pending_import.swap_remove(idx);
                if hauler_carrying < amount {
                    pending_import.push((import_id, pos, priority, amount - hauler_carrying));
                }
                Task::Supply(import_id)
            } else {
                let Some(KeyCmp {
                    key: (priority, energy),
                    value: export_id,
                }) = pending_export.pop()
                else {
                    break;
                };
                if carriable < energy {
                    pending_export.push(KeyCmp {
                        key: (priority, energy - carriable),
                        value: export_id,
                    });
                }
                Task::Pickup(export_id)
            };
            self.haulers.get_mut(&hauler_id).unwrap().task = Some(task);
        }
    }

    pub fn delegate(&self, creep: &Creep) -> Option<Task> {
        self.haulers
            .get(&creep.try_id()?)
            .and_then(|info| info.task)
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
pub enum Task {
    Pickup(ResourceExportId),
    Supply(ResourceImportId),
}

#[allow(unused)]
#[enum_dispatch(ResourceExport)]
trait ResourceExporter {}

#[allow(unused)]
#[enum_dispatch(ResourceImport)]
trait ResourceImporter {}

// Contract: All variant must implement `Withdrawable`
#[enum_dispatch]
pub enum ResourceExport {
    Container(StructureContainer),
    Spawn(StructureSpawn),
    Extension(StructureExtension),
    Storage(StructureStorage),
}

// Contract: All variant must implement `Transferable`
#[enum_dispatch]
pub enum ResourceImport {
    Spawn(StructureSpawn),
    Extension(StructureExtension),
    Tower(StructureTower),
    Storage(StructureStorage),
}

impl ResourceExport {
    fn id(&self) -> ResourceExportId {
        match self {
            Self::Container(container) => ResourceExportId::Container(container.id()),
            Self::Spawn(spawn) => ResourceExportId::Spawn(spawn.id()),
            Self::Extension(extension) => ResourceExportId::Extension(extension.id()),
            Self::Storage(storage) => ResourceExportId::Storage(storage.id()),
        }
    }
}

impl ResourceImport {
    fn id(&self) -> ResourceImportId {
        match self {
            Self::Spawn(spawn) => ResourceImportId::Spawn(spawn.id()),
            Self::Extension(extension) => ResourceImportId::Extension(extension.id()),
            Self::Tower(tower) => ResourceImportId::Tower(tower.id()),
            Self::Storage(storage) => ResourceImportId::Storage(storage.id()),
        }
    }
}

impl HasStore for ResourceExport {
    fn store(&self) -> Store {
        match self {
            Self::Container(container) => container.store(),
            Self::Spawn(spawn) => spawn.store(),
            Self::Extension(extension) => extension.store(),
            Self::Storage(storage) => storage.store(),
        }
    }
}

impl HasStore for ResourceImport {
    fn store(&self) -> Store {
        match self {
            Self::Spawn(spawn) => spawn.store(),
            Self::Extension(extension) => extension.store(),
            Self::Tower(tower) => tower.store(),
            Self::Storage(storage) => storage.store(),
        }
    }
}

impl Withdrawable for ResourceExport {}

impl Transferable for ResourceImport {}

impl AsRef<RoomObject> for ResourceExport {
    fn as_ref(&self) -> &RoomObject {
        match self {
            Self::Container(container) => container.as_ref(),
            Self::Spawn(spawn) => spawn.as_ref(),
            Self::Extension(extension) => extension.as_ref(),
            Self::Storage(storage) => storage.as_ref(),
        }
    }
}

impl AsRef<RoomObject> for ResourceImport {
    fn as_ref(&self) -> &RoomObject {
        match self {
            Self::Spawn(spawn) => spawn.as_ref(),
            Self::Extension(extension) => extension.as_ref(),
            Self::Tower(tower) => tower.as_ref(),
            Self::Storage(storage) => storage.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum ResourceExportId {
    Container(ObjectId<StructureContainer>),
    Spawn(ObjectId<StructureSpawn>),
    Extension(ObjectId<StructureExtension>),
    Storage(ObjectId<StructureStorage>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum ResourceImportId {
    Spawn(ObjectId<StructureSpawn>),
    Extension(ObjectId<StructureExtension>),
    Tower(ObjectId<StructureTower>),
    Storage(ObjectId<StructureStorage>),
}

impl ResourceExportId {
    pub fn resolve(&self) -> Option<ResourceExport> {
        Some(match self {
            ResourceExportId::Container(id) => ResourceExport::Container(id.resolve()?),
            ResourceExportId::Spawn(id) => ResourceExport::Spawn(id.resolve()?),
            ResourceExportId::Extension(id) => ResourceExport::Extension(id.resolve()?),
            ResourceExportId::Storage(id) => ResourceExport::Storage(id.resolve()?),
        })
    }
}

impl ResourceImportId {
    pub fn resolve(&self) -> Option<ResourceImport> {
        Some(match self {
            ResourceImportId::Spawn(id) => ResourceImport::Spawn(id.resolve()?),
            ResourceImportId::Extension(id) => ResourceImport::Extension(id.resolve()?),
            ResourceImportId::Tower(id) => ResourceImport::Tower(id.resolve()?),
            ResourceImportId::Storage(id) => ResourceImport::Storage(id.resolve()?),
        })
    }
}
