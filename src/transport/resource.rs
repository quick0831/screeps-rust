use enum_dispatch::enum_dispatch;
use screeps::ObjectId;
use screeps::RoomObject;
use screeps::Store;
use screeps::StructureContainer;
use screeps::StructureExtension;
use screeps::StructureSpawn;
use screeps::StructureStorage;
use screeps::StructureTower;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused)]
#[enum_dispatch(ResourceExport)]
trait ResourceExporter {}

#[allow(unused)]
#[enum_dispatch(ResourceImport)]
trait ResourceImporter {}

// Contract: All variant must implement `Withdrawable`
#[derive(Debug)]
#[enum_dispatch]
pub enum ResourceExport {
    Container(StructureContainer),
    Spawn(StructureSpawn),
    Extension(StructureExtension),
    Storage(StructureStorage),
}

// Contract: All variant must implement `Transferable`
#[derive(Debug)]
#[enum_dispatch]
pub enum ResourceImport {
    Spawn(StructureSpawn),
    Extension(StructureExtension),
    Tower(StructureTower),
    Storage(StructureStorage),
}

impl ResourceExport {
    pub fn id(&self) -> ResourceExportId {
        match self {
            Self::Container(container) => ResourceExportId::Container(container.id()),
            Self::Spawn(spawn) => ResourceExportId::Spawn(spawn.id()),
            Self::Extension(extension) => ResourceExportId::Extension(extension.id()),
            Self::Storage(storage) => ResourceExportId::Storage(storage.id()),
        }
    }
}

impl ResourceImport {
    pub fn id(&self) -> ResourceImportId {
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
