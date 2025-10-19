use std::fmt::Display;
use rand::{SeedableRng, rngs::StdRng};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::Size,
    prelude::*,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use crate::game_state::GameState;

mod base;
mod map;
mod robot;
mod utils;
mod game_state;

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

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = utils::configure_logger();
    tracing::info!("Application started!");

    // rng et channels setup
    const REPEATED_SEED: [u8; 32] = [0; 32];
    let _rng = StdRng::from_seed(REPEATED_SEED);
    let (tx_base, rx_base) = mpsc::channel::<base::BaseMessage>(1024);
    let (tx_broadcast, rx_broadcast) = broadcast::channel::<base::BroadcastMessage>(1024);

    // base setup
    let base = base::Base::new(tx_broadcast.clone());
    let base_clone = base.clone();
    tokio::spawn(async move {
        base_clone.run(rx_base).await;
    });

    // terminal setup
    let terminal = ratatui::init();
    let area: Size = terminal.size().map_err(SimulationError::Io)?;

    // map generation
    let mut map = map::generate_map(area.width, area.height - 1)?;
    let sources = map::generate_sources_rand(area.width, area.height - 1)?;
    sources.iter().for_each(|(x, y, resource)| {
        if let map::Tile::Floor = map[*y as usize][*x as usize] {
            map[*y as usize][*x as usize] = resource.clone();
        }
    });

    // base center generation
    let start_x = (area.width / 2) - 1;
    let start_y = (area.height / 2) - 1;
    for y in start_y..start_y + 3 {
        for x in start_x..start_x + 3 {
            map[y as usize][x as usize] = map::Tile::Base;
        }
    }

    // robots generation -- A REFACTO
    let robot1 = robot::robots_eclaireur(area.width, area.height, (1, 0));
    let robot2 = robot::robots_eclaireur(area.width, area.height, (0, 1));
    let robot3 = robot::robots_collecteur(area.width, area.height);
    let robot4 = robot::robots_collecteur(area.width, area.height);

    // game configuration -- A REFACTO
    tracing::info!("Map generated");
    let mut game_state = GameState::new(
        map,
        area.width,
        area.height,
        vec![robot1, robot2, robot3, robot4],
        base,
        rx_broadcast,
        tx_base.clone(),
    );

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
        }

        while let Ok(msg) = game_state.rx_broadcast.try_recv() {
            match msg {
                base::BroadcastMessage::BaseStats { energy, crystals } => {
                    game_state.energy = energy;
                    game_state.crystals = crystals;
                }
            }
        }

        last_tick = Instant::now();

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));
        if event::poll(timeout).map_err(SimulationError::Io)?
            && let Event::Key(key_event) = event::read().map_err(SimulationError::Io)?
            && key_event.code == KeyCode::Char(' ')
        {
            tracing::info!("Space key pressed, exiting game loop");
            return Ok(());
        }

        terminal
            .draw(|f| render_map_simple(f, game_state, area))
            .map_err(SimulationError::Io)?;
    }
}

fn render_map_simple(f: &mut Frame<'_>, game_state: &GameState, area: Size) {
    let score_text = vec![Line::from(vec![
        Span::styled("Énergie: ", Style::default().fg(Color::Green)),
        Span::styled(
            game_state.energy.to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw("   "),
        Span::styled("Cristaux: ", Style::default().fg(Color::Magenta)),
        Span::styled(
            game_state.crystals.to_string(),
            Style::default().fg(Color::White),
        ),
    ])];
    let score_widget = Paragraph::new(score_text);
    f.render_widget(score_widget, Rect::new(0, 0, area.width, 1));

    let map_lines: Vec<Line> = game_state
        .map
        .iter()
        .enumerate()
        .take((game_state.height.saturating_sub(1)) as usize)
        .map(|(y, row)| {
            let spans: Vec<Span> = row
                .iter()
                .enumerate()
                .take(game_state.width as usize)
                .map(|(x, tile)| {
                    let robot_here = game_state
                        .robots
                        .iter()
                        .find(|r| r.position.0 == x as u16 && r.position.1 == y as u16);

                    let (ch, color) = if let Some(robot) = robot_here {
                        match robot.robot_type {
                            robot::RobotType::Eclaireur => ('X', Color::Red),
                            robot::RobotType::Collecteur => ('O', Color::Magenta),
                        }
                    } else {
                        match tile {
                            map::Tile::Wall => ('0', Color::LightCyan),
                            map::Tile::Floor => (' ', Color::Reset),
                            map::Tile::Source(_qty) => ('E', Color::Green),
                            map::Tile::SourceFound(qty) => {
                                if *qty > 0 {
                                    ('E', Color::Blue)
                                } else {
                                    ('░', Color::Gray)
                                }
                            }
                            map::Tile::Cristal(_qty) => ('C', Color::LightMagenta),
                            map::Tile::CristalFound(qty) => {
                                if *qty > 0 {
                                    ('C', Color::Yellow)
                                } else {
                                    ('░', Color::Gray)
                                }
                            }
                            map::Tile::Base => ('#', Color::LightGreen),
                            // map::Tile::Eclaireur => ('X', Color::Red),
                            // map::Tile::Collecteur => ('O', Color::Magenta),
                            map::Tile::Explored => ('░', Color::Gray),
                        }
                    };
                    Span::styled(ch.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let map_widget = Paragraph::new(map_lines);
    f.render_widget(map_widget, Rect::new(0, 1, area.width, area.height));
}
