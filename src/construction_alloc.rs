use std::collections::BTreeMap;

use enum_dispatch::enum_dispatch;
use num::Rational32;
use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Position;
use screeps::REPAIR_POWER;
use screeps::ResourceType;
use screeps::Structure;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::StructureRampart;
use screeps::StructureRoad;
use screeps::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use crate::room::RoleCount;

#[derive(Debug)]
pub struct ConstructionAllocator {
    creeps: BTreeMap<ObjectId<Creep>, CreepInfo>,
    sites: Vec<SiteBasicInfo>,
    repairables: Vec<RepairBasicInfo>,
}

#[derive(Debug)]
struct CreepInfo {
    pos: Position,
    energy: u32,
    target: Option<ConstructionTask>,
}

#[derive(Debug, Clone)]
struct SiteBasicInfo {
    id: ObjectId<ConstructionSite>,
    pos: Position,
    request_energy: u32,
}

#[derive(Debug, Clone)]
struct RepairBasicInfo {
    id: RepairableObjectId,
    pos: Position,
    hit_ratio: Rational32,
    request_energy: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "task", content = "id")]
pub enum ConstructionTask {
    Build(ObjectId<ConstructionSite>),
    Repair(RepairableObjectId),
}

#[derive(Debug, Clone)]
struct TargetInfo {
    task: ConstructionTask,
    pos: Position,
    hit_ratio: Rational32,
    request_energy: u32,
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
                Some(SiteBasicInfo {
                    id: c.try_id()?,
                    pos: c.pos(),
                    request_energy: c.progress_total() - c.progress(),
                })
            })
            .collect();
        let repairables = repairables
            .into_iter()
            .filter_map(|s| match s {
                RepairableObject::Rampart(ref rampart) => {
                    // Repairs up to 25_000_000 hits
                    let hits = rampart.hits();
                    let hit_ratio = Rational32::new(hits as i32, 50_000);
                    let need_fix = hit_ratio < Rational32::new(500, 1);
                    let request_energy = 25_000_000_u32.saturating_sub(hits).div_ceil(REPAIR_POWER);
                    if need_fix {
                        Some((s, hit_ratio, request_energy))
                    } else {
                        None
                    }
                }
                _ => {
                    let hits = s.hits();
                    let hits_max = s.hits_max();
                    let hit_ratio = Rational32::new(hits as i32, hits_max as i32);
                    let target_ratio = Rational32::new(9, 10);
                    let need_fix = hit_ratio < target_ratio;
                    let target_hits = (target_ratio * hits_max as i32).ceil().to_integer() as u32;
                    let request_energy = target_hits.saturating_sub(hits).div_ceil(REPAIR_POWER);
                    if need_fix {
                        Some((s, hit_ratio, request_energy))
                    } else {
                        None
                    }
                }
            })
            .map(|(s, hit_ratio, request_energy)| RepairBasicInfo {
                id: s.id(),
                pos: s.pos(),
                hit_ratio,
                request_energy,
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
        let info = CreepInfo {
            pos: creep.pos(),
            energy: creep.store().get(ResourceType::Energy).unwrap_or(0),
            target,
        };
        self.creeps.insert(id, info);
    }

    pub fn allocate(&mut self) {
        let build_iter = self.sites.iter().map(|s| TargetInfo {
            task: ConstructionTask::Build(s.id),
            pos: s.pos,
            hit_ratio: Rational32::new(0, 1),
            request_energy: s.request_energy,
        });
        let repair_iter = self.repairables.iter().map(|s| TargetInfo {
            task: ConstructionTask::Repair(s.id),
            pos: s.pos,
            hit_ratio: s.hit_ratio,
            request_energy: s.request_energy,
        });
        let mut targets: Vec<_> = build_iter.chain(repair_iter).collect();

        for info in self.creeps.values() {
            if let Some(old_task) = info.target
                && let Some(target) = targets.iter_mut().find(|target| target.task == old_task)
            {
                target.request_energy = target.request_energy.saturating_sub(info.energy);
            }
        }

        for info in self.creeps.values_mut() {
            if let Some(old_task) = info.target
                && let Some(_) = targets.iter().find(|target| target.task == old_task)
            {
                // retain old task
                continue;
            }

            info.target = targets
                .iter()
                .filter(|target| target.request_energy > 0)
                .cloned()
                .min_by_key(|target| {
                    let is_repair = matches!(target.task, ConstructionTask::Repair(_));
                    let distance = info.pos.get_range_to(target.pos);
                    (is_repair, target.hit_ratio, distance)
                })
                .map(|target| target.task);
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
    Container(StructureContainer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum RepairableObjectId {
    Rampart(ObjectId<StructureRampart>),
    Road(ObjectId<StructureRoad>),
    Container(ObjectId<StructureContainer>),
}

impl RepairableObject {
    fn id(&self) -> RepairableObjectId {
        match self {
            Self::Rampart(rampart) => RepairableObjectId::Rampart(rampart.id()),
            Self::Road(road) => RepairableObjectId::Road(road.id()),
            Self::Container(container) => RepairableObjectId::Container(container.id()),
        }
    }
}

impl RepairableObjectId {
    pub fn resolve(&self) -> Option<RepairableObject> {
        Some(match self {
            Self::Rampart(id) => RepairableObject::Rampart(id.resolve()?),
            Self::Road(id) => RepairableObject::Road(id.resolve()?),
            Self::Container(id) => RepairableObject::Container(id.resolve()?),
        })
    }
}

impl Repairable for RepairableObject {}

impl HasPosition for RepairableObject {
    fn pos(&self) -> Position {
        match self {
            Self::Rampart(rampart) => rampart.pos(),
            Self::Road(road) => road.pos(),
            Self::Container(container) => container.pos(),
        }
    }
}

impl AsRef<Structure> for RepairableObject {
    fn as_ref(&self) -> &Structure {
        match self {
            Self::Rampart(rampart) => rampart.as_ref(),
            Self::Road(road) => road.as_ref(),
            Self::Container(container) => container.as_ref(),
        }
    }
}

impl TryFrom<StructureObject> for RepairableObject {
    type Error = ();

    fn try_from(value: StructureObject) -> Result<Self, Self::Error> {
        match value {
            StructureObject::StructureRampart(rampart) => Ok(Self::Rampart(rampart)),
            StructureObject::StructureRoad(road) => Ok(Self::Road(road)),
            StructureObject::StructureContainer(container) => Ok(Self::Container(container)),
            _ => Err(()),
        }
    }
}
