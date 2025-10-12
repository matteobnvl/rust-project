use crate::map::Cell;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum MessageToBase {
    Discovery { pos: (usize, usize), cell: Cell },
    Collected { kind: Cell, amount: u32 },
    ReachedBase { robot_id: usize, unload: Option<Cell> },
}

#[derive(Default, Clone)]
pub struct BaseStats {
    pub energy_total: u32,
    pub crystal_total: u32,
}

#[derive(Clone)]
pub struct BaseShared {
    pub stats: Arc<Mutex<BaseStats>>,
    pub known_resources: Arc<RwLock<VecDeque<((usize, usize), Cell)>>>,
    pub assigned_resources: Arc<RwLock<Vec<(usize, usize)>>>,
    pub discovery_tx: broadcast::Sender<((usize, usize), Cell)>,
    pub to_base_tx: mpsc::Sender<MessageToBase>,
    pub to_base_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<MessageToBase>>>,
}

impl BaseShared {
    pub fn new() -> Self {
        let (discovery_tx, _) = broadcast::channel(256);
        let (to_base_tx, to_base_rx) = mpsc::channel(512);
        Self {
            stats: Arc::new(Mutex::new(BaseStats::default())),
            known_resources: Arc::new(RwLock::new(VecDeque::new())),
            assigned_resources: Arc::new(RwLock::new(Vec::new())),
            discovery_tx,
            to_base_tx,
            to_base_rx: Arc::new(tokio::sync::Mutex::new(to_base_rx)),
        }
    }

    pub fn remove_known_resource(&self, pos: (usize, usize)) {
        {
            let mut known = self.known_resources.write().unwrap();
            known.retain(|(p, _)| *p != pos);
        }
        {
            let mut assigned = self.assigned_resources.write().unwrap();
            assigned.retain(|p| *p != pos);
        }
    }

    pub fn get_next_resource(&self) -> Option<((usize, usize), Cell)> {
        let mut known = self.known_resources.write().unwrap();
        let mut assigned = self.assigned_resources.write().unwrap();

        for (pos, cell) in known.iter() {
            if !assigned.contains(pos) {
                assigned.push(*pos);
                return Some((*pos, *cell));
            }
        }

        None
    }

    pub fn try_reserve_resource(&self, pos: (usize, usize)) -> bool {
        let known = self.known_resources.read().unwrap();
        if !known.iter().any(|(p, _)| *p == pos) {
            return false;
        }
        let mut assigned = self.assigned_resources.write().unwrap();
        if assigned.contains(&pos) {
            return false;
        }
        assigned.push(pos);
        true
    }

    pub fn broadcast_discovery(&self, pos: (usize, usize), cell: Cell) {
        if let Err(e) = self.discovery_tx.send((pos, cell)) {
            debug!("No collectors currently listening for discovery: {:?}", e);
        } else {
            debug!("Broadcasted discovery of {:?} at {:?}", cell, pos);
        }
    }

    pub fn release_resource(&self, pos: (usize, usize)) {
        let mut assigned = self.assigned_resources.write().unwrap();
        assigned.retain(|p| *p != pos);
    }

}

pub async fn base_loop(shared: BaseShared) {
    loop {
        let msg = {
            let mut rx = shared.to_base_rx.lock().await;
            rx.recv().await
        };
        let Some(msg) = msg else { break };

        match msg {
            MessageToBase::Discovery { pos, cell } => {
                if matches!(cell, Cell::Energy(_) | Cell::Crystal(_)) {
                    {
                        let mut k = shared.known_resources.write().unwrap();
                        k.push_back((pos, match cell {
                            Cell::Energy(q) => Cell::Energy(q),
                            Cell::Crystal(q) => Cell::Crystal(q),
                            _ => Cell::Empty,
                        }));
                    }
                    let _ = shared.discovery_tx.send((pos, cell));
                    debug!(?pos, ?cell, "Discovery broadcasted");
                }
            }
            MessageToBase::Collected { kind, amount } => {
                let mut s = shared.stats.lock().unwrap();
                match kind {
                    Cell::Energy(_) => s.energy_total = s.energy_total.saturating_add(amount),
                    Cell::Crystal(_) => s.crystal_total = s.crystal_total.saturating_add(amount),
                    _ => {}
                }
                debug!(?kind, amount, "Base totals updated");
            }
            MessageToBase::ReachedBase { robot_id, unload } => {
                if let Some(cell) = unload {
                    let mut s = shared.stats.lock().unwrap();
                    match cell {
                        Cell::Energy(_) => s.energy_total += 1,
                        Cell::Crystal(_) => s.crystal_total += 1,
                        _ => {}
                    }
                    debug!(robot_id, ?cell, "Unload at base");
                }
            }
        }
    }
}
