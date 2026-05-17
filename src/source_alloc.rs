use std::collections::BTreeMap;

use screeps::{Creep, HasId, MaybeHasId, ObjectId, Part, Source};

use crate::roles::harvester::HarvesterMemory;

#[derive(Debug)]
pub struct SourceAllocator {
    creeps: BTreeMap<ObjectId<Creep>, Info>,
    sources: Vec<ObjectId<Source>>,
}

#[derive(Debug)]
struct Info {
    target: Option<ObjectId<Source>>,
    size: u8,
}

const SLOTS_PER_SOURCE: u8 = 5;

impl SourceAllocator {
    pub fn new(sources: Vec<Source>) -> Self {
        SourceAllocator {
            creeps: BTreeMap::new(),
            sources: sources.into_iter().map(|s| s.id()).collect(),
        }
    }

    pub fn register(&mut self, creep: Creep, memory: &HarvesterMemory) {
        if let Some(id) = creep.try_id() {
            let info = Info {
                target: memory.get_target(),
                size: creep
                    .body()
                    .into_iter()
                    .filter(|p| p.part() == Part::Work)
                    .count() as u8,
            };
            self.creeps.insert(id, info);
        }
    }

    pub fn allocate(&mut self) -> u8 {
        if self.sources.is_empty() {
            return 0;
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
        unbound.sort_unstable_by_key(|(_, size)| -(*size as i8));

        for (creep, size) in unbound {
            allocs.sort_unstable_by_key(|(_, slot)| *slot);
            let (source_id, alloc) = allocs[0];
            if alloc >= SLOTS_PER_SOURCE {
                break;
            }
            self.creeps.get_mut(&creep).unwrap().target = Some(source_id);
            allocs[0].1 += size;
        }

        allocs.sort_unstable_by_key(|(_, slot)| *slot);
        SLOTS_PER_SOURCE.saturating_sub(allocs[0].1)
    }

    pub fn delegate(&self, creep: &Creep) -> Option<ObjectId<Source>> {
        self.creeps.get(&creep.try_id()?)?.target
    }
}
