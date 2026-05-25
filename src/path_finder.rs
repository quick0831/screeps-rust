use screeps::CostMatrix;
use screeps::Creep;
use screeps::Position;
use screeps::RoomName;
use screeps::action_error_codes::CreepMoveByPathErrorCode;
use screeps::find;
use screeps::game;
use screeps::pathfinder::MultiRoomCostResult;
use screeps::pathfinder::SearchOptions;
use screeps::pathfinder::search;
use screeps::prelude::*;

const COST_UNWALKABLE: u8 = 255;

// TODO: cache the result when queried on the same tick
fn get_costmatrix(room_name: RoomName) -> MultiRoomCostResult {
    let Some(room) = game::rooms().get(room_name) else {
        return MultiRoomCostResult::Default;
    };

    let cost_matrix = CostMatrix::new();

    let creep_pos = room.find(find::CREEPS, None).into_iter().map(|c| c.pos());
    let structure_pos = room
        .find(find::MY_STRUCTURES, None)
        .into_iter()
        .map(|s| s.pos());
    for pos in creep_pos.chain(structure_pos) {
        let (x, y) = pos.coords();
        cost_matrix.set(x, y, COST_UNWALKABLE);
    }
    MultiRoomCostResult::CostMatrix(cost_matrix)
}

pub fn path_away_from(
    creep: &Creep,
    target: Position,
    range: u32,
) -> Result<(), CreepMoveByPathErrorCode> {
    let options = SearchOptions::new(get_costmatrix)
        .flee(true)
        .max_rooms(1)
        .max_ops(100);
    let result = search(creep.pos(), target, range, Some(options));
    creep.move_by_path(&result.opaque_path())
}
