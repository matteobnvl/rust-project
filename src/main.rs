use std::fmt::Display;
use std::collections::HashMap;

use rand::{SeedableRng, rngs::StdRng};
use ratatui::{
    prelude::*,
    crossterm::event::{self, Event, KeyCode},
    layout::Size,
    style::{Color, Style},
    widgets::{Paragraph},
    DefaultTerminal, 
    Frame,
    text::{Span, Line},
};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

mod utils;
mod map;
mod robot;
mod base;

pub struct GameState {
    map: Vec<Vec<map::Tile>>,
    width: u16,
    height: u16,
    robots: Vec<robot::Robot>,
    map_discovered: HashMap<(u16, u16), map::Tile>,
    _base: base::SharedBase
}

impl GameState {
    pub fn new(
        map: Vec<Vec<map::Tile>>,
        width: u16,
        height: u16,
        robots: Vec<robot::Robot>,
        base: base::SharedBase
    ) -> Self {
        Self {
            map,
            width,
            height,
            robots,
            map_discovered: HashMap::new(),
            _base: base,
        }
    }
    
    pub fn update(&mut self) {
        
        for robot in &mut self.robots {
            if robot.robot_type == robot::RobotType::Eclaireur {
                robot::move_robot(robot, &mut self.map, self.width, self.height);
                self.map_discovered.extend(robot.map_discovered.iter().map(|(x, y)| (*x, y.clone())));
            }
        }

        let mut reserved_positions: std::collections::HashSet<(u16, u16)> = self
            .robots
            .iter()
            .filter_map(|r| r.target_resource)
            .map(|pos| (pos.0, pos.1))
            .collect();

        for robot in &mut self.robots {
            robot::get_discovered_map(robot, &self.map_discovered);

            if robot.robot_type == robot::RobotType::Collecteur {
                if robot.target_resource.is_none() {
                    if let Some(target_pos) = robot::find_nearest_resource(robot, &self.map_discovered, &reserved_positions) {
                        robot.target_resource = Some(target_pos);
                        reserved_positions.insert((target_pos.0, target_pos.1));
                    }
                }

                if let Some(target) = robot.target_resource {
                    robot::collect_resources(robot, target, &mut self.map, self.width, self.height);
                }
            }
        }


        let base_center = (self.width / 2, self.height / 2);
        
        for dy in -1..=1 {
            for dx in -1..=1 {
                let bx = (base_center.0 as i16 + dx) as usize;
                let by = (base_center.1 as i16 + dy) as usize;
                self.map[by][bx] = map::Tile::Base;
            }
        }

        for robot in &self.robots {
            let robot_position = robot.position;
            let tile = match robot.robot_type {
                robot::RobotType::Collecteur => map::Tile::Collecteur,
                robot::RobotType::Eclaireur => map::Tile::Eclaireur,
            };
            self.map[robot_position.1 as usize][robot_position.0 as usize] = tile;
        }
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

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = utils::configure_logger();
    tracing::info!("Application started!");
    const REPEATED_SEED: [u8; 32] = [0; 32];
    let mut _rng = StdRng::from_seed(REPEATED_SEED);

    let (tx_base, rx_base) = mpsc::channel::<base::BaseMessage>(1024);
    let (tx_broadcast, _rx_ignore) = broadcast::channel::<base::BroadcastMessage>(1024);
    let base = base::Base::new(tx_broadcast.clone());

    {
        let base_clone = base.clone();
        tokio::spawn(async move {
            base_clone.run(rx_base).await;
        });
    }

    let terminal = ratatui::init();
    let area: Size = terminal.size().map_err(SimulationError::Io)? ;

    let sources = map::generate_sources_rand(area.width, area.height)?;
    let mut map = map::generate_map(area.width, area.height)?;
    let start_x = (area.width / 2) - 1;
    let start_y = (area.height / 2) - 1;

    for y in start_y..start_y + 3 {
        for x in start_x..start_x + 3 {
            map[y as usize][x as usize] = map::Tile::Base;
        }
    }

    sources.iter().for_each(|(x, y, resource)| {
        if let map::Tile::Floor = map[*y as usize][*x as usize] {
            map[*y as usize][*x as usize] = resource.clone();
        }
    });

    let robot1 = robot::robots_eclaireur(area.width, area.height);
    let robot2 = robot::robots_eclaireur(area.width, area.height);

    let robot3 = robot::robots_collecteur(area.width, area.height);
    let robot4 = robot::robots_collecteur(area.width, area.height);

    tracing::info!("Map generated");
    let mut game_state = GameState::new(map, area.width, area.height, vec![robot1, robot2, robot3, robot4], base);

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
            .draw(|f| {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(render_map_simple(f, game_state, area))
                });
            })
            .map_err(SimulationError::Io)?;

    }
}

async fn render_map_simple(f: &mut Frame<'_>, game_state: &GameState, area: Size) {
    use ratatui::widgets::{Block, Borders};

    let map_lines: Vec<Line> = game_state
        .map
        .iter()
        .take(game_state.height as usize)
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .take(game_state.width as usize)
                .map(|tile| {
                    let (ch, color) = match tile {
                        map::Tile::Wall => ('O', Color::LightCyan),
                        map::Tile::Floor => (' ', Color::Reset),
                        map::Tile::Source(_) => ('E', Color::Green),
                        map::Tile::SourceFound(_) => ('E', Color::Blue),
                        map::Tile::Cristal(_) => ('C', Color::LightMagenta),
                        map::Tile::CristalFound(_) => ('C', Color::Yellow),
                        map::Tile::Base => ('#', Color::LightGreen),
                        map::Tile::Eclaireur => ('X', Color::Red),
                        map::Tile::Collecteur => ('H', Color::White),
                        map::Tile::Explored => (' ', Color::Gray),
                    };
                    Span::styled(ch.to_string(), Style::default().fg(color))
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let map_rect = Rect::new(0, 0, area.width, area.height - 2);
    f.render_widget(Paragraph::new(map_lines).block(Block::default().borders(Borders::NONE)), map_rect);

    let (energy, crystals) = game_state._base.totals().await;

    let stats_text = Line::from(vec![
        Span::styled(
            format!("⚡ Énergie: {}", energy),
            Style::default().fg(Color::Green),
        ),
        Span::raw("   |   "),
        Span::styled(
            format!("💎 Cristaux: {}", crystals),
            Style::default().fg(Color::LightMagenta),
        ),
    ]);

    let stats_rect = Rect::new(0, area.height - 2, area.width, 2);
    let stats_widget = Paragraph::new(stats_text).block(Block::default().borders(Borders::TOP));
    f.render_widget(stats_widget, stats_rect);
}
