use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::map::Tile;

use crate::base::BaseMessage;
use pathfinding::prelude::astar;
use pathfinding::prelude::bfs;
use tokio::sync::mpsc::Sender;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub map_discovered: HashMap<(u16, u16), Tile>,
    pub found_resources: bool,
    pub collected_resources: u32,
    pub target_resource: Option<RobotPosition>,
    pub carried_resource: Option<Tile>,
    pub direction: Option<(i16, i16)>,
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
        self.0.abs_diff(other.0) + self.1.abs_diff(other.1)
    }

    fn successors(&self) -> Vec<(RobotPosition, u16)> {
        let &RobotPosition(x, y) = self;
        let mut moves = Vec::new();
        for (dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x as i16 + dx;
            let ny = y as i16 + dy;
            if nx >= 0 && ny >= 0 {
                moves.push((RobotPosition(nx as u16, ny as u16), 1));
            }
        }
        moves
    }
}

pub fn robots_eclaireur(width: u16, height: u16, direction: (i16, i16)) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Eclaireur,
        map_discovered: HashMap::new(),
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
        carried_resource: None,
        direction: Some(direction),
    }
}

pub fn robots_collecteur(width: u16, height: u16) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Collecteur,
        map_discovered: HashMap::new(),
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
        carried_resource: None,
        direction: None,
    }
}

pub fn robot_vision(
    robot: &Robot,
    map: &[Vec<Tile>],
    width: u16,
    height: u16,
) -> HashMap<(u16, u16), Tile> {
    let RobotPosition(rx, ry) = robot.position;
    let mut map_around = HashMap::new();

    // Directions cardinales: haut, bas, gauche, droite
    let directions = [
        (0i16, -1i16), // haut
        (0, 1),        // bas
        (-1, 0),       // gauche
        (1, 0),        // droite
    ];

    for (dx, dy) in directions {
        let nx = rx as i16 + dx;
        let ny = ry as i16 + dy;
        if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
            map_around.insert(
                (nx as u16, ny as u16),
                map[ny as usize][nx as usize].clone(),
            );
        }
    }

    map_around
}

pub fn collect_resources(
    robot: &mut Robot,
    map: &mut [Vec<Tile>],
    width: u16,
    height: u16,
    tx_base: &Sender<BaseMessage>,
    reserved: &HashSet<(u16, u16)>,
) {
    let base = RobotPosition(width / 2, height / 2);

    let Some(target) = robot.target_resource else {
        tracing::info!(" Pas de target définie");
        return;
    };

    if matches!(map[target.1 as usize][target.0 as usize], Tile::Explored) {
        tracing::info!(
            " Ressource ({}, {}) déjà collectée → RESET TARGET",
            target.0,
            target.1
        );
        robot.target_resource = None;
        robot.collected_resources = 0;
        robot.carried_resource = None;
        tracing::info!(" Target reset à None");
        find_nearest_resource(robot, &robot.map_discovered, reserved);
        return;
    }

    if robot.position == base && robot.collected_resources > 0 {
        let amount = robot.collected_resources;
        robot.collected_resources = 0;
        let resource_type = robot.carried_resource.clone().unwrap();
        robot.carried_resource = None;

        let _ = tx_base.try_send(BaseMessage::Collected {
            resource: resource_type,
            amount,
        });

        tracing::info!(" Déposé {} unités", amount);
        return;
    }

    if robot.collected_resources > 0 && robot.position != base {
        go_to_nearest_point(robot, base);
        return;
    }

    if robot.position != target {
        go_to_nearest_point(robot, target);
        return;
    }

    let (tx, ty) = (target.0 as usize, target.1 as usize);

    match &mut map[ty][tx] {
        Tile::SourceFound(qty) if *qty > 0 => {
            *qty -= 1;
            robot.collected_resources += 1;
            robot.carried_resource = Some(Tile::Source(0));

            if *qty == 0 {
                map[ty][tx] = Tile::Explored;
                robot
                    .map_discovered
                    .insert((tx as u16, ty as u16), Tile::Explored);
                robot.target_resource = None;
                tracing::info!("Source épuisée");
            }
        }
        Tile::CristalFound(qty) if *qty > 0 => {
            *qty -= 1;
            robot.collected_resources += 1;
            robot.carried_resource = Some(Tile::Cristal(0));

            if *qty == 0 {
                map[ty][tx] = Tile::Explored;
                robot
                    .map_discovered
                    .insert((tx as u16, ty as u16), Tile::Explored);
                robot.target_resource = None;
                tracing::info!("Cristal épuisé");
            }
        }
        _ => {
            tracing::warn!("⚠️ Ressource non disponible");
            robot.target_resource = None;
        }
    }
}

pub fn get_discovered_map(robot: &mut Robot, discovered: &HashMap<(u16, u16), Tile>) {
    robot.map_discovered = discovered.clone();
}

pub fn go_to_nearest_point(robot: &mut Robot, target: RobotPosition) {
    let result = astar(
        &robot.position,
        |p: &RobotPosition| {
            p.successors()
                .into_iter()
                .filter(|(next, _)| {
                    *next == target
                        || matches!(
                            robot.map_discovered.get(&(next.0, next.1)),
                            Some(Tile::Explored)
                                | Some(Tile::SourceFound(_))
                                | Some(Tile::CristalFound(_))
                                | Some(Tile::Floor)
                                | Some(Tile::Base)
                        )
                })
                .collect::<Vec<_>>()
        },
        |p| p.distance(&target),
        |p| *p == target,
    );

    if let Some((path, _cost)) = result {
        if path.len() > 1 {
            robot.position = path[1];
        }
    } else {
        tracing::warn!("⚠️ Aucun chemin trouvé vers {:?}", target);
    }
}

pub fn move_robot(
    robot: &mut Robot, 
    map: &mut [Vec<Tile>], 
    width: u16, 
    height: u16,
    other_eclaireurs_positions: &HashSet<(u16, u16)>,
    last_visited: &HashMap<(u16, u16), usize>,
    current_robot_id: usize,
    pending_resources: &mut HashSet<(u16, u16)> 
) {
    let current_position = robot.position;
    let center_map = RobotPosition(width / 2, height / 2);

    if matches!(
        map[current_position.1 as usize][current_position.0 as usize],
        Tile::Floor | Tile::Base
    ) {
        map[current_position.1 as usize][current_position.0 as usize] = Tile::Explored;
        robot
            .map_discovered
            .insert((current_position.0, current_position.1), Tile::Explored);
    }

    let around_robot = robot_vision(robot, map, width, height);

    if around_robot
        .iter()
        .any(|(&pos, tile)| 
            matches!(tile, Tile::Cristal(_) | Tile::Source(_))
            && !pending_resources.contains(&pos)  // ← Vérifier aussi pending_resources !
        )
        && !robot.found_resources
    {
        robot.found_resources = true;
    }

    for (&(x, y), tile) in &around_robot {
        match tile {
            Tile::Source(qty) if !pending_resources.contains(&(x, y)) => {
                pending_resources.insert((x, y));

                let target_resource = Tile::SourceFound(*qty);
                robot.carried_resource = Some(target_resource);
                robot.target_resource = Some(RobotPosition(x, y));
            }
            Tile::Cristal(qty) if !pending_resources.contains(&(x, y)) => {
                pending_resources.insert((x, y));

                let target_resource = Tile::CristalFound(*qty);
                robot.carried_resource = Some(target_resource);
                robot.target_resource = Some(RobotPosition(x, y));
            }
            _ => {}
        }
    }

    if robot.found_resources && current_position == center_map {
        robot.found_resources = false;
        let ressource_found = robot.target_resource.clone();
        if let Some(ressource_found) = ressource_found {
            map[ressource_found.1 as usize][ressource_found.0 as usize] = robot.carried_resource.clone().unwrap();
            robot.map_discovered.insert((ressource_found.0, ressource_found.1), robot.carried_resource.clone().unwrap());
            pending_resources.remove(&(ressource_found.0, ressource_found.1));
        }
    }

    if robot.found_resources && current_position != center_map {
        go_to_nearest_point(robot, center_map);
        return;
    }

    let path = bfs(
        &current_position,
        |pos| {
            pos.successors()
                .into_iter()
                .filter(|(p, _)| {
                    // ❌ BLOQUER : Ne pas aller sur une case occupée par un autre éclaireur
                    if other_eclaireurs_positions.contains(&(p.0, p.1)) {
                        return false;
                    }

                    (p.0 < width) && (p.1 < height) && {
                        let tile = &map[p.1 as usize][p.0 as usize];
                        matches!(
                            tile,
                            Tile::Floor
                                | Tile::Explored
                                | Tile::Base
                                | Tile::SourceFound(_)
                                | Tile::CristalFound(_)
                        )
                    }
                })
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
        },
        |p| {
            // ❌ ÉVITER : Ne pas viser une case récemment visitée par un autre robot
            let visited_by_other = last_visited.get(&(p.0, p.1))
                .map_or(false, |&visitor_id| visitor_id != current_robot_id);

            let is_preferred_direction = if let Some((dx, dy)) = robot.direction {
                if robot.map_discovered.len() < 20 {
                    let diff_x = p.0 as i16 - current_position.0 as i16;
                    let diff_y = p.1 as i16 - current_position.1 as i16;
                    (diff_x * dx + diff_y * dy) > 0
                } else {
                    true
                }
            } else {
                true
            };

            // ✅ La case doit être non explorée ET pas visitée par un autre robot
            is_preferred_direction 
            && !visited_by_other 
            && !matches!(
                robot.map_discovered.get(&(p.0, p.1)),
                Some(Tile::Explored)
                    | Some(Tile::SourceFound(_))
                    | Some(Tile::CristalFound(_))
                    | Some(Tile::Base)
            )
        },
    );

    if let Some(path) = path {
        if path.len() > 1 {
            let next_pos = path[1];
            robot.position = next_pos;
        }
    } else {
        tracing::info!("🔄 Aucune case non explorée accessible");
    }
}
pub fn find_nearest_resource(
    robot: &Robot,
    discovered: &HashMap<(u16, u16), Tile>,
    reserved: &HashSet<(u16, u16)>,
) -> Option<RobotPosition> {
    let resource_positions: Vec<RobotPosition> = discovered
        .iter()
        .filter(|(pos, tile)| {
            if reserved.contains(pos) {
                return false;
            }
            match tile {
                Tile::Source(qty) | Tile::SourceFound(qty) => *qty > 0,
                Tile::Cristal(qty) | Tile::CristalFound(qty) => *qty > 0,
                _ => false,
            }
        })
        .map(|(&pos, _)| RobotPosition(pos.0, pos.1))
        .collect();

    if resource_positions.is_empty() {
        tracing::info!("🔄 Aucune ressource disponible");
        return None;
    }

    let result = bfs(
        &robot.position,
        |pos| {
            pos.successors()
                .into_iter()
                .filter(|(next_pos, _)| {
                    matches!(
                        discovered.get(&(next_pos.0, next_pos.1)),
                        Some(Tile::Explored)
                            | Some(Tile::SourceFound(_))
                            | Some(Tile::CristalFound(_))
                            | Some(Tile::Floor)
                            | Some(Tile::Base)
                    )
                })
                .map(|(pos, _)| pos)
                .collect::<Vec<_>>()
        },
        |pos| resource_positions.contains(pos),
    );

    result.and_then(|path| path.into_iter().last())
}
