use std::collections::BTreeMap;

use screeps::CostMatrix;
use screeps::Creep;
use screeps::ObjectId;
use screeps::PolyStyle;
use screeps::Position;
use screeps::RoomName;
use screeps::find;
use screeps::game;
use screeps::pathfinder::MultiRoomCostResult;
use screeps::pathfinder::SearchOptions;
use screeps::pathfinder::search;
use screeps::prelude::*;

const COST_UNWALKABLE: u8 = 255;

pub struct PathFinder {
    creeps: BTreeMap<ObjectId<Creep>, (PathType, Position)>,
}

enum PathType {
    NoTarget,
    MoveTo(Position, u32),
    MoveAway(Position, u32),
}

impl PathFinder {
    pub fn new<'a>(creeps_iter: impl IntoIterator<Item = &'a Creep>) -> Self {
        let mut creeps = BTreeMap::new();
        for creep in creeps_iter {
            let Some(id) = creep.try_id() else {
                continue;
            };
            creeps.insert(id, (PathType::NoTarget, creep.pos()));
        }

        Self { creeps }
    }

    /// Note:
    /// If the target is not walkable, set the range to at least 1 to avoid wasting CPU
    pub fn move_to(&mut self, creep: &Creep, target: impl HasPosition, range: u32) {
        let target_pos = target.pos();
        if creep.pos().get_range_to(target_pos) <= range {
            return;
        }
        let id = creep.try_id();
        let Some(id) = id else { return };
        let r = self.creeps.get_mut(&id);
        let Some(r) = r else { return };
        r.0 = PathType::MoveTo(target_pos, range);
    }

    pub fn move_away_from(&mut self, creep: &Creep, target: impl HasPosition, range: u32) {
        let target_pos = target.pos();
        if creep.pos().get_range_to(target_pos) >= range {
            return;
        }
        let id = creep.try_id();
        let Some(id) = id else { return };
        let r = self.creeps.get_mut(&id);
        let Some(r) = r else { return };
        r.0 = PathType::MoveAway(target_pos, range);
    }

    pub fn process_movements(&self) {
        let stasis_creeps: Vec<_> = self
            .creeps
            .iter()
            .filter(|(_, (p, _))| matches!(p, PathType::NoTarget))
            .map(|(_, (_, p))| p)
            .cloned()
            .collect();

        let all_rooms = game::rooms();
        let mut cache: BTreeMap<RoomName, MultiRoomCostResult> = BTreeMap::new();

        let mut get_costmatrix = move |room_name: RoomName| {
            if let Some(cache_result) = cache.get(&room_name) {
                return clone_result(cache_result);
            }

            let Some(room) = all_rooms.get(room_name) else {
                cache.insert(room_name, MultiRoomCostResult::Default);
                return MultiRoomCostResult::Default;
            };

            let cost_matrix = CostMatrix::new();

            let structure_pos = room
                .find(find::STRUCTURES, None)
                .into_iter()
                .filter(|s| s.structure_type().is_obstacle())
                .map(|s| s.pos());
            let construction_site_pos = room
                .find(find::CONSTRUCTION_SITES, None)
                .into_iter()
                .filter(|s| s.structure_type().is_obstacle())
                .map(|s| s.pos());
            for pos in stasis_creeps
                .iter()
                .cloned()
                .chain(structure_pos)
                .chain(construction_site_pos)
            {
                let (x, y) = pos.coords();
                cost_matrix.set(x, y, COST_UNWALKABLE);
            }
            let result = MultiRoomCostResult::CostMatrix(cost_matrix);
            cache.insert(room_name, clone_result(&result));
            result
        };

        let mut idx = 0;
        for (id, (target, pos)) in self.creeps.iter() {
            let result = match target {
                PathType::MoveTo(target, range) => {
                    let options = SearchOptions::new(&mut get_costmatrix).max_rooms(1);
                    search(*pos, *target, *range, Some(options))
                }
                PathType::MoveAway(target, range) => {
                    let options = SearchOptions::new(&mut get_costmatrix)
                        .flee(true)
                        .max_rooms(1)
                        .max_ops(100);
                    search(*pos, *target, *range, Some(options))
                }
                PathType::NoTarget => continue,
            };
            let creep = id.resolve();
            let Some(creep) = creep else { continue };
            let _ = creep.move_by_path(&result.opaque_path());

            if let Some(room) = creep.room() {
                let visual = room.visual();

                let offset = if idx % 2 == 0 { idx / 2 } else { -idx / 2 } as f32 * 0.05;
                let points = Some(*pos)
                    .into_iter()
                    .chain(result.path())
                    .map(|p| {
                        let (x, y) = p.coords();
                        (x as f32 + offset, y as f32 + offset)
                    })
                    .collect();
                let style = PolyStyle::default()
                    .opacity(0.7)
                    .stroke("#ffffff")
                    .stroke_width(0.02);
                visual.poly(points, Some(style));
            }

            idx += 1;
        }
    }
}

/// manually cloning the result because `MultiRoomCostResult` is somehow not `Clone`
fn clone_result(input: &MultiRoomCostResult) -> MultiRoomCostResult {
    match input {
        MultiRoomCostResult::CostMatrix(cm) => MultiRoomCostResult::CostMatrix(cm.clone()),
        MultiRoomCostResult::Impassable => MultiRoomCostResult::Impassable,
        MultiRoomCostResult::Default => MultiRoomCostResult::Default,
    }
}
