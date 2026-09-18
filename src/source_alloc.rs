use std::cmp::min;
use std::collections::BTreeMap;

use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::Source;
use screeps::prelude::*;

use crate::source::SourceInfo;

#[derive(Debug)]
pub struct SourceAllocator {
    creeps: BTreeMap<ObjectId<Creep>, Info>,
    sources: Vec<SourceBasicInfo>,
}

#[derive(Debug)]
struct Info {
    size: u8,
}

#[derive(Debug, Clone)]
struct SourceBasicInfo {
    id: ObjectId<Source>,
    site: u8,
    alloced_site: u8,
    alloced_size: u8,
}

// required work part per source
// 0.5 is a rough estimate of deposit time loss
// ceil(3000 / 300 / 2 + 0.5) = 6
const SLOTS_PER_SOURCE: u8 = 6;

impl SourceAllocator {
    pub fn new(sources: &[SourceInfo]) -> Self {
        SourceAllocator {
            creeps: BTreeMap::new(),
            sources: sources
                .iter()
                .map(|s| SourceBasicInfo {
                    id: s.source.id(),
                    site: s.site,
                    alloced_site: 0,
                    alloced_size: 0,
                })
                .collect(),
        }
    }

    pub fn register_harvester(&mut self, creep: &Creep, target: ObjectId<Source>) {
        let Some(id) = creep.try_id() else { return };
        let size = creep
            .body()
            .into_iter()
            .filter(|p| p.part() == Part::Work)
            .count() as u8;
        let info = Info { size };
        self.creeps.insert(id, info);

        if let Some(e) = self.sources.iter_mut().find(|s| s.id == target) {
            e.alloced_site += 1;
            e.alloced_size += size;
        }
    }

    pub fn get_creep_spawn_info(&self) -> Option<(ObjectId<Source>, u8)> {
        let max_creep_size = self
            .creeps
            .values()
            .map(|info| info.size)
            .max()
            .unwrap_or(0);

        let spawn_target = self
            .sources
            .iter()
            .filter(|s| s.alloced_site < s.site)
            .min_by_key(|s| s.alloced_size)?;

        let spawn_size = SLOTS_PER_SOURCE.saturating_sub(spawn_target.alloced_size);
        let spawn_size = min(spawn_size, max_creep_size + 1);

        if spawn_size == 0 {
            return None;
        }

        Some((spawn_target.id, spawn_size))
    }
}
