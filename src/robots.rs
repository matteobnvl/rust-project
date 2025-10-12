use crate::base::{BaseShared, MessageToBase};
use crate::map::{Cell, Map};
use rand::{Rng, SeedableRng};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

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
    pub carrying: Option<Cell>
}

#[derive(Clone)]
pub struct RobotsShared {
    inner: Arc<tokio::sync::RwLock<Vec<RobotState>>>,
    visited: Arc<tokio::sync::RwLock<HashSet<(usize, usize)>>>,
    frontier: Arc<tokio::sync::RwLock<VecDeque<(usize, usize)>>>,
    claimed_targets: Arc<tokio::sync::RwLock<HashSet<(usize, usize)>>>,
}

impl RobotsShared {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            visited: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            frontier: Arc::new(tokio::sync::RwLock::new(VecDeque::new())),
            claimed_targets: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
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

    pub async fn mark_visited(&self, pos: (usize, usize)) -> bool {
        let mut v = self.visited.write().await;
        v.insert(pos)
    }

    pub async fn is_visited(&self, pos: (usize, usize)) -> bool {
        let v = self.visited.read().await;
        v.contains(&pos)
    }

    pub async fn push_frontier_many(&self, items: impl IntoIterator<Item = (usize, usize)>) {
        let v = self.visited.read().await;
        let prelim: Vec<(usize, usize)> = items
            .into_iter()
            .filter(|it| !v.contains(it))
            .collect();
        drop(v);
        let mut f = self.frontier.write().await;
        for it in prelim {
            if !f.contains(&it) {
                f.push_back(it);
            }
        }
    }

    pub async fn pop_frontier(&self) -> Option<(usize, usize)> {
        let mut f = self.frontier.write().await;
        f.pop_front()
    }

    pub async fn try_claim_target(&self, pos: (usize, usize)) -> bool {
        let mut c = self.claimed_targets.write().await;
        if c.contains(&pos) {
            false
        } else {
            c.insert(pos);
            true
        }
    }

    pub async fn release_claim(&self, pos: (usize, usize)) {
        let mut c = self.claimed_targets.write().await;
        c.remove(&pos);
    }

    pub async fn is_claimed(&self, pos: (usize, usize)) -> bool {
        let c = self.claimed_targets.read().await;
        c.contains(&pos)
    }
}

pub async fn scout_loop(id: usize, map: Map, base: BaseShared, robots: RobotsShared) {
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut pos = map.base_pos;

    if robots.mark_visited(pos).await {
        let mut seed = Vec::new();
        for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
            let nx = pos.0 as isize + dx;
            let ny = pos.1 as isize + dy;
            if map.in_bounds(nx, ny) {
                let c = map.get_cell(nx as usize, ny as usize);
                if Map::is_walkable(&c) {
                    seed.push((nx as usize, ny as usize));
                }
            }
        }
        robots.push_frontier_many(seed).await;
    }

    let mut current_path: VecDeque<(usize, usize)> = VecDeque::new();
    let mut current_target: Option<(usize, usize)> = None;
    let mut last_pos = pos;
    let mut no_move_ticks: u32 = 0;
    let mut pending_discoveries: Vec<((usize, usize), Cell)> = Vec::new();
    let mut returning_to_base: bool = false;

    loop {
        if current_path.is_empty() {
            if returning_to_base {
                if let Some(mut path) = map.find_path(pos, map.base_pos) {
                    if path.len() > 1 {
                        path.remove(0);
                        current_path = path.into();
                    }
                }
            } else {
                let mut candidates: Vec<(usize, usize)> = Vec::new();
                for _ in 0..30 {
                    if let Some(t) = robots.pop_frontier().await {
                        candidates.push(t);
                    } else {
                        break;
                    }
                }

            let mut viable: Vec<(usize, usize)> = Vec::new();
            for &c in &candidates {
                if !robots.is_visited(c).await {
                    viable.push(c);
                }
            }

            let others = robots.snapshot().await;
            let mut other_scouts: Vec<(usize, usize)> = Vec::new();
            for r in others {
                if r.id != id {
                    if let RobotKind::Scout = r.kind {
                        other_scouts.push(r.pos);
                    }
                }
            }

            fn manhattan(a: (usize, usize), b: (usize, usize)) -> i32 {
                (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs()
            }

            let mut scored: Vec<((usize, usize), i32)> = Vec::new();
            for &t in &viable {
                let d_me = manhattan(pos, t);
                let d_others = if other_scouts.is_empty() {
                    0
                } else {
                    other_scouts
                        .iter()
                        .map(|&o| manhattan(o, t))
                        .min()
                        .unwrap_or(0)
                };
                let score = -(d_me as i32) + 2 * d_others + (rng.gen_range(0..3) as i32);
                scored.push((t, score));
            }

            scored.sort_by(|a, b| b.1.cmp(&a.1));

            let mut chosen: Option<(usize, usize)> = None;
            for (t, _s) in &scored {
                // éviter celles déjà claimées ou visitées pendant l'attente
                if robots.is_visited(*t).await || robots.is_claimed(*t).await { continue; }
                if robots.try_claim_target(*t).await {
                    chosen = Some(*t);
                    break;
                }
            }

            if !candidates.is_empty() {
                let mut to_requeue = Vec::new();
                for c in candidates {
                    if Some(c) != chosen {
                        to_requeue.push(c);
                    }
                }
                if !to_requeue.is_empty() {
                    robots.push_frontier_many(to_requeue).await;
                }
            }

            if let Some(target) = chosen {
                if let Some(prev) = current_target.take() {
                    if prev != target {
                        robots.release_claim(prev).await;
                    }
                }
                current_target = Some(target);
                if let Some(mut path) = map.find_path(pos, target) {
                    if path.len() > 1 {
                        // on ignore la première case (position actuelle)
                        path.remove(0);
                        current_path = path.into();
                    }
                }
            } else {
                let mut extra = Vec::new();
                for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                    let nx = pos.0 as isize + dx;
                    let ny = pos.1 as isize + dy;
                    if map.in_bounds(nx, ny) {
                        let c = map.get_cell(nx as usize, ny as usize);
                        if Map::is_walkable(&c) {
                            let np = (nx as usize, ny as usize);
                            extra.push(np);
                        }
                    }
                }
                if extra.is_empty() {
                    let dirs = [(-1,0),(1,0),(0,-1),(0,1)];
                    let (dx, dy) = dirs[rng.gen_range(0..dirs.len())];
                    let nx = pos.0 as isize + dx;
                    let ny = pos.1 as isize + dy;
                    if map.in_bounds(nx, ny) {
                        let c = map.get_cell(nx as usize, ny as usize);
                        if Map::is_walkable(&c) {
                            current_path.push_back((nx as usize, ny as usize));
                        }
                    }
                } else {
                    robots.push_frontier_many(extra).await;
                }
            }
        }
        }

        if let Some(next_pos) = current_path.pop_front() {
            pos = next_pos;
            robots.update_pos(id, pos).await;

            if let Some(tgt) = current_target {
                if pos == tgt {
                    robots.release_claim(tgt).await;
                    current_target = None;
                }
            }

            if robots.mark_visited(pos).await {
                let mut neigh = Vec::new();
                for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                    let nx = pos.0 as isize + dx;
                    let ny = pos.1 as isize + dy;
                    if map.in_bounds(nx, ny) {
                        let c = map.get_cell(nx as usize, ny as usize);
                        if Map::is_walkable(&c) {
                            neigh.push((nx as usize, ny as usize));
                        }
                    }
                }
                robots.push_frontier_many(neigh).await;
            }

            let cell = map.get_cell(pos.0, pos.1);
            if matches!(cell, Cell::Energy(_) | Cell::Crystal(_)) {
                // Ajouter si pas déjà présent
                if !pending_discoveries.iter().any(|(p, _)| *p == pos) {
                    pending_discoveries.push((pos, cell));
                }
                if !returning_to_base {
                    returning_to_base = true;
                    if let Some(tgt) = current_target.take() {
                        robots.release_claim(tgt).await;
                    }
                    current_path.clear();
                    if let Some(mut path) = map.find_path(pos, map.base_pos) {
                        if path.len() > 1 {
                            path.remove(0);
                            current_path = path.into();
                        }
                    }
                }
            }

            if pos == map.base_pos && !pending_discoveries.is_empty() {
                let mut to_send = Vec::new();
                to_send.append(&mut pending_discoveries);
                for (p, c) in to_send {
                    let _ = base.to_base_tx
                        .send(MessageToBase::Discovery { pos: p, cell: c })
                        .await;
                }
                returning_to_base = false;
            }
        }

        if pos == last_pos {
            no_move_ticks = no_move_ticks.saturating_add(1);
        } else {
            no_move_ticks = 0;
            last_pos = pos;
        }
        if no_move_ticks >= 30 {
            current_path.clear();
            if let Some(tgt) = current_target.take() {
                robots.release_claim(tgt).await;
            }
            let dirs = [(-1,0),(1,0),(0,-1),(0,1)];
            let start = rng.gen_range(0..dirs.len());
            let mut moved = false;
            for i in 0..dirs.len() {
                let (dx, dy) = dirs[(start + i) % dirs.len()];
                let nx = pos.0 as isize + dx;
                let ny = pos.1 as isize + dy;
                if map.in_bounds(nx, ny) {
                    let c = map.get_cell(nx as usize, ny as usize);
                    if Map::is_walkable(&c) {
                        pos = (nx as usize, ny as usize);
                        robots.update_pos(id, pos).await;
                        if robots.mark_visited(pos).await {
                            let mut neigh = Vec::new();
                            for (dx2, dy2) in [(1,0),(-1,0),(0,1),(0,-1)] {
                                let nnx = pos.0 as isize + dx2;
                                let nny = pos.1 as isize + dy2;
                                if map.in_bounds(nnx, nny) {
                                    let cc = map.get_cell(nnx as usize, nny as usize);
                                    if Map::is_walkable(&cc) {
                                        neigh.push((nnx as usize, nny as usize));
                                    }
                                }
                            }
                            robots.push_frontier_many(neigh).await;
                        }
                        moved = true;
                        break;
                    }
                }
            }
            if moved {
                no_move_ticks = 0;
                last_pos = pos;
            }
        }

        sleep(Duration::from_millis(80)).await;
    }
}

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

    let mut local_targets: VecDeque<(usize, usize)> = VecDeque::new();
    let mut rx: broadcast::Receiver<((usize, usize), Cell)> = base.discovery_tx.subscribe();

    let mut last_pos = pos;
    let mut stuck_counter = 0;

    robots.update_carrying(id, None).await;

    loop {
        if pos == last_pos {
            stuck_counter += 1;
        } else {
            stuck_counter = 0;
            last_pos = pos;
        }

        if stuck_counter > 25 {
            if let Some(tgt) = local_targets.pop_front() {
                base.release_resource(tgt);
            }
            stuck_counter = 0;
            continue;
        }

        if carrying.is_some() {
            let step = map.next_step_towards(pos, map.base_pos);
            pos = step;
            robots.update_pos(id, pos).await;

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

        if local_targets.is_empty() {
            if let Some(((tx, ty), _cell)) = base.get_next_resource() {
                local_targets.push_back((tx, ty));
            } else {
                if let Ok(((tx, ty), _cell)) = rx.try_recv() {
                    if base.try_reserve_resource((tx, ty)) {
                        local_targets.push_back((tx, ty));
                    }
                }
            }

            // Si aucune cible après tentative d'obtention: retourner à la base pour ne pas rester sur place
            if local_targets.is_empty() && pos != map.base_pos {
                let step = map.next_step_towards(pos, map.base_pos);
                pos = step;
                robots.update_pos(id, pos).await;
                sleep(Duration::from_millis(80)).await;
                continue;
            }
        }

        if let Some(target) = local_targets.front().cloned() {
            let step = map.next_step_towards(pos, target);
            pos = step;
            robots.update_pos(id, pos).await;

            if pos == target {
                if let Some(collected) = map.try_collect_one(pos.0, pos.1) {
                    carrying = Some(collected);
                    robots.update_carrying(id, carrying).await;
                    let _ = base.to_base_tx.send(MessageToBase::Collected {
                        kind: collected,
                        amount: 1,
                    }).await;
                } else {
                    base.remove_known_resource(target);
                    base.release_resource(target);
                    local_targets.pop_front();
                }
            }

            sleep(Duration::from_millis(80)).await;
            continue;
        }

        // Toujours se replacer à la base lorsqu'aucune mission n'est disponible
        if pos != map.base_pos {
            let step = map.next_step_towards(pos, map.base_pos);
            pos = step;
            robots.update_pos(id, pos).await;
            sleep(Duration::from_millis(80)).await;
            continue;
        }

        sleep(Duration::from_millis(150)).await;
    }
}

