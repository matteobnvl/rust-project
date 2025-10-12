use crate::base::{BaseShared, MessageToBase};
use crate::map::{Cell, Map};
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

#[derive(Clone, Copy, Debug)]
pub enum RobotKind {
    Scout,
    Collector,
}

#[derive(Clone, Debug)]
pub struct RobotState {
    pub id: usize,
    pub kind: RobotKind,
    pub pos: (usize, usize),
    pub carrying: Option<Cell>, // Collectors only
}

#[derive(Clone)]
pub struct RobotsShared {
    inner: Arc<tokio::sync::RwLock<Vec<RobotState>>>,
}

impl RobotsShared {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn set_initial(&self, robots: Vec<RobotState>) {
        let mut w = self.inner.write().await;
        *w = robots;
    }

    pub async fn snapshot(&self) -> Vec<RobotState> {
        self.inner.read().await.clone()
    }

    pub async fn update_pos(&self, id: usize, pos: (usize, usize)) {
        let mut w = self.inner.write().await;
        if let Some(r) = w.iter_mut().find(|r| r.id == id) {
            r.pos = pos;
        }
    }

    pub async fn update_carrying(&self, id: usize, carry: Option<Cell>) {
        let mut w = self.inner.write().await;
        if let Some(r) = w.iter_mut().find(|r| r.id == id) {
            r.carrying = carry;
        }
    }
}

/// Boucle d’un scout: explore, évite obstacles simples, broadcast découvertes.
pub async fn scout_loop(id: usize, map: Map, base: BaseShared, robots: RobotsShared) {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut pos = map.base_pos;

    loop {
        // exploration aléatoire (8 directions)
        let dirs = [
            (-1, -1), (0, -1), (1, -1),
            (-1,  0),          (1,  0),
            (-1,  1), (0,  1), (1,  1),
        ];
        let (dx, dy) = dirs[rng.gen_range(0..dirs.len())];
        let nx = pos.0 as isize + dx;
        let ny = pos.1 as isize + dy;

        if map.in_bounds(nx, ny) {
            let cell = map.get_cell(nx as usize, ny as usize);
            if crate::map::Map::is_walkable(&cell) {
                pos = (nx as usize, ny as usize);
                robots.update_pos(id, pos).await;

                // découvres-tu une ressource ?
                if matches!(cell, Cell::Energy(_) | Cell::Crystal(_)) {
                    let _ = base.to_base_tx.send(MessageToBase::Discovery {
                        pos,
                        cell,
                    }).await;
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Boucle d’un collector: suit les découvertes, collecte 1 unité, retourne base, décharge.
pub async fn collector_loop(
    id: usize,
    map: Map,
    base: BaseShared,
    robots: RobotsShared,
) {
    use std::collections::VecDeque;
    use rand::Rng;
    let mut pos = map.base_pos;
    let mut carrying: Option<Cell> = None;

    // Liste locale de cibles à visiter (mémoire du collecteur)
    let mut local_targets: VecDeque<(usize, usize)> = VecDeque::new();

    // Abonnement aux découvertes
    let mut rx: broadcast::Receiver<((usize, usize), Cell)> = base.discovery_tx.subscribe();

    // Variables pour détecter si le robot est bloqué
    let mut last_pos = pos;
    let mut stuck_counter = 0;

    robots.update_carrying(id, None).await;

    loop {
        // 🔹 Vérifie si le robot est bloqué
        if pos == last_pos {
            stuck_counter += 1;
        } else {
            stuck_counter = 0;
            last_pos = pos;
        }

        // 🔹 Si bloqué trop longtemps → abandonner la cible
        if stuck_counter > 25 {
            // on abandonne la cible actuelle
            if !local_targets.is_empty() {
                local_targets.pop_front();
            }
            stuck_counter = 0;
            continue;
        }

        // 1️⃣ Si on transporte une ressource → retour à la base
        if carrying.is_some() {
            let step = map.next_step_towards(pos, map.base_pos);
            pos = step;
            robots.update_pos(id, pos).await;

            // arrivé à la base ?
            if pos == map.base_pos {
                let unload = carrying.take();
                robots.update_carrying(id, None).await;
                let _ = base.to_base_tx.send(MessageToBase::ReachedBase {
                    robot_id: id,
                    unload,
                }).await;
            }

            sleep(Duration::from_millis(80)).await;
            continue;
        }

        // 2️⃣ Sinon, choisir une cible
        if local_targets.is_empty() {
            // d’abord essayer une ressource connue par la base
            if let Some(((tx, ty), _cell)) = base.get_next_resource() {
                local_targets.push_back((tx, ty));
            } else {
                // sinon, essayer de recevoir une découverte via broadcast
                if let Ok(((tx, ty), _cell)) = rx.try_recv() {
                    local_targets.push_back((tx, ty));
                }
            }
        }

        // 3️⃣ Si on a une cible → aller vers elle
        if let Some(target) = local_targets.front().cloned() {
            let step = map.next_step_towards(pos, target);
            pos = step;
            robots.update_pos(id, pos).await;

            // arrivé à destination ?
            if pos == target {
                // tenter de collecter une unité
                if let Some(collected) = map.try_collect_one(pos.0, pos.1) {
                    carrying = Some(collected);
                    robots.update_carrying(id, carrying).await;
                    // informer la base
                    let _ = base.to_base_tx.send(MessageToBase::Collected {
                        kind: collected,
                        amount: 1,
                    }).await;
                } else {
                    // ressource vide → retirer de la liste
                    local_targets.pop_front();
                }
            }

            sleep(Duration::from_millis(80)).await;
            continue;
        }

        // 4️⃣ Si aucune cible → attendre un peu
        sleep(Duration::from_millis(150)).await;
    }
}

