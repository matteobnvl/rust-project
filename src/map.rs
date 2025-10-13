use noise::{NoiseFn, Perlin};
use crate::SimulationError;
use rand::prelude::*;
use rand::Rng;

#[derive(Clone)]
#[derive(PartialEq)]
#[derive(Debug)]
pub enum Tile {
    Wall,
    Floor,
    Source(u32),
    Cristal(u32),
    CristalFound(u32),
    SourceFound(u32),
    Base,
    // Eclaireur,
    // Collecteur,
    Explored
}

pub fn generate_map(width: u16, height: u16) -> Result<Vec<Vec<Tile>>, SimulationError> {
    let perlin = Perlin::new(21);
    let scale = 0.1;
    let map = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    let noise_val = perlin.get([x as f64 * scale, y as f64 * scale, 0.0]);
                    if noise_val < 0.3 {
                        Tile::Floor
                    } else {
                        Tile::Wall
                    }
                })
                .collect::<Vec<Tile>>()
        })
        .collect::<Vec<Vec<Tile>>>();
    Ok(map)
}

pub fn generate_sources_rand(width: u16, height: u16) -> Result<Vec<(u16, u16, Tile)>, SimulationError> {
    let mut rng = rand::thread_rng();
    let mut sources: Vec<(u16, u16, Tile)> = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let roll: f64 = rng.r#gen(); // nombre entre 0.0 et 1.0

            if roll < 0.005 {
                // 💡 0.5% de chances d’être une source d’énergie
                let qty = rng.gen_range(5..=10);
                sources.push((x, y, Tile::Source(qty)));
            } else if roll < 0.008 {
                // 💡 0.3% de chances d’être un cristal
                let qty = rng.gen_range(5..=10);
                sources.push((x, y, Tile::Cristal(qty)));
            }
        }
    }

    Ok(sources)
}