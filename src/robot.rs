use core::hash;
use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::Hash;

use crate::map::{Tile};
use crate::robot;
use pathfinding::prelude::bfs_reach;
use pathfinding::prelude::astar;


use crate::utils;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub map_discovered: HashMap<(u16, u16), Tile>,
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
        vec![RobotPosition(x + 1, y), RobotPosition(x - 1, y), RobotPosition(x, y + 1), RobotPosition(x, y - 1)]
            .into_iter().map(|p| (p, 1)).collect()
    }
}

pub fn robots_eclaireur(width: u16, height: u16) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    let robot = Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Eclaireur,
        map_discovered: HashMap::new(),
        collected_resources: 0,
    };
    return robot;
}

pub fn robot_vision(robot: &Robot, map: &Vec<Vec<Tile>>, width: u16, height: u16) -> HashMap<(u16, u16), Tile> {
    let RobotPosition(rx, ry) = robot.position;
    let mut map_around = HashMap::new();
    for y_around in -1..=1 {
        for x_around in -1..=1 {
            let nx = rx as i16 + x_around;
            let ny = ry as i16 + y_around;
            if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
                map_around.insert((nx as u16, ny as u16), map[ny as usize][nx as usize].clone());
            }
        }
    }

    map_around
}

pub fn go_to_nearest_point(robot: &mut Robot, target: RobotPosition) -> () {
    // let _guard = utils::configure_logger();
    tracing::info!("✅ Target point found: {:?}", target);
    let result = astar(&robot.position, |p :&RobotPosition| p.successors(), |p| p.distance(&target) / 3 , |p| *p == target);
    match result {
        Some((path, _cost)) => {
            if path.len() > 1 {
                let next_step = path[1];
                tracing::info!(
                    "🤖 Déplacement étape par étape : {:?} -> {:?}",
                    robot.position,
                    next_step
                );
                robot.position = next_step;
            } else {
                tracing::info!("📍 Le robot est déjà sur la position cible {:?}", target);
            }
        }
        None => {
            tracing::warn!("⚠️ Aucun chemin trouvé vers {:?}", target);
        }
    }
}

pub fn move_robot(robot: &mut Robot, map: &mut Vec<Vec<Tile>>, width: u16, height: u16) {
    let current_position = robot.position;

    // Découverte autour du robot
    let around_robot = robot_vision(robot, map, width, height);
    robot.map_discovered.extend(around_robot.clone());

    // Marquer les cases explorées
    for (&(x, y), tile) in &around_robot {
        if *tile == Tile::Floor {
            map[y as usize][x as usize] = Tile::Explored;
            robot.map_discovered.insert((x, y), Tile::Explored);
        }
    }

    // Trouver la première case non explorée à atteindre avec BFS
    let next_position = bfs_reach(current_position, |pos| {
        pos.successors().into_iter()
            .filter(|(p, _)| {
                (p.0 < width) && (p.1 < height) && (p.0 as i16 >= 0) && (p.1 as i16 >= 0) && {
                    let tile = &map[p.1 as usize][p.0 as usize];
                    matches!(tile, Tile::Floor | Tile::Explored | Tile::Base)
                }
            })
            .map(|(p, _)| p)
    })
    .skip(1) // ignorer la position actuelle
    .find(|p| {
        // On cherche la première case non explorée
        !matches!(robot.map_discovered.get(&(p.0, p.1)), Some(Tile::Explored))
    });

    // Avancer d'une case vers cette position si elle existe
    if let Some(next_pos) = next_position {
        let is_successor = current_position.successors().iter().any(|(p, _)| *p == next_pos);
        if is_successor {
            robot.position = next_pos;
        } else {
            go_to_nearest_point(robot, next_pos);
        }
    }
}
