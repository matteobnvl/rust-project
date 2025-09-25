use std::fmt::Display;

use crossterm::event::{self, Event};
use rand::{SeedableRng, rngs::StdRng};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

mod utils;

struct GameState {}

impl GameState {
    pub fn new() -> Self {
        Self {}
    }
    pub fn update(&mut self) {
        todo!()
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
    const REAPTED_SEED: [u8; 32] = [0; 32];
    let mut rng = StdRng::from_seed(REAPTED_SEED);
    let terminal = ratatui::init();
    let mut game_state = GameState::new();
    run(terminal, &mut game_state)?;
    ratatui::restore();
    Ok(())
}

fn run(mut terminal: DefaultTerminal, game_state: &mut GameState) -> Result<()> {
    const TICK_RATE: Duration = Duration::from_millis(50); // 20 updates per second

    let mut last_tick = Instant::now();

    // Configure crossterm to not block on event reading
    event::poll(Duration::from_millis(0)).map_err(SimulationError::Io)?;

    loop {
        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout).map_err(SimulationError::Io)? {
            if let Event::Key(_) = event::read().map_err(SimulationError::Io)? {
                break Ok(());
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            game_state.update();
            last_tick = Instant::now();
        }

        terminal
            .draw(|f| render(f, game_state))
            .map_err(SimulationError::Io)?;
    }
}

fn render(f: &mut ratatui::Frame<'_>, game_state: &GameState) {
    todo!()
}
