use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::map::{Tile};

use pathfinding::prelude::bfs;
use pathfinding::prelude::astar;
use tokio::sync::mpsc::Sender;
use crate::base::BaseMessage;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub map_discovered: HashMap<(u16, u16), Tile>,
    pub found_resources: bool,
    pub collected_resources: u32,
    pub target_resource: Option<RobotPosition>,
    pub carried_resource: Option<Tile>,
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
        for (dx, dy) in &[(1,0), (-1,0), (0,1), (0,-1)] {
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
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
        carried_resource: None,
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
        found_resources: false,
        collected_resources: 0,
        target_resource: None,
        carried_resource: None,
    };
    return robot;
}


pub fn robot_vision(robot: &Robot, map: &Vec<Vec<Tile>>, width: u16, height: u16) -> HashMap<(u16, u16), Tile> {
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
            map_around.insert((nx as u16, ny as u16), map[ny as usize][nx as usize].clone());
        }
    }

    map_around
}

pub fn collect_resources(
    robot: &mut Robot,
    map: &mut Vec<Vec<Tile>>,
    width: u16,
    height: u16,
    tx_base: &Sender<BaseMessage>,
    reserved: &mut HashSet<(u16, u16)>
) {
    let base = RobotPosition(width / 2, height / 2);

    // ✅ Vérifie si la cible est déjà vide ou explorée
    if let Some(target) = robot.target_resource {
        if matches!(map[target.1 as usize][target.0 as usize], Tile::Explored) {
            tracing::info!("⚠️ La ressource ciblée a déjà été collectée : {:?}", target);
            reserved.remove(&(target.0, target.1));
            robot.target_resource = None;
            robot.found_resources = false;
            robot.carried_resource = None;
        }
    }

    // 🔍 Recherche d’une nouvelle cible si besoin
    if robot.target_resource.is_none() {
        if let Some(new_target) = find_nearest_resource(robot, &robot.map_discovered, reserved) {
            robot.target_resource = Some(new_target);
            reserved.insert((new_target.0, new_target.1));
            tracing::info!("🎯 Nouvelle cible assignée : {:?}", new_target);
        } else {
            tracing::info!("Aucune nouvelle cible disponible.");
            return;
        }
    }

    // 🏠 Retour à la base si le robot transporte quelque chose
    if robot.collected_resources > 0 {
        if robot.position == base {
            let amount = robot.collected_resources;
            robot.collected_resources = 0;

            if let Some(resource_type) = robot.carried_resource.take() {
                let _ = tx_base.try_send(BaseMessage::Collected {
                    resource: resource_type,
                    amount,
                });
                tracing::info!(
                    "⚡ Robot collecteur a déposé {} unités à la base",
                    amount
                );
            } else {
                tracing::warn!(
                    "⚠️ Robot collecteur à la base sans ressource définie (amount={})",
                    amount
                );
            }

            // Reset complet après dépôt
            robot.found_resources = false;
        } else {
            go_to_nearest_point(robot, base);
        }
        return;
    }

    // 🎯 Vérifie l’état de la cible actuelle
    if let Some(target) = robot.target_resource {
        let (tx, ty) = (target.0 as usize, target.1 as usize);
        match map[ty][tx] {
            Tile::SourceFound(qty) | Tile::CristalFound(qty) if qty == 0 => {
                map[ty][tx] = Tile::Explored;
                robot.map_discovered.insert((tx as u16, ty as u16), Tile::Explored);
                reserved.remove(&(tx as u16, ty as u16));
                robot.target_resource = None;
                robot.found_resources = false;
            }
            _ => {}
        }
    }

    // 🚶 Déplacement vers la cible
    let target = robot.target_resource.unwrap();
    if robot.position != target {
        go_to_nearest_point(robot, target);
        return;
    }

    // ⛏️ Collecte sur la cible atteinte
    let (tx, ty) = (target.0 as usize, target.1 as usize);
    let mut emptied = false;

    match &mut map[ty][tx] {
        Tile::SourceFound(qty) if *qty > 0 => {
            *qty -= 1;
            robot.collected_resources += 1;
            robot.carried_resource = Some(Tile::Source(0));
            if *qty == 0 { emptied = true; }
        }
        Tile::CristalFound(qty) if *qty > 0 => {
            *qty -= 1;
            robot.collected_resources += 1;
            robot.carried_resource = Some(Tile::Cristal(0));
            if *qty == 0 { emptied = true; }
        }
        _ => {}
    }

    // 🚫 Si ressource épuisée, marquer comme explorée
    if emptied {
    tracing::info!("✅ Ressource vidée à {:?}", target);

        // Marquer la case comme explorée
        map[ty][tx] = Tile::Explored;
        robot.map_discovered.insert((tx as u16, ty as u16), Tile::Explored);
        if let Some(global_tile) = robot.map_discovered.get_mut(&(tx as u16, ty as u16)) {
            *global_tile = Tile::Explored;
        }
        reserved.remove(&(tx as u16, ty as u16));

        // Réinitialiser les infos
        robot.target_resource = None;
        robot.found_resources = false;

        // 👉 Et le faire repartir à la base immédiatement
        let base = RobotPosition(width / 2, height / 2);
        go_to_nearest_point(robot, base);

        // Important : on force la frame à se terminer ici
        return;
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
                 *next == target || matches!(robot.map_discovered.get(&(next.0, next.1)), Some(Tile::Explored) | Some(Tile::SourceFound(_)) | Some(Tile::CristalFound(_)) | Some(Tile::Floor) | Some(Tile::Base) | Some(Tile::Eclaireur) | Some(Tile::Collecteur))
             })
             .collect::<Vec<_>>()
        },
        |p| p.distance(&target),
        |p| *p == target
    );

    if let Some((path, _cost)) = result {
        if path.len() > 1 {
            robot.position = path[1];
        }
    } else {
        tracing::warn!("⚠️ Aucun chemin trouvé vers {:?}", target);
    }
}

pub fn move_robot(robot: &mut Robot, map: &mut Vec<Vec<Tile>>, width: u16, height: u16) {
    let current_position = robot.position;
    let center_map = RobotPosition(width / 2, height / 2);

    if matches!(map[current_position.1 as usize][current_position.0 as usize], Tile::Floor | Tile::Base) {
        map[current_position.1 as usize][current_position.0 as usize] = Tile::Explored;
        robot.map_discovered.insert((current_position.0, current_position.1), Tile::Explored);
    }


    let around_robot = robot_vision(robot, map, width, height);

    for (&(x, y), tile) in &around_robot {
        match tile {
            Tile::Source(qty) => {
                map[y as usize][x as usize] = Tile::SourceFound(*qty);
                robot.map_discovered.insert((x, y), Tile::SourceFound(*qty));
            }
            Tile::Cristal(qty) => {
                map[y as usize][x as usize] = Tile::CristalFound(*qty);
                robot.map_discovered.insert((x, y), Tile::CristalFound(*qty));
            }
            _ => {}
        }
    }

    if robot.found_resources && current_position == center_map {
        robot.found_resources = false;
    }
    
    if around_robot.iter().any(|(_, tile)| matches!(tile, Tile::Cristal(_) | Tile::Source(_))) && !robot.found_resources  {
        robot.found_resources = true;
    }
    
    if robot.found_resources && current_position != center_map {
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
                        matches!(tile, Tile::Floor | Tile::Explored | Tile::Base | Tile::SourceFound(_) | Tile::CristalFound(_) | Tile::Eclaireur | Tile::Collecteur)
                    }
                })
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
        },
        |p| {
            !matches!(robot.map_discovered.get(&(p.0, p.1)), Some(Tile::Explored) | Some(Tile::SourceFound(_)) | Some(Tile::CristalFound(_)) | Some(Tile::Base) | Some(Tile::Eclaireur) | Some(Tile::Collecteur))
        }
    );

    if let Some(path) = path {
        if path.len() > 1 {
            let next_pos = path[1];
            robot.position = next_pos;
        } 
    } else {
        tracing::info!("🔄 Aucune case non explorée accessible, mouvement aléatoire");
    }
}


pub fn find_nearest_resource( robot: &Robot, discovered: &HashMap<(u16, u16), Tile>,reserved: &HashSet<(u16, u16)>,) -> Option<RobotPosition> {
    
    let resource_positions: Vec<RobotPosition> = discovered
        .iter()
        .filter(|(pos, tile)| {
            !reserved.contains(pos)
                && match tile {
                    Tile::Source(qty) | Tile::Cristal(qty) if *qty > 0 => true,
                    Tile::SourceFound(qty) | Tile::CristalFound(qty) if *qty > 0 => true,
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