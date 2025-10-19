use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, mpsc};

use crate::map::Tile;

#[derive(Debug, Clone)]
pub enum BaseMessage {
    Collected { resource: Tile, amount: u32 },
}

#[derive(Debug, Clone)]
pub enum BroadcastMessage {
    BaseStats { energy: u32, crystals: u32 },
}

pub struct BaseStateData {
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
                total_energy: 0,
                total_crystals: 0,
                tx_broadcast,
            }),
        })
    }

    pub async fn run(self: Arc<Self>, mut rx_events: mpsc::Receiver<BaseMessage>) {
        while let Some(msg) = rx_events.recv().await {
            match msg {
                BaseMessage::Collected { resource, amount } => {
                    let mut guard = self.state.write().await;
                    match resource {
                        Tile::Source(_) => {
                            guard.total_energy = guard.total_energy.saturating_add(amount)
                        }
                        Tile::Cristal(_) => {
                            guard.total_crystals = guard.total_crystals.saturating_add(amount)
                        }
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
}
