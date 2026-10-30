use std::collections::BTreeMap;

use enum_dispatch::enum_dispatch;
use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Position;
use screeps::Structure;
use screeps::StructureObject;
use screeps::StructureRampart;
use screeps::StructureRoad;
use screeps::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::room::RoleCount;
use crate::utils::Fraction;

#[derive(Debug)]
pub struct ConstructionAllocator {
    creeps: BTreeMap<ObjectId<Creep>, Info>,
    sites: Vec<SiteBasicInfo>,
    repairables: Vec<RepairBasicInfo>,
}

#[derive(Debug)]
struct Info {
    pos: Position,
    target: Option<ConstructionTask>,
}

#[derive(Debug, Clone)]
struct SiteBasicInfo {
    id: ObjectId<ConstructionSite>,
    pos: Position,
}

#[derive(Debug, Clone)]
struct RepairBasicInfo {
    id: RepairableObjectId,
    pos: Position,
    hit_ratio: Fraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task", content = "id")]
pub enum ConstructionTask {
    Build(ObjectId<ConstructionSite>),
    Repair(RepairableObjectId),
}

impl ConstructionAllocator {
    pub fn new(
        construction_sites: Vec<ConstructionSite>,
        repairables: Vec<RepairableObject>,
    ) -> Self {
        let creeps = BTreeMap::new();
        let sites = construction_sites
            .into_iter()
            .filter_map(|c| {
                let id = c.try_id()?;
                let pos = c.pos();
                Some(SiteBasicInfo { id, pos })
            })
            .collect();
        let repairables = repairables
            .into_iter()
            .filter_map(|s| match s {
                RepairableObject::Rampart(ref rampart) => {
                    // Repairs up to 25_000_000 hits
                    let hit_ratio = Fraction(rampart.hits(), 50_000);
                    let need_fix = hit_ratio < Fraction(500, 1);
                    if need_fix { Some((s, hit_ratio)) } else { None }
                }
                _ => {
                    let hit_ratio = Fraction(s.hits(), s.hits_max());
                    let need_fix = hit_ratio < Fraction(9, 10);
                    if need_fix { Some((s, hit_ratio)) } else { None }
                }
            })
            .map(|(s, hit_ratio)| RepairBasicInfo {
                id: s.id(),
                pos: s.pos(),
                hit_ratio,
            })
            .collect();
        ConstructionAllocator {
            creeps,
            sites,
            repairables,
        }
    }

    pub fn register_builder(&mut self, creep: &Creep, target: Option<ConstructionTask>) {
        let Some(id) = creep.try_id() else { return };
        let pos = creep.pos();
        let info = Info { pos, target };
        self.creeps.insert(id, info);
    }

    pub fn allocate(&mut self) {
        let build_iter = self
            .sites
            .iter()
            .map(|s| (ConstructionTask::Build(s.id), s.pos, Fraction(0, 1)));
        let repair_iter = self
            .repairables
            .iter()
            .map(|s| (ConstructionTask::Repair(s.id), s.pos, s.hit_ratio));
        let targets: Vec<_> = build_iter.chain(repair_iter).collect();

        for info in self.creeps.values_mut() {
            if let Some(old_task) = info.target
                && targets
                    .iter()
                    .find(|(task, _, _)| *task == old_task)
                    .is_some()
            {
                // retain old task
                continue;
            }

            info.target = targets
                .iter()
                .cloned()
                .min_by_key(|(task, pos, hit_ratio)| {
                    let is_repair = matches!(task, ConstructionTask::Repair(_));
                    let distance = info.pos.get_range_to(*pos);
                    (is_repair, *hit_ratio, distance)
                })
                .map(|(task, _, _)| task);
        }
    }

    pub fn delegate(&self, creep: &Creep) -> Option<ConstructionTask> {
        let id = creep.try_id()?;
        self.creeps.get(&id).and_then(|info| info.target)
    }

    pub fn get_creep_spawn_info(&self, role_count: &RoleCount) -> bool {
        role_count.builders < 2
            && role_count.upgraders != 0
            && !(self.repairables.is_empty() && self.sites.is_empty())
    }
}

#[allow(unused)]
#[enum_dispatch(RepairableObjects)]
trait RepairableObjectTrait {}

// Contract: All variant must implement `Repairable`
#[enum_dispatch]
pub enum RepairableObject {
    Rampart(StructureRampart),
    Road(StructureRoad),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum RepairableObjectId {
    Rampart(ObjectId<StructureRampart>),
    Road(ObjectId<StructureRoad>),
}

impl RepairableObject {
    fn id(&self) -> RepairableObjectId {
        match self {
            Self::Rampart(rampart) => RepairableObjectId::Rampart(rampart.id()),
            Self::Road(road) => RepairableObjectId::Road(road.id()),
        }
    }
}

impl RepairableObjectId {
    pub fn resolve(&self) -> Option<RepairableObject> {
        Some(match self {
            Self::Rampart(id) => RepairableObject::Rampart(id.resolve()?),
            Self::Road(id) => RepairableObject::Road(id.resolve()?),
        })
    }
}

impl Repairable for RepairableObject {}

impl HasPosition for RepairableObject {
    fn pos(&self) -> Position {
        match self {
            Self::Rampart(rampart) => rampart.pos(),
            Self::Road(road) => road.pos(),
        }
    }
}

impl AsRef<Structure> for RepairableObject {
    fn as_ref(&self) -> &Structure {
        match self {
            Self::Rampart(rampart) => rampart.as_ref(),
            Self::Road(road) => road.as_ref(),
        }
    }
}

impl TryFrom<StructureObject> for RepairableObject {
    type Error = ();

    fn try_from(value: StructureObject) -> Result<Self, Self::Error> {
        match value {
            StructureObject::StructureRampart(rampart) => Ok(Self::Rampart(rampart)),
            StructureObject::StructureRoad(road) => Ok(Self::Road(road)),
            _ => Err(()),
        }
    }
}
