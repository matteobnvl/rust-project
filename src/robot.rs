use crate::map::Tile;


pub struct Robot {
    pub position: (u16, u16),
    pub energy: u32,
    pub robot_type: RobotType,
}

pub enum RobotType {
    Eclaireur,
    Collecteur,
}


pub fn robots_eclaireur(width: u16, height: u16) -> Robot {
    let center_map = (width / 2, height / 2);
    let robot = Robot {
        position: center_map,
        energy: 100,
        robot_type: RobotType::Eclaireur,
    };
    return robot;
}

pub fn move_robot(robot: &mut Robot, width: u16, height: u16) {
    let center_map = (width / 2, height / 2);
    if robot.energy > 0 {
        robot.position.0 += 1;
        robot.energy -= 1;
    }else {
        if robot.position != center_map {
            robot.position.0 -= 1;
        }
    }
}