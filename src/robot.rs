use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::map::{Tile};

use pathfinding::prelude::bfs;
use pathfinding::prelude::astar;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub map_discovered: HashMap<(u16, u16), Tile>,
    pub map_vision: HashMap<(u16, u16), Tile>,
    pub found_resources: bool,
    pub collected_resources: u32,
    pub target_resource: Option<RobotPosition>,
}

#[derive(PartialEq)]
pub enum RobotType {
    Eclaireur,
    Collecteur,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RobotPosition(pub u16, pub u16);

impl RobotPosition {
    fn distance(&self, other: &RobotPosition) -> u16 {
        (self.0.abs_diff(other.0) + self.1.abs_diff(other.1)) as u16
    }

    fn successors(&self) -> Vec<(RobotPosition, u16)> {
        let &RobotPosition(x, y) = self;
        let mut moves = Vec::new();
        for (dx, dy) in &[(1i16,0), (-1,0), (0,1), (0,-1)] {
            let nx = x as i16 + dx;
            let ny = y as i16 + dy;
            if nx >= 0 && ny >= 0 {
                moves.push((RobotPosition(nx as u16, ny as u16), 1));
            }
        }
        moves
    }
}

pub fn robots_eclaireur(width: u16, height: u16) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    let robot = Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Eclaireur,
        map_discovered: HashMap::new(),
        map_vision: HashMap::new(),
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
    };
    return robot;
}

pub fn robots_collecteur(width: u16, height: u16) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    let robot = Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Collecteur,
        map_discovered: HashMap::new(),
        map_vision: HashMap::new(),
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
    };
    return robot;
}


pub fn robot_vision(robot: &Robot, map: &Vec<Vec<Tile>>, width: u16, height: u16) -> HashMap<(u16, u16), Tile> {
    let RobotPosition(rx, ry) = robot.position;
    let mut map_around = HashMap::new();
    let mut view_distance = 1;
    loop {
        for y_around in -view_distance..=view_distance {
            for x_around in -view_distance..=view_distance {
                let nx = rx as i16 + x_around;
                let ny = ry as i16 + y_around;
                if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
                    map_around.insert((nx as u16, ny as u16), map[ny as usize][nx as usize].clone());
                }
            }
        }
        if robot.position == RobotPosition(width / 2, height / 2) && view_distance < 3 {
            view_distance += 1;
            continue;
        } else {
            break;
        }
    }

    map_around
}

pub fn collect_resources(robot: &mut Robot, target: RobotPosition, map: &mut Vec<Vec<Tile>>, width: u16, height: u16) {

    let around_robot = robot_vision(robot, map, width, height);
    robot.map_vision = around_robot.clone();

    for (&(x, y), tile) in &around_robot {
        if *tile == Tile::Floor || *tile == Tile::Eclaireur || *tile == Tile::Collecteur {
            map[y as usize][x as usize] = Tile::Explored;
            robot.map_discovered.insert((x, y), Tile::Explored);
        }
    }

    if robot.found_resources && robot.position == RobotPosition(width / 2, height / 2) {
       tracing::info!("✅ Arrivé à la base, reset found_resources");
       robot.found_resources = false;
    } else if robot.found_resources && robot.position != RobotPosition(width / 2, height / 2) {
       go_to_nearest_point(robot, RobotPosition(width / 2, height / 2));
    }else {
        go_to_nearest_point(robot, target);
    }

    if robot.position == target {
        let (tx, ty) = (target.0 as usize, target.1 as usize);
        tracing::info!("🍻 Arrivé sur ressource {:?}", map[ty][tx]);

        match &mut map[ty][tx] {
            Tile::SourceFound(qty) => {
                if *qty > 0 {
                    *qty -= 1;
                    robot.collected_resources += 1;
                    tracing::info!("🔋 Collecté 1 énergie — reste {}", *qty);
                    if *qty == 0 {
                        map[ty][tx] = Tile::Explored;
                        tracing::info!("⚡ Source épuisée");
                    }
                }
                robot.found_resources = true;
            }
            Tile::CristalFound(qty) => {
                if *qty > 0 {
                    *qty -= 1;
                    robot.collected_resources += 1;
                    tracing::info!("💎 Collecté 1 cristaux — reste {}", *qty);
                    if *qty == 0 {
                        map[ty][tx] = Tile::Explored;
                        tracing::info!("💎 Cristal épuisé");
                    }
                }
                robot.found_resources = true;
            }
            _ => {}
        }
    }
}

pub fn get_discovered_map(robot: &mut Robot, discovered: &HashMap<(u16, u16), Tile>) {
    robot.map_discovered = discovered.clone();
}

pub fn go_to_nearest_point(robot: &mut Robot, target: RobotPosition) {
    tracing::info!("✅ Target point found: {:?}", target);

    let result = astar(
        &robot.position,
        |p: &RobotPosition| {
            p.successors()
             .into_iter()
             .filter(|(next, _)| {
                 *next == target || matches!(robot.map_discovered.get(&(next.0, next.1)), Some(Tile::Explored))
             })
             .collect::<Vec<_>>()
        },
        |p| p.distance(&target),
        |p| *p == target
    );

    if let Some((path, _cost)) = result {
        if path.len() > 1 {
            robot.position = path[1];
        } else {
            tracing::info!("📍 Le robot est déjà sur la position cible {:?}", target);
        }
    } else {
        tracing::warn!("⚠️ Aucun chemin trouvé vers {:?}", target);
    }
}


pub fn move_robot(robot: &mut Robot, map: &mut Vec<Vec<Tile>>, width: u16, height: u16) {
    let current_position = robot.position;
    let center_map = RobotPosition(width / 2, height / 2);

    let around_robot = robot_vision(robot, map, width, height);
    robot.map_vision = around_robot.clone();

    for (&(x, y), tile) in &around_robot {
        if *tile == Tile::Floor || *tile == Tile::Eclaireur || *tile == Tile::Collecteur {
            map[y as usize][x as usize] = Tile::Explored;
            robot.map_discovered.insert((x, y), Tile::Explored);
        } else if let Tile::Source(qty) = tile {
            map[y as usize][x as usize] = Tile::SourceFound(*qty);
            robot.map_discovered.insert((x, y), tile.clone());
        } else if let Tile::Cristal(qty) = tile {
            map[y as usize][x as usize] = Tile::CristalFound(*qty);
            robot.map_discovered.insert((x, y), tile.clone());
        }
    }

    if robot.found_resources && current_position == center_map {
        tracing::info!("✅ Arrivé à la base, reset found_resources");
        robot.found_resources = false;
    }
    
    if around_robot.iter().any(|(_, tile)| matches!(tile, Tile::Cristal(_) | Tile::Source(_))) && !robot.found_resources  {
        robot.found_resources = true;
    }
    
    if robot.found_resources && current_position != center_map {
        tracing::info!("🏠 Retour à la base");
        go_to_nearest_point(robot, center_map);
        return;
    }

    let path = bfs(
        &current_position,
        |pos| {
            pos.successors().into_iter()
                .filter(|(p, _)| {
                    (p.0 < width) && (p.1 < height) && {
                        let tile = &map[p.1 as usize][p.0 as usize];
                        matches!(tile, Tile::Floor | Tile::Explored | Tile::Base | Tile::Collecteur)
                    }
                })
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
        },
        |p| {
            !matches!(robot.map_discovered.get(&(p.0, p.1)), Some(Tile::Explored))
        }
    );

    if let Some(path) = path {
        if path.len() > 1 {
            robot.position = path[1];
            tracing::info!("🤖 Déplacement vers {:?}, cible finale: {:?}", path[1], path.last());
        } else {
            tracing::info!("📍 Déjà sur une case non explorée");
        }
    } else {
        tracing::warn!("⚠️ Aucune case non explorée accessible");
    }
}

pub fn find_nearest_resource(
    robot: &Robot,
    discovered: &HashMap<(u16, u16), Tile>,
    reserved: &HashSet<(u16, u16)>,
) -> Option<RobotPosition> {
    let mut nearest: Option<(RobotPosition, u16)> = None;
    for (&(x, y), tile) in discovered {
        if reserved.contains(&(x, y)) {
            continue; 
        }
        if matches!(tile, Tile::Source(_) | Tile::Cristal(_) | Tile::SourceFound(_) | Tile::CristalFound(_)) {
            let pos = RobotPosition(x, y);
            let dist = robot.position.distance(&pos);
            match nearest {
                Some((_, best_dist)) if dist >= best_dist => continue,
                _ => nearest = Some((pos, dist)),
            }
        }
    }
    nearest.map(|(pos, _)| pos)
}
