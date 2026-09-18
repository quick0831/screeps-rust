use screeps::ConstructionSite;
use screeps::ObjectId;
use screeps::Room;
use screeps::RoomTerrain;
use screeps::Source;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::StructureType;
use screeps::Terrain;
use screeps::look::LookResult;
use screeps::look::PositionedLookResult;
use screeps::prelude::*;

#[derive(Debug)]
pub struct SourceInfo {
    pub source: Source,
    pub container: ContainerInfo,
    pub nearby_area: Vec<PositionedLookResult>,
    pub site: u8,
}

#[derive(Debug, Clone)]
pub enum ContainerInfo {
    Built(ObjectId<StructureContainer>),
    Constructing(ObjectId<ConstructionSite>),
    None,
}

pub fn ananlyze_source(source: Source, room: &Room) -> SourceInfo {
    let pos = source.pos().xy();
    let x = pos.x.u8();
    let y = pos.y.u8();

    let nearby_area: Vec<_> = room.look_at_area(y - 1, x - 1, y + 1, x + 1);

    let container = nearby_area
        .iter()
        .find_map(|p| match &p.look_result {
            LookResult::Structure(s) => {
                if let StructureObject::StructureContainer(c) = StructureObject::from(s.clone()) {
                    Some(ContainerInfo::Built(c.id()))
                } else {
                    None
                }
            }
            LookResult::ConstructionSite(s) if s.structure_type() == StructureType::Container => {
                s.try_id().map(ContainerInfo::Constructing)
            }
            _ => None,
        })
        .unwrap_or(ContainerInfo::None);

    let mut site = 0;
    if let Some(room_terrain) = RoomTerrain::new(room.name()) {
        for x in (x - 1)..=(x + 1) {
            for y in (y - 1)..=(y + 1) {
                match room_terrain.get(x, y) {
                    Terrain::Wall => {}
                    _ => {
                        site += 1;
                    }
                }
            }
        }
    }

    SourceInfo {
        source,
        container,
        nearby_area,
        site,
    }
}
