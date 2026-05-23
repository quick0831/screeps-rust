use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;

use screeps::CARRY_CAPACITY;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::Position;
use screeps::ResourceType;
use screeps::RoomObject;
use screeps::Store;
use screeps::StructureContainer;
use screeps::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::KeyCmp;

pub struct TransportAllocator {
    providers: Vec<EnergyStore>,
    haulers: HashMap<ObjectId<Creep>, Info>,
}

struct Info {
    target: Option<EnergyStoreId>,
    size: u8,
}

impl TransportAllocator {
    pub fn new() -> Self {
        TransportAllocator {
            providers: Vec::new(),
            haulers: HashMap::new(),
        }
    }

    pub fn register_hauler(&mut self, creep: &Creep, target: Option<EnergyStoreId>) {
        let Some(creep_id) = creep.try_id() else {
            return;
        };
        let size = creep
            .body()
            .into_iter()
            .map(|p| p.part())
            .filter(|p| *p == Part::Work)
            .count() as u8;
        self.haulers.insert(creep_id, Info { target, size });
    }

    pub fn file_request(&mut self, provider: EnergyStore) {
        self.providers.push(provider);
    }

    pub fn allocate(&mut self) {
        let being_served: HashSet<_> = self
            .haulers
            .iter()
            .filter_map(|(_, info)| info.target)
            .collect();
        let mut pending_serve: BinaryHeap<_> = self
            .providers
            .iter()
            .filter_map(|p| Some((p.id()?, p)))
            .filter(|(id, _)| !being_served.contains(id))
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
            .filter(|(_, info)| info.target.is_none())
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
            self.haulers.get_mut(&hauler_id).unwrap().target = Some(store_id);
            let carriable = size as u32 * CARRY_CAPACITY;
            if carriable < energy {
                pending_serve.push(KeyCmp {
                    key: energy - carriable,
                    value: store_id,
                });
            }
        }
    }

    pub fn delegate(&self, creep: &Creep) -> Option<EnergyStoreId> {
        self.haulers
            .get(&creep.try_id()?)
            .and_then(|info| info.target)
    }
}

#[derive(Debug, Clone)]
pub enum EnergyStore {
    Creep(Creep),
    Container(StructureContainer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum EnergyStoreId {
    Creep(ObjectId<Creep>),
    Container(ObjectId<StructureContainer>),
}

impl HasPosition for EnergyStore {
    fn pos(&self) -> Position {
        match self {
            EnergyStore::Creep(creep) => creep.pos(),
            EnergyStore::Container(container) => container.pos(),
        }
    }
}

impl EnergyStore {
    pub fn id(&self) -> Option<EnergyStoreId> {
        Some(match self {
            EnergyStore::Creep(creep) => EnergyStoreId::Creep(creep.try_id()?),
            EnergyStore::Container(container) => EnergyStoreId::Container(container.id()),
        })
    }

    pub fn store(&self) -> Store {
        match self {
            EnergyStore::Creep(creep) => creep.store(),
            EnergyStore::Container(container) => container.store(),
        }
    }

    pub fn as_withdrawable(&self) -> Option<impl Withdrawable> {
        struct WithdrawableRoomObject(RoomObject);

        impl Withdrawable for WithdrawableRoomObject {}

        impl AsRef<RoomObject> for WithdrawableRoomObject {
            fn as_ref(&self) -> &RoomObject {
                &self.0
            }
        }

        // Contract: The types must implement `Withdrawable`
        let room_object: &RoomObject = match self {
            EnergyStore::Creep(_) => return None,
            EnergyStore::Container(container) => container,
        };
        Some(WithdrawableRoomObject(room_object.clone()))
    }
}

impl EnergyStoreId {
    pub fn resolve(&self) -> Option<EnergyStore> {
        Some(match self {
            EnergyStoreId::Creep(creep) => EnergyStore::Creep(creep.resolve()?),
            EnergyStoreId::Container(container) => EnergyStore::Container(container.resolve()?),
        })
    }
}
