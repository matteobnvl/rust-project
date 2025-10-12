use noise::{NoiseFn, Perlin};
use rand::Rng;
use ratatui::layout::Size;
use std::cmp::{max, min};
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Obstacle,
    Energy(u32),
    Crystal(u32),
    Base,
}

#[derive(Clone)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub grid: Arc<RwLock<Vec<Vec<Cell>>>>,
    pub base_pos: (usize, usize),
}

impl Map {
    pub fn from_area(area: Size) -> Self {
        let width = area.width.max(20) as usize;
        // On réserve une ligne en plus pour la barre de titre → carte - 1 ligne
        let height = area.height.saturating_sub(1).max(10) as usize;

        let mut grid = vec![vec![Cell::Empty; width]; height];
        let perlin = Perlin::new(2);

        // Obstacles (Perlin)
        for y in 0..height {
            for x in 0..width {
                let v = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                if v > 0.2 {
                    grid[y][x] = Cell::Obstacle;
                }
            }
        }

        // Base au centre (3x3)
        let cx = width / 2;
        let cy = height / 2;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = (cx as isize + dx) as usize;
                let y = (cy as isize + dy) as usize;
                if x < width && y < height {
                    grid[y][x] = Cell::Base;
                }
            }
        }

        // Dégager base (Empty sous-jacent)
        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = (cx as isize + dx) as usize;
                let y = (cy as isize + dy) as usize;
                if x < width && y < height {
                    grid[y][x] = Cell::Base;
                }
            }
        }

        // Ressources aléatoires
        let mut rng = rand::thread_rng();
        for y in 0..height {
            for x in 0..width {
                // Ne place pas sur la base
                if (x as isize - cx as isize).abs() <= 1 && (y as isize - cy as isize).abs() <= 1 {
                    continue;
                }
                if matches!(grid[y][x], Cell::Empty) {
                    let roll: f64 = rng.r#gen();
                    if roll < 0.006 {
                        grid[y][x] = Cell::Energy(rng.gen_range(50..=200));
                    } else if roll < 0.010 {
                        grid[y][x] = Cell::Crystal(rng.gen_range(50..=200));
                    }
                }
            }
        }

        Self {
            width,
            height,
            grid: Arc::new(RwLock::new(grid)),
            base_pos: (cx, cy),
        }
    }

    pub fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    pub fn is_walkable(cell: &Cell) -> bool {
        !matches!(cell, Cell::Obstacle)
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Cell {
        let g = self.grid.read().unwrap();
        g[y][x]
    }

    pub fn set_cell(&self, x: usize, y: usize, cell: Cell) {
        let mut g = self.grid.write().unwrap();
        g[y][x] = cell;
    }

    /// Décrémente une ressource de 1 si présente. Retourne Some(Cell::Energy/Crystal) si succès.
    pub fn try_collect_one(&self, x: usize, y: usize) -> Option<Cell> {
        let mut g = self.grid.write().unwrap();
        match g[y][x] {
            Cell::Energy(q) if q > 0 => {
                let nq = q - 1;
                g[y][x] = if nq == 0 { Cell::Empty } else { Cell::Energy(nq) };
                Some(Cell::Energy(1))
            }
            Cell::Crystal(q) if q > 0 => {
                let nq = q - 1;
                g[y][x] = if nq == 0 { Cell::Empty } else { Cell::Crystal(nq) };
                Some(Cell::Crystal(1))
            }
            _ => None,
        }
    }
}

// ============================================
// 🔥 A* Pathfinding : ajout complet à la suite
// ============================================
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    pos: (usize, usize),
    cost: u32,
    estimated_total_cost: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // inversion pour BinaryHeap (min-heap)
        other.estimated_total_cost.cmp(&self.estimated_total_cost)
            .then_with(|| self.pos.cmp(&other.pos))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Map {
    fn heuristic(a: (usize, usize), b: (usize, usize)) -> u32 {
        ((a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs()) as u32
    }

    fn neighbors(&self, pos: (usize, usize)) -> Vec<(usize, usize)> {
        let (x, y) = pos;
        let mut n = Vec::new();
        let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for (dx, dy) in dirs {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if self.in_bounds(nx, ny) {
                let c = self.get_cell(nx as usize, ny as usize);
                if Self::is_walkable(&c) {
                    n.push((nx as usize, ny as usize));
                }
            }
        }
        n
    }

    pub fn find_path(&self, from: (usize, usize), to: (usize, usize)) -> Option<Vec<(usize, usize)>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut open_set = BinaryHeap::new();
        open_set.push(Node {
            pos: from,
            cost: 0,
            estimated_total_cost: Self::heuristic(from, to),
        });

        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_score: HashMap<(usize, usize), u32> = HashMap::new();
        g_score.insert(from, 0);

        while let Some(current) = open_set.pop() {
            if current.pos == to {
                // chemin trouvé → reconstruction
                let mut path = vec![to];
                let mut cur = to;
                while let Some(prev) = came_from.get(&cur) {
                    path.push(*prev);
                    cur = *prev;
                }
                path.reverse();
                return Some(path);
            }

            for neighbor in self.neighbors(current.pos) {
                let tentative_g = g_score.get(&current.pos).unwrap_or(&u32::MAX) + 1;
                if tentative_g < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                    came_from.insert(neighbor, current.pos);
                    g_score.insert(neighbor, tentative_g);
                    let f_score = tentative_g + Self::heuristic(neighbor, to);
                    open_set.push(Node {
                        pos: neighbor,
                        cost: tentative_g,
                        estimated_total_cost: f_score,
                    });
                }
            }
        }

        None
    }

    pub fn next_step_towards(&self, from: (usize, usize), to: (usize, usize)) -> (usize, usize) {
        if let Some(path) = self.find_path(from, to) {
            if path.len() > 1 {
                return path[1];
            }
        }
        from
    }
}
