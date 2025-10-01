use noise::{NoiseFn, Perlin};
use crate::SimulationError;
use rand::prelude::*;

#[derive(Clone)]
pub enum Tile {
    Wall,
    Floor,
    Source,
    Cristal,
    Base,
}

// pub struct Map {
//     width: u16,
//     height: u16,
//     tiles: Vec<Vec<Tile>>,
// }



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

// pub fn generate_sources_noise(width: u16, height: u16) -> Result<Vec<(u16, u16, Tile)>, SimulationError> {
//     let perlin = Perlin::new(65899529);
//     let mut sources_quantity = rand::thread_rng().gen_range(50..200);
//     let mut cristal_quantity = rand::thread_rng().gen_range(50..200);
//     let scale = 0.4;
//     let mut sources: Vec<(u16, u16, Tile)> = Vec::new();
//     for y in 0..height {
//         for x in 0..width {
//             let noise_val = perlin.get([x as f64 * scale, y as f64 * scale, 100.0]);
//             if  noise_val > 0.6 && sources_quantity > 0 {
//                 sources_quantity -= 1;
//                 sources.push((x, y, Tile::Source));
//             }
//             if noise_val < -0.6 && cristal_quantity > 0 {
//                 cristal_quantity -= 1;
//                 sources.push((x, y, Tile::Cristal));
//             }
//         }
//     }
//     Ok(sources)
// }