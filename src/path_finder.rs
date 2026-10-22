use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::collections::hash_map::Entry;
use std::mem::take;

use screeps::Creep;
use screeps::LocalCostMatrix;
use screeps::MOVE_COST_ROAD;
use screeps::ObjectId;
use screeps::PolyStyle;
use screeps::Position;
use screeps::RoomName;
use screeps::StructureType;
use screeps::find;
use screeps::game;
use screeps::pathfinder::MultiRoomCostResult;
use screeps::pathfinder::SearchGoal;
use screeps::pathfinder::SearchOptions;
use screeps::pathfinder::search;
use screeps::pathfinder::search_many;
use screeps::prelude::*;

use crate::cache::Cache;

const COST_UNWALKABLE: u8 = 255;
const CACHE_VALID_TICK: u8 = 20;

static PATH_CACHE: Cache<HashMap<ObjectId<Creep>, CachedPath>> = Cache::new();

pub struct PathFinder {
    creeps: BTreeMap<ObjectId<Creep>, (Option<PathType>, Position)>,
}

enum PathType {
    Toward(Position, u32),
    Away(Position, u32),
    TowardMulti(Vec<SearchGoal>),
    AwayMulti(Vec<SearchGoal>),
}

struct CachedPath {
    valid: u8,
    path: VecDeque<Position>,
}

impl PathFinder {
    pub fn new<'a>(creeps_iter: impl IntoIterator<Item = &'a Creep>) -> Self {
        let mut creeps = BTreeMap::new();
        for creep in creeps_iter {
            let Some(id) = creep.try_id() else {
                continue;
            };
            creeps.insert(id, (None, creep.pos()));
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
        r.0 = Some(PathType::Toward(target_pos, range));
    }

    #[allow(unused)]
    pub fn move_away(&mut self, creep: &Creep, target: impl HasPosition, range: u32) {
        let target_pos = target.pos();
        if creep.pos().get_range_to(target_pos) >= range {
            return;
        }
        let id = creep.try_id();
        let Some(id) = id else { return };
        let r = self.creeps.get_mut(&id);
        let Some(r) = r else { return };
        r.0 = Some(PathType::Away(target_pos, range));
    }

    #[allow(unused)]
    pub fn move_to_multi(&mut self, creep: &Creep, goals: &[SearchGoal]) {
        let creep_pos = creep.pos();
        if goals
            .iter()
            .any(|g| creep_pos.get_range_to(g.pos().pos()) <= g.range())
        {
            return;
        }
        let goals = goals.iter().map(clone_search_goal).collect();
        let id = creep.try_id();
        let Some(id) = id else { return };
        let r = self.creeps.get_mut(&id);
        let Some(r) = r else { return };
        r.0 = Some(PathType::TowardMulti(goals));
    }

    pub fn move_away_multi(&mut self, creep: &Creep, goals: &[SearchGoal]) {
        let creep_pos = creep.pos();
        if goals
            .iter()
            .all(|g| creep_pos.get_range_to(g.pos().pos()) >= g.range())
        {
            return;
        }
        let goals = goals.iter().map(clone_search_goal).collect();
        let id = creep.try_id();
        let Some(id) = id else { return };
        let r = self.creeps.get_mut(&id);
        let Some(r) = r else { return };
        r.0 = Some(PathType::AwayMulti(goals));
    }

    pub fn process_movements(mut self) {
        let stasis_creeps: Vec<_> = self
            .creeps
            .iter()
            .filter(|(_, (p, _))| p.is_none())
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

            let mut cost_matrix = LocalCostMatrix::new();

            let structures = room.find(find::STRUCTURES, None);
            let road_pos = structures
                .iter()
                .filter(|s| s.structure_type() == StructureType::Road)
                .map(|s| s.pos());
            let structure_pos = structures
                .iter()
                .filter(|s| s.structure_type().is_obstacle())
                .map(|s| s.pos());
            let construction_site_pos = room
                .find(find::CONSTRUCTION_SITES, None)
                .into_iter()
                .filter(|s| s.structure_type().is_obstacle())
                .map(|s| s.pos());
            for pos in road_pos {
                cost_matrix.set(pos.xy(), MOVE_COST_ROAD as u8);
            }
            for pos in stasis_creeps
                .iter()
                .cloned()
                .chain(structure_pos)
                .chain(construction_site_pos)
            {
                cost_matrix.set(pos.xy(), COST_UNWALKABLE);
            }
            let result = MultiRoomCostResult::CostMatrix(cost_matrix.into());
            cache.insert(room_name, clone_result(&result));
            result
        };

        let mut call_path_finder = |target, pos| {
            let result = match target {
                PathType::Toward(target, range) => {
                    let options = SearchOptions::new(&mut get_costmatrix).max_rooms(1);
                    search(pos, target, range, Some(options))
                }
                PathType::Away(target, range) => {
                    let options = SearchOptions::new(&mut get_costmatrix)
                        .flee(true)
                        .max_rooms(1)
                        .max_ops(100);
                    search(pos, target, range, Some(options))
                }
                PathType::TowardMulti(goals) => {
                    let options = SearchOptions::new(&mut get_costmatrix).max_rooms(1);
                    search_many(pos, goals.into_iter(), Some(options))
                }
                PathType::AwayMulti(goals) => {
                    let options = SearchOptions::new(&mut get_costmatrix)
                        .flee(true)
                        .max_rooms(1)
                        .max_ops(100);
                    search_many(pos, goals.into_iter(), Some(options))
                }
            };
            VecDeque::from(result.path())
        };

        let mut path_cache = PATH_CACHE.get_or_init(HashMap::new);

        let mut idx = 0;
        for (id, (target, pos)) in take(&mut self.creeps).into_iter() {
            let Some(target) = target else { continue };
            let mut entry = match path_cache.entry(id) {
                Entry::Occupied(mut entry) => {
                    let e = entry.get_mut();
                    if e.valid == 0 {
                        e.valid = CACHE_VALID_TICK;
                        e.path = call_path_finder(target, pos);
                    } else {
                        e.valid -= 1;
                    }
                    entry
                }
                Entry::Vacant(entry) => {
                    let valid = CACHE_VALID_TICK;
                    let path = call_path_finder(target, pos);
                    let value = CachedPath { valid, path };
                    entry.insert_entry(value)
                }
            };
            let path = &mut entry.get_mut().path;

            path.pop_front_if(|p| *p == pos);
            let Some(direction) = path.front().and_then(|p| pos.get_direction_to(*p)) else {
                continue;
            };

            let Some(creep) = id.resolve() else { continue };
            let _ = creep.move_direction(direction);

            if let Some(room) = creep.room() {
                let visual = room.visual();

                let offset = if idx % 2 == 0 { idx / 2 } else { -idx / 2 } as f32 * 0.05;
                let points = Some(pos)
                    .into_iter()
                    .chain(path.iter().cloned())
                    .map(|p| {
                        let (x, y) = p.coords();
                        (x as f32, y as f32 + offset)
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

fn clone_search_goal(input: &SearchGoal) -> SearchGoal {
    unsafe { std::mem::transmute_copy(input) }
}
