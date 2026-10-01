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
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::KeyCmp;

pub struct TransportAllocator {
    haulers: HashMap<ObjectId<Creep>, Info>,
    // imports: Vec<ResourceImport>,
    exports: Vec<ResourceExport>,
}

struct Info {
    task: Option<Task>,
    size: u8,
}

impl TransportAllocator {
    pub fn new() -> Self {
        TransportAllocator {
            haulers: HashMap::new(),
            // imports: Vec::new(),
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
        self.haulers.insert(creep_id, Info { task, size });
    }

    pub fn register_export(&mut self, export: impl Into<ResourceExport>) {
        self.exports.push(export.into());
    }

    /*
    pub fn register_import(&mut self, import: impl Into<ResourceImport>) {
        self.imports.push(import.into());
    }
    */

    pub fn allocate(&mut self) {
        let being_served: HashSet<_> = self.haulers.values().filter_map(|info| info.task).collect();
        let mut pending_serve: BinaryHeap<_> = self
            .exports
            .iter()
            .map(|p| (p.id(), p))
            .filter(|(id, _)| !being_served.contains(&Task::Pickup(*id)))
            .filter_map(|(id, p)| {
                Some(KeyCmp {
                    key: p.store().get(ResourceType::Energy)?,
                    value: id,
                })
            })
            .collect();
        let mut idle_haulers: BinaryHeap<_> = self
            .haulers
            .iter()
            .filter(|(_, info)| info.task.is_none())
            .map(|(creep_id, info)| KeyCmp {
                key: info.size,
                value: *creep_id,
            })
            .collect();

        while let Some(KeyCmp {
            key: size,
            value: hauler_id,
        }) = idle_haulers.pop()
            && let Some(KeyCmp {
                key: energy,
                value: store_id,
            }) = pending_serve.pop()
        {
            let task = Task::Pickup(store_id);
            self.haulers.get_mut(&hauler_id).unwrap().task = Some(task);
            let carriable = size as u32 * CARRY_CAPACITY;
            if carriable < energy {
                pending_serve.push(KeyCmp {
                    key: energy - carriable,
                    value: store_id,
                });
            }
        }
    }

    pub fn delegate(&self, creep: &Creep) -> Option<Task> {
        self.haulers
            .get(&creep.try_id()?)
            .and_then(|info| info.task)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task")]
pub enum Task {
    Pickup(ResourceExportId),
    // Supply,
}

#[allow(unused)]
#[enum_dispatch(ResourceExport)]
trait ResourceExporter {}

/*
#[allow(unused)]
#[enum_dispatch(ResourceImport)]
trait ResourceImporter {}
*/

// Contract: All variant must implement `Withdrawable`
#[enum_dispatch]
pub enum ResourceExport {
    Container(StructureContainer),
}

/*
// Contract: All variant must implement `Transferable`
#[enum_dispatch]
pub enum ResourceImport {
    Tower(StructureTower),
}
*/

impl ResourceExport {
    fn id(&self) -> ResourceExportId {
        let Self::Container(container) = self;
        ResourceExportId::Container(container.id())
    }
}

impl HasStore for ResourceExport {
    fn store(&self) -> Store {
        let Self::Container(container) = self;
        container.store()
    }
}

impl Withdrawable for ResourceExport {}

impl AsRef<RoomObject> for ResourceExport {
    fn as_ref(&self) -> &RoomObject {
        let ResourceExport::Container(container) = self;
        container.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum ResourceExportId {
    Container(ObjectId<StructureContainer>),
}

impl ResourceExportId {
    pub fn resolve(&self) -> Option<ResourceExport> {
        Some(match self {
            ResourceExportId::Container(container) => {
                ResourceExport::Container(container.resolve()?)
            }
        })
    }
}
