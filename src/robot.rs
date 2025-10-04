use std::vec;

use crate::map::{self, Tile};
use pathfinding::prelude::bfs;

pub struct Robot {
    pub position: RobotPosition,
    pub energy: u32,
    pub robot_type: RobotType,
    pub collected_resources: u32,
}

#[derive(PartialEq)]
pub enum RobotType {
    Eclaireur,
    Collecteur,
}
#[derive(Clone, Copy)]

pub struct RobotPosition(pub u16, pub u16);

impl RobotPosition {
    fn successors(&self) -> Vec<RobotPosition> {
        let &RobotPosition(x, y) = self;
        vec![
            RobotPosition(x + 1, y),     // Right
            RobotPosition(x - 1, y),     // Left
            RobotPosition(x, y + 1),     // Down
            RobotPosition(x, y - 1),     // Up
        ]
    }
}

pub fn robots_eclaireur(width: u16, height: u16) -> Robot {
    let center_map: RobotPosition = RobotPosition(width / 2, height / 2);
    let robot = Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Eclaireur,
        collected_resources: 0,
    };
    return robot;
}

pub fn move_robot(robot: &mut Robot, width: u16, height: u16) {
    let RobotPosition(x, y) = robot.position;
    let possible_moves = robot.position.successors();
    let valid_moves: Vec<RobotPosition> = possible_moves
        .into_iter()
        .filter(|pos| pos.0 >= 0 && pos.0 < width && pos.1 >= 0 && pos.1 < height)
        .collect();
    if let Some(new_position) = valid_moves.get(0) {
        robot.position = *new_position;
    }
}

pub fn explore_map(robot: &mut Robot, width: u16, height: u16, view_size: u16, map: &mut Vec<Vec<Tile>>) {
    let RobotPosition(x, y) = robot.position;
    let robot_view = [
        (x + view_size, y),
        (x - view_size, y),
        (x, y + view_size),
        (x, y - view_size),
        (x + view_size, y + view_size),
        (x + view_size, y - view_size),
        (x - view_size, y + view_size),
        (x - view_size, y - view_size),
    ];
    for (px, py) in robot_view.iter() {
        if *px < width && *py < height {
            if let Tile::Floor | Tile::Eclaireur = map[*py as usize][*px as usize] {
                map[*py as usize][*px as usize] = Tile::Explored;
            }
        }
    }

}



// ✅ Summary: Best Pathfinding Methods for Your Scenario
// Robot Type	Goal	Recommended Algorithm	Why
// Explorer Robots	Explore entire map	BFS or Frontier-Based	Simple, complete, good for mapping
// Resource Robots	Go to specific points quickly	A*	Fast, optimal path to known goals