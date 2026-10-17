use std::arch::wasm32::unreachable;
use std::cmp::max;
use std::cmp::min;

use enum_dispatch::enum_dispatch;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::Position;
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

use crate::utils::Fraction;
use crate::utils::PriorityQueue;

pub struct TransportAllocator {
    hauler_id_counter: u16,
    import_id_counter: u16,
    haulers: Vec<Info>,
    creep_imports: Vec<Info>,
    imports: Vec<ImportInfo>,
    exports: Vec<ExportInfo>,
}

pub struct TransportAllocation {
    hauler: Vec<Option<Task>>,
    import: Vec<Option<Task>>,
}

#[derive(Debug)]
pub struct HaulerTaskHandle(u16);
#[derive(Debug)]
pub struct ImportTaskHandle(u16);

struct Info {
    task: Option<Task>,
    size: u8,
    used_capacity: u32,
    free_capacity: u32,
    pos: Position,
    spawning: bool,
}

struct ImportInfo {
    target: ResourceImport,
    priority: Priority,
    amount: u32,
    pos: Position,
}

struct ExportInfo {
    target: ResourceExport,
    priority: Priority,
    amount: u32,
    pos: Position,
}

impl TransportAllocator {
    pub fn new() -> Self {
        TransportAllocator {
            hauler_id_counter: 0,
            import_id_counter: 0,
            haulers: Vec::new(),
            creep_imports: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_hauler(&mut self, creep: &Creep, task: Option<Task>) -> HaulerTaskHandle {
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.haulers.push(Info {
            task,
            size,
            used_capacity: creep.store().get_used_capacity(None),
            free_capacity: creep.store().get_free_capacity(None) as u32,
            pos: creep.pos(),
            spawning: creep.spawning(),
        });

        let id = self.hauler_id_counter;
        self.hauler_id_counter += 1;
        HaulerTaskHandle(id)
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_import(&mut self, creep: &Creep, task: Option<Task>) -> ImportTaskHandle {
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.creep_imports.push(Info {
            task,
            size,
            used_capacity: creep.store().get_used_capacity(None),
            free_capacity: creep.store().get_free_capacity(None) as u32,
            pos: creep.pos(),
            spawning: creep.spawning(),
        });

        let id = self.import_id_counter;
        self.import_id_counter += 1;
        ImportTaskHandle(id)
    }

    /*
    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_export(&mut self, creep: &Creep) {
        // TODO: deposit whatever it is carrying
        let _ = creep;
    }
    */

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

    pub fn allocate(mut self) -> TransportAllocation {
        let mut alloc = TransportAllocation {
            hauler: vec![None; self.hauler_id_counter as usize],
            import: vec![None; self.import_id_counter as usize],
        };

        // Remove corresponded amount already queued
        for info in self.haulers.iter().chain(&self.creep_imports) {
            let Some(task) = info.task else { continue };
            match task {
                Task::Pickup(id) => {
                    if let Some(export) = self.exports.iter_mut().find(|ex| ex.target.id() == id) {
                        export.amount = export.amount.saturating_sub(info.free_capacity);
                    }
                }
                Task::Supply(id) => {
                    if let Some(import) = self.imports.iter_mut().find(|ex| ex.target.id() == id) {
                        import.amount = import.amount.saturating_sub(info.used_capacity);
                    }
                }
            }
        }

        self.exports.retain(|info| info.amount > 0);
        self.imports.retain(|info| info.amount > 0);

        // calculate needed network flow
        let creep_import_amount: u32 = self.creep_imports.iter().map(|i| i.free_capacity).sum();
        let total_import: u32 = self.imports.iter().map(|info| info.amount).sum();
        let total_export: u32 = self.exports.iter().map(|info| info.amount).sum();
        let total_export = total_export + creep_import_amount;
        let total_hauler_holding: u32 = self.haulers.iter().map(|info| info.used_capacity).sum();
        let active_import: u32 = self
            .imports
            .iter()
            .filter(|info| info.priority != Priority::Passive)
            .map(|info| info.amount)
            .sum();
        let active_export: u32 = self
            .exports
            .iter()
            .filter(|info| info.priority != Priority::Passive)
            .map(|info| info.amount)
            .sum();
        let active_export = active_export + creep_import_amount;

        let network_cap = min(total_import, total_export);
        let mut export_requested = min(
            network_cap,
            max(
                active_import.saturating_sub(total_hauler_holding),
                active_export,
            ),
        );
        let mut import_requested = min(
            network_cap,
            max(active_import, active_export + total_hauler_holding),
        );

        enum TaskType {
            Hauler,
            Import,
        }

        let creep_import_infos: PriorityQueue<_, (&Info, TaskType, usize)> = self
            .creep_imports
            .iter()
            .enumerate()
            .map(|(id, info)| (info.size, (info, TaskType::Import, id)))
            .collect();

        let hauler_infos: PriorityQueue<_, (&Info, TaskType, usize)> = self
            .haulers
            .iter()
            .enumerate()
            .map(|(id, info)| (info.size, (info, TaskType::Hauler, id)))
            .collect();

        // dispatch task for haulers without a task
        for (_, (info, task_type, id)) in creep_import_infos.into_iter().chain(hauler_infos) {
            if info.task.is_some() || info.spawning {
                continue;
            }

            let can_import = matches!(task_type, TaskType::Hauler);
            let import_candidate = if can_import && import_requested > 0 {
                self.imports
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, import)| {
                        let amount = min(import.amount, info.used_capacity);
                        if amount != 0 {
                            let distance = import.pos.get_range_to(info.pos);
                            Some((idx, (import.priority, Fraction(amount, distance))))
                        } else {
                            None
                        }
                    })
                    .max_by_key(|(_, score)| *score)
            } else {
                None
            };

            let export_candidate = if export_requested > 0 {
                self.exports
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, export)| {
                        let amount = min(export.amount, info.free_capacity);
                        if amount != 0 {
                            let distance = export.pos.get_range_to(info.pos);
                            Some((idx, (export.priority, Fraction(amount, distance))))
                        } else {
                            None
                        }
                    })
                    .max_by_key(|(_, score)| *score)
            } else {
                None
            };

            let task = match (import_candidate, export_candidate) {
                (Some((id, im)), ex) if ex.is_none_or(|(_, ex)| im > ex) => {
                    let mut import = self.imports.swap_remove(id);
                    let id = import.target.id();
                    let amount = min(import.amount, info.used_capacity);
                    import_requested -= amount;
                    import.amount -= amount;
                    if import.amount > 0 {
                        self.imports.push(import);
                    }
                    Task::Supply(id)
                }
                (_, Some((id, _))) => {
                    let mut export = self.exports.swap_remove(id);
                    let id = export.target.id();
                    let amount = min(export.amount, info.free_capacity);
                    export_requested -= amount;
                    export.amount -= amount;
                    if export.amount > 0 {
                        self.exports.push(export);
                    }
                    Task::Pickup(id)
                }
                (None, None) => continue,
                (Some(_), None) => unreachable(),
            };

            match task_type {
                TaskType::Hauler => alloc.hauler[id] = Some(task),
                TaskType::Import => alloc.import[id] = Some(task),
            }
        }

        alloc
    }
}

impl HaulerTaskHandle {
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<Task> {
        transport.hauler[self.0 as usize]
    }
}

impl ImportTaskHandle {
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<Task> {
        transport.import[self.0 as usize]
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
