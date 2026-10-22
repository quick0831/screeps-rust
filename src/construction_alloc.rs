use std::collections::BTreeMap;

use screeps::ConstructionSite;
use screeps::Creep;
use screeps::ObjectId;
use screeps::Position;
use screeps::prelude::*;

#[derive(Debug)]
pub struct ConstructionAllocator {
    creeps: BTreeMap<ObjectId<Creep>, Info>,
    sites: Vec<SiteBasicInfo>,
}

#[derive(Debug)]
struct Info {
    pos: Position,
    target: Option<ObjectId<ConstructionSite>>,
}

#[derive(Debug, Clone)]
struct SiteBasicInfo {
    id: ObjectId<ConstructionSite>,
    pos: Position,
}

impl ConstructionAllocator {
    pub fn new(construction_sites: Vec<ConstructionSite>) -> Self {
        let creeps = BTreeMap::new();
        let sites = construction_sites
            .into_iter()
            .filter_map(|c| {
                let id = c.try_id()?;
                let pos = c.pos();
                Some(SiteBasicInfo { id, pos })
            })
            .collect();
        ConstructionAllocator { creeps, sites }
    }

    pub fn register_builder(&mut self, creep: &Creep, target: Option<ObjectId<ConstructionSite>>) {
        let Some(id) = creep.try_id() else { return };
        let pos = creep.pos();
        let info = Info { pos, target };
        self.creeps.insert(id, info);
    }

    pub fn allocate(&mut self) {
        for info in self.creeps.values_mut() {
            info.target = self
                .sites
                .iter()
                .min_by_key(|site| info.pos.get_range_to(site.pos))
                .map(|site| site.id);
        }
    }

    pub fn delegate(&self, creep: &Creep) -> Option<ObjectId<ConstructionSite>> {
        let id = creep.try_id()?;
        self.creeps.get(&id).and_then(|info| info.target)
    }
}
