use std::collections::{HashMap, HashSet};

use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Each line is a Vec of (text, highlighted) spans
pub fn build_graph_lines(
    services: &[(String, Vec<String>)],
    selected: Option<&str>,
) -> Vec<Vec<(String, bool)>> {
    if services.is_empty() {
        return vec![vec![("No services found".to_string(), false)]];
    }

    let dep_map: HashMap<&str, &Vec<String>> = services.iter().map(|(n, d)| (n.as_str(), d)).collect();
    let all_names: Vec<&str> = services.iter().map(|(n, _)| n.as_str()).collect();

    // Find roots (no dependencies)
    let roots: Vec<&str> = all_names
        .iter()
        .filter(|n| dep_map.get(*n).is_none_or(|d| d.is_empty()))
        .copied()
        .collect();

    // Build levels via topological ordering
    let mut levels: Vec<Vec<&str>> = vec![];
    let mut placed: HashSet<&str> = HashSet::new();

    if !roots.is_empty() {
        placed.extend(roots.iter());
        levels.push(roots);
    }

    for _ in 0..services.len() {
        let next_level: Vec<&str> = all_names
            .iter()
            .filter(|n| {
                !placed.contains(*n)
                    && dep_map.get(*n).is_none_or(|deps| deps.iter().all(|d| placed.contains(d.as_str())))
            })
            .copied()
            .collect();
        if next_level.is_empty() { break; }
        placed.extend(next_level.iter());
        levels.push(next_level);
    }

    // Place any remaining (circular deps)
    for name in &all_names {
        if !placed.contains(name) {
            levels.push(vec![name]);
            placed.insert(name);
        }
    }

    // Render
    let mut lines: Vec<Vec<(String, bool)>> = vec![];
    for (i, level) in levels.iter().enumerate() {
        let mut line: Vec<(String, bool)> = vec![];
        for (j, name) in level.iter().enumerate() {
            if j > 0 { line.push(("   ".to_string(), false)); }
            let is_selected = selected == Some(*name);
            line.push((format!("[{}]", name), is_selected));
        }
        lines.push(line);

        if i + 1 < levels.len() {
            let has_arrows = levels[i + 1].iter().any(|next| {
                dep_map.get(next).is_some_and(|deps| deps.iter().any(|d| level.contains(&d.as_str())))
            });
            if has_arrows {
                lines.push(vec![("  │".to_string(), false), ("  ▼".to_string(), false)]);
            }
        }
    }
    lines
}

pub fn render_graph(f: &mut Frame, area: Rect, app: &App) {
    if app.projects.is_empty() {
        f.render_widget(Paragraph::new("No compose projects found"), area);
        return;
    }

    let selected_name = app.selected_container().and_then(|c| {
        c.names.as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
    });

    let mut all_services: Vec<(String, Vec<String>)> = vec![];
    let mut service_images: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for project in &app.projects {
        for svc in &project.services {
            all_services.push((svc.name.clone(), svc.depends_on.clone()));
            if let Some(ref img) = svc.image {
                service_images.insert(svc.name.clone(), img.clone());
            }
        }
    }

    let graph_lines = build_graph_lines(&all_services, selected_name.as_deref());

    let lines: Vec<Line> = graph_lines.iter().map(|spans| {
        let mut result_spans: Vec<Span> = vec![];
        for (text, highlighted) in spans {
            if *highlighted {
                result_spans.push(Span::styled(text.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
            } else {
                result_spans.push(Span::raw(text.clone()));
            }
            // Show image name next to service nodes
            if text.starts_with('[') && text.ends_with(']') {
                let svc_name = &text[1..text.len()-1];
                if let Some(img) = service_images.get(svc_name) {
                    result_spans.push(Span::styled(format!(" ({})", img), Style::default().fg(Color::DarkGray)));
                }
            }
        }
        Line::from(result_spans)
    }).collect();

    f.render_widget(Paragraph::new(lines), area);
}
