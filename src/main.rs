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
    _base: base::SharedBase,
    pub energy: u32,
    pub crystals: u32,
    pub rx_broadcast: tokio::sync::broadcast::Receiver<base::BroadcastMessage>,
    pub tx_base: mpsc::Sender<base::BaseMessage>,
}

impl GameState {
    pub fn new(
        map: Vec<Vec<map::Tile>>,
        width: u16,
        height: u16,
        robots: Vec<robot::Robot>,
        base: base::SharedBase,
        rx_broadcast: tokio::sync::broadcast::Receiver<base::BroadcastMessage>,
        tx_base: mpsc::Sender<base::BaseMessage>,
    ) -> Self {
        Self {
            map,
            width,
            height,
            robots,
            map_discovered: HashMap::new(),
            _base: base,
            energy: 0,
            crystals: 0,
            rx_broadcast,
            tx_base
        }
    }
    
    pub fn update(&mut self) {
        
        for robot in &mut self.robots {
            if robot.robot_type == robot::RobotType::Eclaireur {
                robot::move_robot(robot, &mut self.map, self.width, self.height);
                self.map_discovered.extend(robot.map_discovered.iter().map(|(x, y)| (*x, y.clone())));
            }
        }

        let resources_left = self.map_discovered
            .values()
            .filter(|t| matches!(t, map::Tile::SourceFound(qty) | map::Tile::CristalFound(qty) if *qty > 0))
            .count();

        tracing::info!("🔎 Ressources encore présentes: {}", resources_left);

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
                    for ((x, y), tile) in self.map_discovered.clone() {
                        match self.map[y as usize][x as usize] {
                            map::Tile::Explored => {
                                self.map_discovered.insert((x, y), map::Tile::Explored);
                            }
                            map::Tile::SourceFound(qty) | map::Tile::CristalFound(qty) if qty == 0 => {
                                self.map_discovered.insert((x, y), map::Tile::Explored);
                            }
                            _ => {}
                        }
                    }

                    if let Some(new_target) = robot::find_nearest_resource(robot, &self.map_discovered, &reserved_positions) {
                        robot.target_resource = Some(new_target);
                        reserved_positions.insert((new_target.0, new_target.1));
                        tracing::info!("🎯 Nouvelle cible assignée : {:?}", new_target);
                    }
                }



                if let Some(_target) = robot.target_resource {
                    let tx_base = self.tx_base.clone();
                    let before = robot.target_resource;
                    robot::collect_resources(robot, &mut self.map, self.width, self.height, &tx_base, &mut reserved_positions);

                    if let Some(target) = before {
                        if matches!(self.map[target.1 as usize][target.0 as usize], map::Tile::Explored) {
                            self.map_discovered.insert((target.0, target.1), map::Tile::Explored);
                        }
                    }

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
    let (tx_broadcast, mut rx_broadcast) = broadcast::channel::<base::BroadcastMessage>(1024);
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
    let mut game_state = GameState::new(map, area.width, area.height, vec![robot1, robot2, robot3, robot4], base, rx_broadcast, tx_base.clone(),);

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

        while let Ok(msg) = game_state.rx_broadcast.try_recv() {
            if let base::BroadcastMessage::BaseStats { energy, crystals } = msg {
                game_state.energy = energy;
                game_state.crystals = crystals;
                tracing::info!("🏆 Score mis à jour : énergie = {}, cristaux = {}", energy, crystals);
            }
        }

        last_tick = Instant::now();

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

fn render_map_simple(f: &mut Frame<'_>, game_state: &GameState, area: Size) {
     let score_text = vec![
        Line::from(vec![
            Span::styled("Énergie: ", Style::default().fg(Color::Green)),
            Span::styled(game_state.energy.to_string(), Style::default().fg(Color::White)),
            Span::raw("   "),
            Span::styled("Cristaux: ", Style::default().fg(Color::Magenta)),
            Span::styled(game_state.crystals.to_string(), Style::default().fg(Color::White)),
        ])
    ];
    let score_widget = Paragraph::new(score_text);
    f.render_widget(score_widget, Rect::new(0, 0, area.width, 1));

    let map_lines: Vec<Line> = game_state.map.iter()
        .enumerate()
        .take((game_state.height.saturating_sub(1)) as usize)
        .map(|(y, row)| {
            let spans: Vec<Span> = row.iter()
                .enumerate()
                .take(game_state.width as usize)
                .map(|(x, tile)| {
                    let robot_here = game_state.robots.iter()
                        .find(|r| r.position.0 == x as u16 && r.position.1 == y as u16);
                    
                    let (ch, color) = if let Some(robot) = robot_here {
                        match robot.robot_type {
                            robot::RobotType::Eclaireur => ('X', Color::Red),
                            robot::RobotType::Collecteur => ('Y', Color::White),
                        }
                    } else {
                        match tile {
                            map::Tile::Wall => ('0', Color::LightCyan),
                            map::Tile::Floor => (' ', Color::Reset),
                            map::Tile::Source(qty) => ('E', Color::Green),
                            map::Tile::SourceFound(qty) => {
                                if *qty > 0 { ('E', Color::Blue) } 
                                else { ('░', Color::Gray) }
                            },
                            map::Tile::Cristal(qty) => ('C', Color::LightMagenta),
                            map::Tile::CristalFound(qty) => {
                                if *qty > 0 { ('C', Color::Yellow) } 
                                else { ('░', Color::Gray) }
                            },
                            map::Tile::Base => ('#', Color::LightGreen),
                            map::Tile::Eclaireur => ('X', Color::Red),
                            map::Tile::Collecteur => ('Y', Color::White),
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

