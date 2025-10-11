use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, RwLock};

use crate::map::Tile;
use crate::robot::RobotPosition;

#[derive(Debug, Clone)]
pub enum BaseMessage {
    Discovery { pos: RobotPosition, tile: Tile },
    Collected { resource: Tile, amount: u32 },
}

#[derive(Debug, Clone)]
pub enum BroadcastMessage {
    NewResource { pos: RobotPosition, tile: Tile },
    BaseStats { energy: u32, crystals: u32 },
}

pub struct BaseStateData {
    pub known_map: HashMap<RobotPosition, Tile>,
    pub total_energy: u32,
    pub total_crystals: u32,
    pub tx_broadcast: broadcast::Sender<BroadcastMessage>,
}

pub struct Base {
    state: RwLock<BaseStateData>,
}

pub type SharedBase = Arc<Base>;

impl Base {
    pub fn new(tx_broadcast: broadcast::Sender<BroadcastMessage>) -> SharedBase {
        Arc::new(Base {
            state: RwLock::new(BaseStateData {
                known_map: HashMap::new(),
                total_energy: 0,
                total_crystals: 0,
                tx_broadcast,
            }),
        })
    }
    
    pub async fn run(self: Arc<Self>, mut rx_events: mpsc::Receiver<BaseMessage>) {
        while let Some(msg) = rx_events.recv().await {
            match msg {
                BaseMessage::Discovery { pos, tile } => {
                    let mut guard = self.state.write().await;
                    guard.known_map.insert(pos, tile.clone());
                    // diffuse la découverte aux robots intéressés
                    let _ = guard
                        .tx_broadcast
                        .send(BroadcastMessage::NewResource { pos, tile });
                }
                BaseMessage::Collected { resource, amount } => {
                    let mut guard = self.state.write().await;
                    match resource {
                        Tile::Source => guard.total_energy = guard.total_energy.saturating_add(amount),
                        Tile::Cristal => guard.total_crystals = guard.total_crystals.saturating_add(amount),
                        _ => {} // ignore les autres tuiles
                    }
                    let _ = guard.tx_broadcast.send(BroadcastMessage::BaseStats {
                        energy: guard.total_energy,
                        crystals: guard.total_crystals,
                    });
                }
            }
        }
    }
    
    pub async fn totals(&self) -> (u32, u32) {
        let guard = self.state.read().await;
        (guard.total_energy, guard.total_crystals)
    }
}
