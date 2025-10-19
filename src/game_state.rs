use crate::{base, map, robot};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc;

pub struct GameState {
    pub(crate) map: Vec<Vec<map::Tile>>,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) robots: Vec<robot::Robot>,
    map_discovered: HashMap<(u16, u16), map::Tile>,
    _base: base::SharedBase,
    pub energy: u32,
    pub crystals: u32,
    pub rx_broadcast: tokio::sync::broadcast::Receiver<base::BroadcastMessage>,
    pub tx_base: mpsc::Sender<base::BaseMessage>,
    pub last_visited: HashMap<(u16, u16), usize>,
    pub pending_resources: HashSet<(u16, u16)>,
}

impl GameState {
    pub fn new(
        map: Vec<Vec<map::Tile>>,
        width: u16,
        height: u16,
        robots: Vec<robot::Robot>,
        base: base::SharedBase,
        rx_broadcast: tokio::sync::broadcast::Receiver<base::BroadcastMessage>,
        tx_base: mpsc::Sender<base::BaseMessage>,
    ) -> Self {
        Self {
            map,
            width,
            height,
            robots,
            map_discovered: HashMap::new(),
            _base: base,
            energy: 0,
            crystals: 0,
            rx_broadcast,
            tx_base,
            last_visited: HashMap::new(),
            pending_resources: HashSet::new(),
        }
    }

    pub fn update(&mut self) {
        // Collecter les positions des éclaireurs
        let eclaireur_positions: HashSet<(u16, u16)> = self
            .robots
            .iter()
            .filter(|r| r.robot_type == robot::RobotType::Eclaireur)
            .map(|r| (r.position.0, r.position.1))
            .collect();

        // Données partagées entre threads (avec Arc + Mutex)
        let map_shared = Arc::new(Mutex::new(self.map.clone()));
        let last_visited_shared = Arc::new(Mutex::new(self.last_visited.clone()));
        let pending_shared = Arc::new(Mutex::new(self.pending_resources.clone()));

        // Séparer éclaireurs et collecteurs
        let mut eclaireurs = Vec::new();
        let mut collecteurs = Vec::new();

        for robot in self.robots.drain(..) {
            if robot.robot_type == robot::RobotType::Eclaireur {
                eclaireurs.push(robot);
            } else {
                collecteurs.push(robot);
            }
        }

        // LANCER LES ÉCLAIREURS EN PARALLÈLE
        let handles: Vec<_> = eclaireurs
            .into_iter()
            .enumerate()
            .map(|(robot_id, mut robot)| {
                let map_clone = Arc::clone(&map_shared);
                let last_visited_clone = Arc::clone(&last_visited_shared);
                let pending_clone = Arc::clone(&pending_shared);
                let eclaireur_pos = eclaireur_positions.clone();
                let width = self.width;
                let height = self.height;

                thread::spawn(move || {
                    let other_positions: HashSet<(u16, u16)> = eclaireur_pos
                        .iter()
                        .filter(|&&pos| pos != (robot.position.0, robot.position.1))
                        .copied()
                        .collect();

                    // Mettre à jour last_visited
                    {
                        let mut lv = last_visited_clone.lock().unwrap();
                        lv.insert((robot.position.0, robot.position.1), robot_id);
                    }

                    // Appeler move_robot avec les locks
                    {
                        let mut map = map_clone.lock().unwrap();
                        let lv = last_visited_clone.lock().unwrap();
                        let mut pending = pending_clone.lock().unwrap();

                        robot::move_robot(
                            &mut robot,
                            &mut map,
                            width,
                            height - 1,
                            &other_positions,
                            &lv,
                            robot_id,
                            &mut pending,
                        );
                    }

                    robot
                })
            })
            .collect();

        // Attendre que tous les threads se terminent
        let mut eclaireurs: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread éclaireur a paniqué"))
            .collect();

        // Récupérer les données partagées
        self.map = Arc::try_unwrap(map_shared)
            .expect("Arc still has references")
            .into_inner()
            .unwrap();
        self.last_visited = Arc::try_unwrap(last_visited_shared)
            .expect("Arc still has references")
            .into_inner()
            .unwrap();
        self.pending_resources = Arc::try_unwrap(pending_shared)
            .expect("Arc still has references")
            .into_inner()
            .unwrap();

        // Mettre à jour map_discovered avec les découvertes de chaque éclaireur
        for robot in &eclaireurs {
            self.map_discovered
                .extend(robot.map_discovered.iter().map(|(x, y)| (*x, y.clone())));
        }

        // Remettre les robots dans la liste
        self.robots.append(&mut eclaireurs);
        self.robots.append(&mut collecteurs);

        // ⭐ COLLECTEURS (séquentiel, pas besoin de paralléliser)
        let mut reserved_positions: HashSet<(u16, u16)> = self
            .robots
            .iter()
            .filter(|r| r.robot_type == robot::RobotType::Collecteur)
            .filter_map(|r| r.target_resource)
            .map(|pos| (pos.0, pos.1))
            .collect();

        for robot in &mut self.robots {
            robot::get_discovered_map(robot, &self.map_discovered);

            if robot.robot_type == robot::RobotType::Collecteur {
                if robot.target_resource.is_none() {
                    for ((x, y), _tile) in self.map_discovered.clone() {
                        match self.map[y as usize][x as usize] {
                            map::Tile::Explored => {
                                self.map_discovered.insert((x, y), map::Tile::Explored);
                            }
                            map::Tile::SourceFound(qty) | map::Tile::CristalFound(qty)
                                if qty == 0 =>
                            {
                                self.map_discovered.insert((x, y), map::Tile::Explored);
                            }
                            _ => {}
                        }
                    }
                    if let Some(new_target) = robot::find_nearest_resource(
                        robot,
                        &self.map_discovered,
                        &reserved_positions,
                    ) {
                        robot.target_resource = Some(new_target);
                        reserved_positions.insert((new_target.0, new_target.1));
                    }
                }

                if let Some(_target) = robot.target_resource {
                    let tx_base = self.tx_base.clone();
                    let before = robot.target_resource;
                    robot::collect_resources(
                        robot,
                        &mut self.map,
                        self.width,
                        self.height,
                        &tx_base,
                        &reserved_positions,
                    );

                    if let Some(target) = before
                        && matches!(
                            self.map[target.1 as usize][target.0 as usize],
                            map::Tile::Explored
                        )
                    {
                        self.map_discovered
                            .insert((target.0, target.1), map::Tile::Explored);
                    }
                }
            }
        }

        // Redessiner la base
        let base_center = (self.width / 2, self.height / 2);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let bx = (base_center.0 as i16 + dx) as usize;
                let by = (base_center.1 as i16 + dy) as usize;
                self.map[by][bx] = map::Tile::Base;
            }
        }
    }
}
