use crate::base::BaseShared;
use crate::map::{Cell, Map};
use crate::robots::{RobotKind, RobotsShared};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render(
    f: &mut ratatui::Frame<'_>,
    map: &Map,
    base_shared: &BaseShared,
    robots_shared: &RobotsShared,
) {
    let area = f.area();

    // Titre / barre d’état en haut
    let stats = base_shared.stats.lock().unwrap().clone();
    let robots = futures_lite::future::block_on(robots_shared.snapshot());

    let title = Line::from(vec![
        Span::styled(" Resource Simulation ", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::raw(format!("Energy: {}", stats.energy_total)),
        Span::raw("  "),
        Span::raw(format!("Crystals: {}", stats.crystal_total)),
        Span::raw("  | Robots: "),
        Span::raw(format!("{}", robots.len())),
        Span::raw("  (press any key to quit)"),
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL);

    // Dessin de la carte sous la barre de titre
    let map_rect = inner_map_area(area);

    let mut lines: Vec<Line> = Vec::with_capacity(map.height);
    let grid = map.grid.read().unwrap();

    for y in 0..map.height {
        let mut spans = Vec::with_capacity(map.width);
        for x in 0..map.width {
            let mut span = match grid[y][x] {
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

    drop(grid); // libère le lock lecture

    // Overlay robots
    for r in robots {
        if r.pos.1 < map.height && r.pos.0 < map.width {
            // Remplace le symbole sur la ligne concernée (simple et efficace)
            let ch = match r.kind {
                RobotKind::Scout => ("x", Color::Red),
                RobotKind::Collector => ("o", Color::Magenta),
            };
            if let Some(line) = lines.get_mut(r.pos.1) {
                if let Some(span) = line.spans.get_mut(r.pos.0) {
                    *span = Span::styled(ch.0, Style::default().fg(ch.1));
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, map_rect);
}

fn inner_map_area(area: Rect) -> Rect {
    // On garde l’encadré, la map prend tout l’espace interne
    area
}
