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
    id_counter: u16,
    creeps: Vec<Info>,
    imports: Vec<ImportInfo>,
    exports: Vec<ExportInfo>,
}

pub struct TransportAllocation {
    alloc: Vec<Option<Task>>,
}

#[derive(Debug)]
pub struct BidirTaskHandle(u16);
#[derive(Debug)]
pub struct ImportTaskHandle(u16);
#[derive(Debug)]
pub struct ExportTaskHandle(u16);

enum TaskType {
    Bidir,
    Import,
    Export,
}

struct Info {
    task: Option<Task>,
    size: u8,
    used_capacity: u32,
    free_capacity: u32,
    pos: Position,
    spawning: bool,
    task_type: TaskType,
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
    pub fn register_hauler(&mut self, creep: &Creep, task: Option<Task>) -> BidirTaskHandle {
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

        BidirTaskHandle(self.next_id())
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_import(&mut self, creep: &Creep, task: Option<Task>) -> ImportTaskHandle {
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

        ImportTaskHandle(self.next_id())
    }

    #[must_use = "Task handle dropped before used"]
    pub fn register_creep_export(&mut self, creep: &Creep, task: Option<Task>) -> ExportTaskHandle {
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

        ExportTaskHandle(self.next_id())
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

    pub fn allocate(mut self) -> TransportAllocation {
        let mut alloc = vec![None; self.id_counter as usize];

        // Remove corresponded amount already queued
        for info in &self.creeps {
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

        // exports
        let creep_export: u32 = self
            .creeps
            .iter()
            .filter(|info| matches!(info.task_type, TaskType::Export))
            .map(|info| info.used_capacity)
            .sum();
        let creep_bidir: u32 = self
            .creeps
            .iter()
            .filter(|info| matches!(info.task_type, TaskType::Bidir))
            .map(|info| info.used_capacity)
            .sum();
        let structure_export: u32 = self.exports.iter().map(|info| info.amount).sum();
        let structure_export_active: u32 = self
            .exports
            .iter()
            .filter(|info| info.priority != Priority::Passive)
            .map(|info| info.amount)
            .sum();

        // imports
        let creep_import: u32 = self
            .creeps
            .iter()
            .filter(|info| matches!(info.task_type, TaskType::Import))
            .map(|info| info.free_capacity)
            .sum();
        let structure_import: u32 = self.imports.iter().map(|info| info.amount).sum();
        let structure_import_active: u32 = self
            .imports
            .iter()
            .filter(|info| info.priority != Priority::Passive)
            .map(|info| info.amount)
            .sum();

        // sum of all network flow
        let total_export = creep_export + creep_bidir + structure_export;
        let active_export = creep_export + creep_bidir + structure_export_active;
        let total_import = creep_import + structure_import;
        let active_import = creep_import + structure_import_active;

        let network_cap = min(total_export, total_import);
        let mut export_requested = min(network_cap, max(structure_export_active, active_import));
        let mut import_requested = min(network_cap, max(active_export, structure_import_active));

        let infos: PriorityQueue<_, (usize, &Info)> = self
            .creeps
            .iter()
            .enumerate()
            .map(|(id, info)| {
                let single_sided = matches!(info.task_type, TaskType::Import | TaskType::Export);
                ((single_sided, info.size), (id, info))
            })
            .collect();

        // dispatch task for creeps without a task
        for (_, (id, info)) in infos {
            if info.task.is_some() || info.spawning {
                continue;
            }

            let can_import = matches!(info.task_type, TaskType::Bidir | TaskType::Export);
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

            let can_export = matches!(info.task_type, TaskType::Bidir | TaskType::Import);
            let export_candidate = if can_export && export_requested > 0 {
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

            alloc[id] = Some(task);
        }

        TransportAllocation { alloc }
    }
}

impl BidirTaskHandle {
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<Task> {
        transport.alloc[self.0 as usize]
    }
}

impl ImportTaskHandle {
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<Task> {
        transport.alloc[self.0 as usize]
    }
}

impl ExportTaskHandle {
    #[allow(unused)]
    pub fn resolve(&self, transport: &TransportAllocation) -> Option<Task> {
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
