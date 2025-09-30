use noise::{NoiseFn, Perlin};
use crate::SimulationError;

pub enum Tile {
    Wall,
    Floor,
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
                    if noise_val < 0.1 {
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