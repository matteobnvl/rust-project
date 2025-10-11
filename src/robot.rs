use core::hash;
use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::Hash;

use crate::map::{Tile};
use crate::robot;
use pathfinding::prelude::bfs;
use pathfinding::prelude::astar;


use crate::utils;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub map_discovered: HashMap<(u16, u16), Tile>,
    pub map_vision: HashMap<(u16, u16), Tile>,
    pub collected_resources: u32,
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
        collected_resources: 0,
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
        if map_around.iter().all(|(_, tile)| *tile == Tile::Base) {
            view_distance += 1;
            continue;
        } else {
            break;
        }
    }

    map_around
}

pub fn go_to_nearest_point(robot: &mut Robot, target: RobotPosition) {
    tracing::info!("✅ Target point found: {:?}", target);

    let result = astar(
        &robot.position,
        |p: &RobotPosition| {
            p.successors()
             .into_iter()
             .filter(|(next, _)| {
                 // Ne passer que par les cases explorées ou la cible finale
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

    // Découverte autour du robot
    let around_robot = robot_vision(robot, map, width, height);
    robot.map_vision = around_robot.clone();

    // Marquer les cases explorées
    for (&(x, y), tile) in &around_robot {
        if *tile == Tile::Floor || *tile == Tile::Eclaireur {
            map[y as usize][x as usize] = Tile::Explored;
            robot.map_discovered.insert((x, y), Tile::Explored);
        }
    }

    // Trouver la première case non explorée à atteindre avec BFS
    let path = bfs(
        &current_position,
        |pos| {
            pos.successors().into_iter()
                .filter(|(p, _)| {
                    (p.0 < width) && (p.1 < height) && {
                        let tile = &map[p.1 as usize][p.0 as usize];
                        matches!(tile, Tile::Floor | Tile::Explored | Tile::Base)
                    }
                })
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
        },
        |p| {
            // Condition de succès : case non explorée
            !matches!(robot.map_discovered.get(&(p.0, p.1)), Some(Tile::Explored | Tile::Base))
        }
    );

    if let Some(path) = path {
        if path.len() > 1 {
            robot.position = path[1]; // Prendre la prochaine étape du chemin
            tracing::info!("🤖 Déplacement vers {:?}, cible finale: {:?}", path[1], path.last());
        } else {
            tracing::info!("📍 Déjà sur une case non explorée");
        }
    } else {
        tracing::warn!("⚠️ Aucune case non explorée accessible");
    }
}