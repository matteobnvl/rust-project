use crossterm::event::{self, Event};
use ratatui::DefaultTerminal;
use std::fmt::Display;
use std::time::{Duration, Instant};
use tracing::{debug, info};

mod utils;        // ⬅️ ton fichier utils.rs tel quel
mod map;
mod base;
mod robots;
mod simulation;
mod ui;

use base::BaseShared;
use map::Map;
use robots::{RobotKind, RobotsShared};
use simulation::{spawn_simulation, SimulationError, Result};

fn main() -> Result<()> {
    let _guard = utils::configure_logger();
    info!("Starting simulation…");

    // --- Init terminal & map ---
    let mut terminal = ratatui::init();
    let area = terminal.size().expect("terminal size");
    let area = terminal.size().expect("terminal size");
    let mut map = Map::from_area(ratatui::layout::Size {
        width: area.width,
        height: area.height,
    });

    // --- Shared state (base + robots) ---
    let base_shared = BaseShared::new();
    let robots_shared = RobotsShared::new();

    // --- Spawn async simulation (Tokio runtime) ---
    //   - robots (scouts/collectors)
    //   - base system (connaissances + totaux)
    let sim_handles = spawn_simulation(&mut map, &base_shared, &robots_shared)?;

    // --- Event loop & render ---
    run_ui_loop(&mut terminal, &mut map, &base_shared, &robots_shared)?;

    // --- Stop simulation tasks gracefully ---
    info!("Stopping simulation…");
    sim_handles.shutdown();

    ratatui::restore();
    info!("Exited cleanly.");
    Ok(())
}

fn run_ui_loop(
    terminal: &mut DefaultTerminal,
    map: &mut Map,
    base_shared: &BaseShared,
    robots_shared: &RobotsShared,
) -> Result<()> {
    const TICK_RATE: Duration = Duration::from_millis(80);
    let mut last_tick = Instant::now();

    loop {
        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        // Quitter sur une touche
        if event::poll(timeout)? {
            if let Event::Key(_) = event::read()? {
                break Ok(());
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            last_tick = Instant::now();
            debug!("tick");
        }

        terminal.draw(|f| ui::render(f, map, base_shared, robots_shared))?;
    }
}
