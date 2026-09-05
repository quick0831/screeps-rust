use std::cmp::min;
use std::collections::BTreeMap;

use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::Source;
use screeps::prelude::*;

#[derive(Debug)]
pub struct SourceAllocator {
    creeps: BTreeMap<ObjectId<Creep>, Info>,
    sources: Vec<ObjectId<Source>>,
    creep_spawn_size: u8,
}

#[derive(Debug)]
struct Info {
    target: Option<ObjectId<Source>>,
    size: u8,
}

// required work part per source
// 0.5 is a rough estimate of deposit time loss
// ceil(3000 / 300 / 2 + 0.5) = 6
const SLOTS_PER_SOURCE: u8 = 6;

impl SourceAllocator {
    pub fn new(sources: Vec<Source>) -> Self {
        SourceAllocator {
            creeps: BTreeMap::new(),
            sources: sources.into_iter().map(|s| s.id()).collect(),
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

        let mut allocs: Vec<(_, u8)> = self.sources.iter().cloned().map(|s| (s, 0)).collect();
        for info in self.creeps.values() {
            if let Some(target) = &info.target
                && let Some(e) = allocs.iter_mut().find(|(s, _)| *s == *target)
            {
                e.1 += info.size;
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
            allocs.sort_unstable_by_key(|(_, slot)| *slot);
            let (source_id, alloc) = allocs[0];
            if alloc >= SLOTS_PER_SOURCE {
                break;
            }
            self.creeps.get_mut(&creep).unwrap().target = Some(source_id);
            allocs[0].1 += size;
        }

        let max_creep_size = self
            .creeps
            .values()
            .map(|info| info.size)
            .max()
            .unwrap_or(0);

        allocs.sort_unstable_by_key(|(_, slot)| *slot);
        let spawn_size = SLOTS_PER_SOURCE.saturating_sub(allocs[0].1);

        self.creep_spawn_size = min(spawn_size, max_creep_size + 1);
    }

    pub fn delegate(&self, creep: &Creep) -> Option<ObjectId<Source>> {
        self.creeps.get(&creep.try_id()?)?.target
    }

    pub fn get_creep_spawn_size(&self) -> u8 {
        self.creep_spawn_size
    }
}
