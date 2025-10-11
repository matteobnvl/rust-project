use core::hash;
use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::Hash;

use crate::map::{Tile};
use pathfinding::prelude::bfs_reach;

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RobotPosition(pub u16, pub u16);

impl RobotPosition {
    fn successors(&self, map: &Vec<Vec<Tile>>, width: u16, height: u16) -> Vec<RobotPosition> {
        let directions = [
            (1, 0),   // right
            (-1, 0),  // left
            (0, 1),   // down
            (0, -1),  // up
        ];

        directions.iter().filter_map(|(delta_x, delta_y)| {
            let nx = self.0 as i16 + delta_x;
            let ny = self.1 as i16 + delta_y;
            if nx >= 0 && ny >= 0 && (nx as u16) < width && (ny as u16) < height {
                let tile = &map[ny as usize][nx as usize];
                if matches!(tile, Tile::Floor | Tile::Explored | Tile::Source | Tile::Cristal | Tile::Base) {
                    Some(RobotPosition(nx as u16, ny as u16))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
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

pub fn move_robot(robot: &mut Robot, map: &Vec<Vec<Tile>>, width: u16, height: u16) {
    let possible_moves = robot.position.successors(map, width, height);

    if let Some(new_position) = possible_moves.iter().find(|pos| {
        let tile = &map[pos.1 as usize][pos.0 as usize];
        matches!(tile, Tile::Floor)
    }) {
        robot.position = *new_position;
        return;
    }

    if let Some(new_position) = possible_moves.get(0) {
        if robot.position != *new_position {
            robot.position = *new_position;
        }
    }
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

pub fn go_to_nearest_point(){
    // A* implementation to go to nearest point of interest
    // need to implement to allow robot to go to the nearest unexplored tile or resource
}

pub fn explore_map_with_bfs(robot: &mut Robot, width: u16, height: u16, map: &mut Vec<Vec<Tile>>, max_steps: usize) {
    let start = robot.position;

    let around_robot = robot_vision(robot, map, width, height);
    robot.map_discovered.extend(around_robot);

    let reachable = bfs_reach(start, |pos| pos.successors(map, width, height))
        .take(max_steps)
        .collect::<HashSet<_>>();

    for RobotPosition(x, y) in reachable {
        let tile = &map[y as usize][x as usize];
        
        robot.map_discovered.insert((x, y), tile.clone());

        if map[y as usize][x as usize] == Tile::Floor {
            map[y as usize][x as usize] = Tile::Explored;
        }
    }
}



// ✅ Summary: Best Pathfinding Methods for Your Scenario
// Robot Type	Goal	Recommended Algorithm	Why
// Explorer Robots	Explore entire map	BFS or Frontier-Based	Simple, complete, good for mapping
// Resource Robots	Go to specific points quickly	A*	Fast, optimal path to known goals