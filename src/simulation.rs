use crate::base::{base_loop, BaseShared};
use crate::map::Map;
use crate::robots::{collector_loop, scout_loop, RobotKind, RobotState, RobotsShared};
use std::fmt::Display;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("spawn error")]
    Spawn,
}

pub type Result<T> = std::result::Result<T, SimulationError>;

pub struct SimHandles {
    rt: tokio::runtime::Runtime,
    handles: Vec<JoinHandle<()>>,
}

impl SimHandles {
    pub fn shutdown(self) {
        // Le runtime droppera les tasks
        drop(self);
    }
}

pub fn spawn_simulation(
    map: &mut Map,
    base_shared: &BaseShared,
    robots_shared: &RobotsShared,
) -> Result<SimHandles> {
    // Runtime multi-thread pour la simu
    let rt = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| SimulationError::Spawn)?;

    // Préparer les robots initiaux (2 scouts, 2 collecteurs)
    let robots = vec![
        RobotState { id: 1, kind: RobotKind::Scout,     pos: map.base_pos, carrying: None },
        RobotState { id: 2, kind: RobotKind::Scout,     pos: map.base_pos, carrying: None },
        RobotState { id: 3, kind: RobotKind::Collector, pos: map.base_pos, carrying: None },
        RobotState { id: 4, kind: RobotKind::Collector, pos: map.base_pos, carrying: None },
    ];

    // Clones pour tasks
    let map_clone_for_scout1 = map.clone();
    let map_clone_for_scout2 = map.clone();
    let map_clone_for_coll1 = map.clone();
    let map_clone_for_coll2 = map.clone();

    let base1 = base_shared.clone();
    let base2 = base_shared.clone();
    let base3 = base_shared.clone();
    let base4 = base_shared.clone();

    let robots_shared_clone = robots_shared.clone();

    // Lancer les tasks
    let handles = rt.block_on(async {
        robots_shared_clone.set_initial(robots).await;

        let mut hs = Vec::new();
        // Base
        hs.push(tokio::spawn(base_loop(base_shared.clone())));

        // Robots
        hs.push(tokio::spawn(scout_loop(1, map_clone_for_scout1, base1, robots_shared.clone())));
        hs.push(tokio::spawn(scout_loop(2, map_clone_for_scout2, base2, robots_shared.clone())));
        hs.push(tokio::spawn(collector_loop(3, map_clone_for_coll1, base3, robots_shared.clone())));
        hs.push(tokio::spawn(collector_loop(4, map_clone_for_coll2, base4, robots_shared.clone())));

        hs
    });

    Ok(SimHandles { rt, handles })
}
