use std::fmt::Display;

use crossterm::event::{self, Event, KeyCode};
use rand::{SeedableRng, rngs::StdRng};
use ratatui::DefaultTerminal;
use ratatui::layout::Size;
use ratatui::widgets::{Block, Borders, Paragraph};
use std::time::{Duration, Instant};

mod map;
mod utils;

pub struct GameState {
    map: Vec<Vec<map::Tile>>,
    width: u16,
    height: u16,
}

impl GameState {
    pub fn new(map: Vec<Vec<map::Tile>>, width: u16, height: u16) -> Self {
        Self { map, width, height }
    }

    pub fn update(&mut self) {
        // Logique de mise à jour du jeu
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    Io(#[from] std::io::Error),
}

impl Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type Result<T> = std::result::Result<T, SimulationError>;

fn main() -> Result<()> {
    let _guard = utils::configure_logger();
    tracing::info!("Application started!");
    const REPEATED_SEED: [u8; 32] = [0; 32];
    let mut _rng = StdRng::from_seed(REPEATED_SEED);

    let terminal = ratatui::init();
    let area: Size = terminal.size().map_err(SimulationError::Io)?;
    let sources = map::generate_sources_noise(area.width, area.height)?;
    let mut map = map::generate_map(area.width, area.height)?;

    sources.iter().for_each(|(x, y, resource)| {
        if let map::Tile::Floor = map[*y as usize][*x as usize] {
            map[*y as usize][*x as usize] = resource.clone();
        }
    });

    tracing::info!("Map generated");
    let mut game_state = GameState::new(map, area.width, area.height);

    tracing::info!("Game state initialized");

    let res = run(terminal, &mut game_state, area);
    tracing::info!("Game loop exited");
    ratatui::restore();
    res
}

fn run(mut terminal: DefaultTerminal, game_state: &mut GameState, area: Size) -> Result<()> {
    const TICK_RATE: Duration = Duration::from_millis(50);

    let mut last_tick = Instant::now();
    event::poll(Duration::from_millis(0)).map_err(SimulationError::Io)?;
    tracing::info!("Crossterm configured");
    loop {
        if last_tick.elapsed() >= TICK_RATE {
            game_state.update();
            last_tick = Instant::now();
        }

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));
        if event::poll(timeout).map_err(SimulationError::Io)? {
            if let Event::Key(key_event) = event::read().map_err(SimulationError::Io)? {
                if key_event.code == KeyCode::Char(' ') {
                    tracing::info!("Space key pressed, exiting game loop");
                    return Ok(());
                }
            }
        }

        terminal
            .draw(|f| render_map_simple(f, game_state, area))
            .map_err(SimulationError::Io)?;
    }
}

fn render_map_simple(f: &mut ratatui::Frame<'_>, game_state: &GameState, area: Size) {
    let map_content = game_state
        .map
        .iter()
        .take(game_state.height as usize)
        .map(|row| {
            row.iter()
                .take(game_state.width as usize)
                .map(|tile| match tile {
                    map::Tile::Wall => 'o',
                    map::Tile::Floor => ' ',
                    map::Tile::Source => 'E',
                    map::Tile::Cristal => 'C',
                })
                .collect::<String>()
        })
        .collect::<Vec<String>>()
        .join("\n");

    let paragraph =
        Paragraph::new(map_content).block(Block::default().borders(Borders::ALL).title("Map"));

    let rect = ratatui::layout::Rect::new(0, 0, area.width, area.height);
    f.render_widget(paragraph, rect);
}
