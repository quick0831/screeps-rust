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
    creep_spawn_size: u8,
}

#[derive(Debug)]
struct Info {
    target: Option<ObjectId<Source>>,
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
                    site: s.nearby_area.len() as u8,
                    alloced_site: 0,
                    alloced_size: 0,
                })
                .collect(),
            creep_spawn_size: 0,
        }
    }

    pub fn register_harvester(&mut self, creep: &Creep, target: Option<ObjectId<Source>>) {
        let Some(id) = creep.try_id() else { return };
        let size = creep
            .body()
            .into_iter()
            .filter(|p| p.part() == Part::Work)
            .count() as u8;
        let info = Info { target, size };
        self.creeps.insert(id, info);
    }

    pub fn allocate(&mut self) {
        if self.sources.is_empty() {
            return;
        }

        for info in self.creeps.values() {
            if let Some(target) = &info.target
                && let Some(e) = self.sources.iter_mut().find(|s| s.id == *target)
            {
                e.alloced_site += 1;
                e.alloced_size += info.size;
            }
        }

        let mut unbound: Vec<_> = self
            .creeps
            .iter()
            .filter(|(_, info)| info.target.is_none())
            .map(|(creep, info)| (*creep, info.size))
            .collect();
        unbound.sort_unstable_by_key(|(_, size)| *size);

        for (creep, size) in unbound.into_iter().rev() {
            let source_info = self
                .sources
                .iter_mut()
                .filter(|s| s.alloced_size < SLOTS_PER_SOURCE)
                .filter(|s| s.alloced_site < s.site)
                .min_by_key(|s| s.alloced_size);
            let Some(source_info) = source_info else {
                break;
            };
            self.creeps.get_mut(&creep).unwrap().target = Some(source_info.id);
            source_info.alloced_site += 1;
            source_info.alloced_size += size;
        }

        let max_creep_size = self
            .creeps
            .values()
            .map(|info| info.size)
            .max()
            .unwrap_or(0);

        let spawn_need = self.sources.iter().map(|s| s.alloced_size).min();
        let spawn_size = if let Some(n) = spawn_need {
            SLOTS_PER_SOURCE.saturating_sub(n)
        } else {
            0
        };

        self.creep_spawn_size = min(spawn_size, max_creep_size + 1);
    }

    pub fn delegate(&self, creep: &Creep) -> Option<ObjectId<Source>> {
        self.creeps.get(&creep.try_id()?)?.target
    }

    pub fn get_creep_spawn_size(&self) -> u8 {
        self.creep_spawn_size
    }
}
