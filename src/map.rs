use noise::{NoiseFn, Perlin};
use crate::SimulationError;
use rand::prelude::*;

#[derive(Clone)]
#[derive(PartialEq)]
pub enum Tile {
    Wall,
    Floor,
    Source,
    Cristal,
    CristalFound,
    SourceFound,
    Base,
    Eclaireur,
    Collecteur,
    Explored
}


pub fn generate_map(width: u16, height: u16) -> Result<Vec<Vec<Tile>>, SimulationError> {
    let perlin = Perlin::new(65899529);
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
    let energy_quantity = rand::thread_rng().gen_range(50..200);
    let cristal_quantity = rand::thread_rng().gen_range(50..200);
    let mut sources: Vec<(u16, u16, Tile)> = Vec::new();
    for _ in 0..energy_quantity {
        let x = rand::thread_rng().gen_range(0..width);
        let y = rand::thread_rng().gen_range(0..height);
        sources.push((x, y, Tile::Source));
    }
    for _ in 0..cristal_quantity {
        let x = rand::thread_rng().gen_range(0..width);
        let y = rand::thread_rng().gen_range(0..height);
        sources.push((x, y, Tile::Cristal));
    }
    Ok(sources)
}