use std::fmt::Display;
use rand::Rng;
use crossterm::event::{self, Event};
use rand::{SeedableRng, rngs::StdRng};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};
use ratatui::widgets::{Paragraph, Block, Borders};
use ratatui::text::{Span, Line};
use ratatui::layout::{Rect, Size};
use noise::{NoiseFn, Perlin};
use ratatui::style::{Style, Color};

mod utils;

#[derive(Clone, Copy)]
enum Cell {
    Empty,
    Obstacle,
    Energy(u32),
    Crystal(u32),
    Base,
}

#[derive(Clone)]
struct Map {
    width: usize,
    height: usize,
    grid: Vec<Vec<Cell>>,
}

impl Map {
    pub fn from_area(area: Size) -> Self {
        let width = area.width as usize;
        let height = area.height as usize;

        let mut grid = vec![vec![Cell::Empty; width]; height];

        let perlin = Perlin::new(2);

        for y in 0..height {
            for x in 0..width {
                let value = perlin.get([x as f64 / 10.0, y as f64 / 10.0]);
                if value > 0.2 {
                    grid[y][x] = Cell::Obstacle;
                }
            }
        }

        let center_x = width / 2;
        let center_y = height / 2;

        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = (center_x as isize + dx) as usize;
                let y = (center_y as isize + dy) as usize;
                if x < width && y < height {
                    grid[y][x] = Cell::Empty;
                }
            }
        }

        for dy in -1..=1 {
            for dx in -1..=1 {
                let x = (center_x as isize + dx) as usize;
                let y = (center_y as isize + dy) as usize;
                if x < width && y < height {
                    grid[y][x] = Cell::Base;
                }
            }
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();

        for y in 0..height {
            for x in 0..width {
                if let Cell::Empty = grid[y][x] {
                    if (x as isize - center_x as isize).abs() <= 1
                        && (y as isize - center_y as isize).abs() <= 1
                    {
                        continue;
                    }

                    let roll: f64 = rng.r#gen();
                    if roll < 0.005 {
                        let qty = rng.gen_range(50..=200);
                        grid[y][x] = Cell::Energy(qty);
                    } else if roll < 0.008 {
                        let qty = rng.gen_range(50..=200);
                        grid[y][x] = Cell::Crystal(qty);
                    }
                }
            }
        }

        Self { width, height, grid }
    }
}

struct GameState {
    map: Map
}

impl GameState {
    pub fn new(area: Size) -> Self {
        let map = Map::from_area(area);
        Self { map }
    }

    pub fn update(&mut self) {
        // plus tard : mouvements robots etc.
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
    let terminal = ratatui::init();
    let area = terminal.size().unwrap();
    let mut game_state = GameState::new(area);
    run(terminal, &mut game_state)?;
    ratatui::restore();
    Ok(())
}

fn run(mut terminal: DefaultTerminal, game_state: &mut GameState) -> Result<()> {
    const TICK_RATE: Duration = Duration::from_millis(50); // 20 updates per second

    let mut last_tick = Instant::now();

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
    let area = f.area();

    let mut lines: Vec<Line> = Vec::new();
    for row in &game_state.map.grid {
        let mut spans: Vec<Span> = Vec::new();
        for cell in row {
            let span = match cell {
                Cell::Empty => Span::raw(" "),
                Cell::Obstacle => Span::styled("O", Style::default().fg(Color::Cyan)),
                Cell::Energy(_) => Span::styled("E", Style::default().fg(Color::Green)),
                Cell::Crystal(_) => Span::styled("C", Style::default().fg(Color::Magenta)),
                Cell::Base => Span::styled("#", Style::default().fg(Color::LightGreen)),
            };
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }

    let widget = Paragraph::new(lines);
    f.render_widget(widget, area);
}

