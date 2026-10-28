use std::cmp::min;

use screeps::Creep;
use screeps::ObjectId;
use screeps::Part;
use screeps::Source;
use screeps::prelude::*;

use crate::source::SourceInfo;
use crate::utils::CreepSizePlanner;

#[derive(Debug)]
pub struct SourceAllocator {
    sources: Vec<SourceBasicInfo>,
}

#[derive(Debug, Clone)]
struct SourceBasicInfo {
    id: ObjectId<Source>,
    site: u8,
    alloced_sizes: Vec<u8>,
}

// required work part per source
// 0.5 is a rough estimate of deposit time loss
// ceil(3000 / 300 / 2 + 0.5) = 6
const SLOTS_PER_SOURCE: u8 = 6;

impl SourceAllocator {
    pub fn new(sources: &[SourceInfo]) -> Self {
        SourceAllocator {
            sources: sources
                .iter()
                .map(|s| SourceBasicInfo {
                    id: s.source.id(),
                    site: s.site,
                    alloced_sizes: Vec::new(),
                })
                .collect(),
        }
    }

    pub fn register_harvester(&mut self, creep: &Creep, target: ObjectId<Source>) {
        let size = creep
            .body()
            .into_iter()
            .filter(|p| p.part() == Part::Work)
            .count() as u8;

        if let Some(e) = self.sources.iter_mut().find(|s| s.id == target) {
            e.alloced_sizes.push(size);
        }
    }

    pub fn get_creep_spawn_info(&self, spawn_cap: u8) -> Option<(ObjectId<Source>, u8)> {
        let spawn_target = self
            .sources
            .iter()
            .filter(|s| (s.alloced_sizes.len() as u8) < s.site)
            .min_by_key(|s| s.alloced_sizes.len())?;

        let mut spawn_total = SLOTS_PER_SOURCE;

        let mut spawn_plan = Vec::new();
        while spawn_total > 0 {
            let size = min(spawn_total, spawn_cap);
            spawn_total -= size;
            spawn_plan.push(size);
        }

        let plan = CreepSizePlanner {
            current_sizes: &spawn_target.alloced_sizes,
            target_plan: &spawn_plan,
            max_creep_count: spawn_target.site,
        };

        let spawn_size = plan.solve()?;

        Some((spawn_target.id, spawn_size))
    }
}
