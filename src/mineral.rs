use log::error;
use screeps::Creep;
use screeps::HasPosition;
use screeps::Mineral;
use screeps::Part;
use screeps::Room;
use screeps::RoomXY;
use screeps::StructureContainer;
use screeps::StructureObject;
use screeps::StructureType;
use screeps::Terrain;
use screeps::look::LookResult;

const MIN_OPERATING_RCL: u8 = 6;

pub struct MineralManager {
    size: u8,
    mineral: Mineral,
    container: Option<ContainerInfo>,
    extractor_ready: bool,
}

#[derive(Debug, Clone)]
enum ContainerInfo {
    Built(StructureContainer),
    Constructing,
}

#[derive(Debug, Clone)]
enum ExtractorInfo {
    Built,
    Constructing,
}

impl MineralManager {
    pub fn new(mineral: Mineral, room: &Room) -> Self {
        let rcl = room.controller().unwrap().level();

        // Disable this module
        if rcl < MIN_OPERATING_RCL {
            return Self {
                size: 0,
                mineral,
                container: None,
                extractor_ready: false,
            };
        }

        let (x, y) = mineral.pos().coords();
        let nearby_area: Vec<_> = room.look_at_area(y - 1, x - 1, y + 1, x + 1);
        let mut container = None;
        let mut extractor = None;
        let mut area = Vec::new();
        for p in nearby_area {
            match p.look_result {
                LookResult::Structure(s) => match s.into() {
                    StructureObject::StructureContainer(c) => {
                        container = Some(ContainerInfo::Built(c));
                    }
                    StructureObject::StructureExtractor(_) => {
                        extractor = Some(ExtractorInfo::Built);
                    }
                    _ => {}
                },
                LookResult::ConstructionSite(s) => match s.structure_type() {
                    StructureType::Container => {
                        container = Some(ContainerInfo::Constructing);
                    }
                    StructureType::Extractor => {
                        extractor = Some(ExtractorInfo::Constructing);
                    }
                    _ => {}
                },
                LookResult::Terrain(Terrain::Plain | Terrain::Swamp) => {
                    area.push(RoomXY::checked_new(p.x, p.y).expect("coordinate not in range"));
                }
                _ => {}
            }
        }

        if container.is_none() {
            let new_container_pos = area
                .iter()
                .max_by_key(|a| area.iter().filter(|b| a.is_near_to(**b)).count())
                .copied();
            if let Some(p) = new_container_pos
                && let Err(err) = room.create_construction_site(
                    p.x.u8(),
                    p.y.u8(),
                    StructureType::Container,
                    None,
                )
            {
                error!("Failed to place construction site for container: {err}");
            }
        }

        if extractor.is_none()
            && let Err(err) = room.create_construction_site(x, y, StructureType::Extractor, None)
        {
            error!("Failed to place construction site for extractor: {err}");
        }

        let extractor_ready = container.is_some() && extractor.is_some();

        Self {
            size: 0,
            mineral,
            container,
            extractor_ready,
        }
    }

    pub fn register_miner(&mut self, creep: &Creep) {
        let size = creep
            .body()
            .into_iter()
            .filter(|p| p.part() == Part::Work)
            .count() as u8;
        self.size += size;
    }

    pub fn get_container(&self) -> Option<StructureContainer> {
        match &self.container {
            Some(ContainerInfo::Built(c)) => Some(c.clone()),
            _ => None,
        }
    }

    pub fn get_mineral(&self) -> Mineral {
        self.mineral.clone()
    }

    pub fn get_creep_spawn_info(&self) -> Option<u8> {
        if self.size == 0 && self.extractor_ready {
            Some(5)
        } else {
            None
        }
    }
}
