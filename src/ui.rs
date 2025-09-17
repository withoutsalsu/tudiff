use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    },
    Frame, Terminal,
};

use crate::app::{App, AppMode, CopyInfo, FilterMode};
use crate::compare::FileStatus;
use crate::utils::{format_file_size, format_modified_time, truncate_path};

pub fn draw_ui<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    terminal.draw(|f| match app.mode {
        AppMode::DirectoryView => draw_directory_view(f, app),
        AppMode::FileView => draw_file_view(f, app),
        AppMode::CopyConfirm => {
            draw_directory_view(f, app);
            draw_copy_confirm_popup(f, app);
        }
    })?;
    Ok(())
}

fn draw_directory_view(f: &mut Frame, app: &mut App) {
    app.viewport_height = f.area().height;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    app.toolbar_area = main_chunks[0];

    draw_toolbar(f, app, main_chunks[0]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    let panel_width = chunks[0].width.saturating_sub(2) as usize;

    draw_left_panel(f, app, chunks[0], panel_width);
    draw_right_panel(f, app, chunks[1], panel_width);

    if app.is_refreshing {
        draw_progress_popup(f, app);
    }
}

fn draw_toolbar(f: &mut Frame, app: &App, area: Rect) {
    let toolbar_items = vec![Line::from(vec![
        Span::styled("📁", Style::default().fg(Color::Yellow)),
        Span::raw(" All Files"),
        Span::raw("("),
        Span::styled("1", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("🔍", Style::default().fg(Color::Cyan)),
        Span::raw(" Different"),
        Span::raw("("),
        Span::styled("2", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("⚡", Style::default().fg(Color::Magenta)),
        Span::raw(" Diff Only"),
        Span::raw("("),
        Span::styled("3", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("📂", Style::default().fg(Color::Green)),
        Span::raw(" Expand All"),
        Span::raw("("),
        Span::styled("+", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("📁", Style::default().fg(Color::Blue)),
        Span::raw(" Collapse All"),
        Span::raw("("),
        Span::styled("-", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("🔄", Style::default().fg(Color::Magenta)),
        Span::raw(" Refresh"),
        Span::raw("("),
        Span::styled("F5", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled("🔃", Style::default().fg(Color::Red)),
        Span::raw(" Swap Panels"),
        Span::raw("("),
        Span::styled("s", Style::default().fg(Color::Red)),
        Span::raw(")"),
        Span::raw(" │ "),
        if app.can_copy() {
            if app.active_panel == 0 {
                Span::styled("▶️", Style::default().fg(Color::Green))
            } else {
                Span::styled("◀️", Style::default().fg(Color::Green))
            }
        } else {
            if app.active_panel == 0 {
                Span::styled("▶️", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled("◀️", Style::default().fg(Color::DarkGray))
            }
        },
        if app.can_copy() {
            Span::styled("Copy", Style::default().fg(Color::White))
        } else {
            Span::styled("Copy", Style::default().fg(Color::DarkGray))
        },
        Span::raw("("),
        if app.can_copy() {
            if app.active_panel == 0 {
                Span::styled("Ctrl+R", Style::default().fg(Color::Red))
            } else {
                Span::styled("Ctrl+L", Style::default().fg(Color::Red))
            }
        } else {
            if app.active_panel == 0 {
                Span::styled("Ctrl+R", Style::default().fg(Color::DarkGray))
            } else {
                Span::styled("Ctrl+L", Style::default().fg(Color::DarkGray))
            }
        },
        Span::raw(")"),
        Span::raw(" │ "),
        Span::styled(
            "Filter: ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            match app.filter_mode {
                FilterMode::All => "All Files",
                FilterMode::Different => "Different Only",
                FilterMode::DifferentNotOrphans => "Diff Only (No Orphans)",
            },
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let toolbar = Paragraph::new(toolbar_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" 🛠️  Tools ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .alignment(Alignment::Left);
    f.render_widget(toolbar, area);
}

fn draw_left_panel(f: &mut Frame, app: &mut App, area: Rect, panel_width: usize) {
    let left_items: Vec<ListItem> = create_list_items(&app.left_items, panel_width);

    let left_list = List::new(left_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Left: {}", app.comparison.left_dir.display()))
                .border_style(if app.active_panel == 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        )
        .highlight_style(if app.active_panel == 0 {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else {
            Style::default().bg(Color::Rgb(60, 60, 80)).fg(Color::White)
        });

    f.render_stateful_widget(left_list, area, &mut app.left_list_state);

    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.left_scrollbar_state,
    );
}

fn draw_right_panel(f: &mut Frame, app: &mut App, area: Rect, panel_width: usize) {
    let right_items: Vec<ListItem> = create_list_items(&app.right_items, panel_width);

    let right_list = List::new(right_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Right: {}", app.comparison.right_dir.display()))
                .border_style(if app.active_panel == 1 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }),
        )
        .highlight_style(if app.active_panel == 1 {
            Style::default().bg(Color::Yellow).fg(Color::Black)
        } else {
            Style::default().bg(Color::Rgb(60, 60, 80)).fg(Color::White)
        });

    f.render_stateful_widget(right_list, area, &mut app.right_list_state);

    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut app.right_scrollbar_state,
    );
}

fn create_list_items(
    items: &[(
        String,
        FileStatus,
        std::path::PathBuf,
        bool,
        Option<u64>,
        Option<std::time::SystemTime>,
    )],
    panel_width: usize,
) -> Vec<ListItem<'_>> {
    items
        .iter()
        .map(|(display_name, status, _, is_dir, size, modified)| {
            if *is_dir && !display_name.trim().is_empty() {
                let trimmed = display_name.trim_start();
                let indent_len = display_name.len() - trimmed.len();
                let indent = &display_name[..indent_len];

                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let icon = parts[0];
                    let folder_name = parts[1];

                    let text_color = match status {
                        FileStatus::Same => Color::White,
                        FileStatus::Different => Color::Red,
                        FileStatus::LeftOnly => Color::Blue,
                        FileStatus::RightOnly => Color::Blue,
                    };

                    let line = Line::from(vec![
                        Span::raw(indent),
                        Span::raw(icon),
                        Span::raw(" "),
                        Span::styled(folder_name, Style::default().fg(text_color)),
                    ]);
                    return ListItem::new(line);
                }
            }

            let color = match status {
                FileStatus::Same => Color::Gray,
                FileStatus::Different => Color::LightRed,
                FileStatus::LeftOnly => Color::LightBlue,
                FileStatus::RightOnly => Color::LightBlue,
            };

            if !*is_dir && !display_name.trim().is_empty() {
                let size_str = format_file_size(*size);
                let modified_str = format_modified_time(*modified);

                let total_width = panel_width;
                let name_width = display_name.len();
                let info_width = size_str.len() + 1 + modified_str.len();

                if name_width + info_width + 2 <= total_width {
                    let padding_width = total_width - name_width - info_width;
                    let padding = " ".repeat(padding_width);

                    let line = Line::from(vec![
                        Span::styled(display_name, Style::default().fg(color)),
                        Span::raw(padding),
                        Span::styled(size_str, Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(modified_str, Style::default().fg(Color::DarkGray)),
                    ]);
                    ListItem::new(line)
                } else {
                    ListItem::new(Line::from(Span::styled(
                        display_name,
                        Style::default().fg(color),
                    )))
                }
            } else {
                ListItem::new(Line::from(Span::styled(
                    display_name,
                    Style::default().fg(color),
                )))
            }
        })
        .collect()
}

fn draw_progress_popup(f: &mut Frame, app: &App) {
    let popup_area = centered_rect(50, 20, f.area());

    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title(" 🔄 새로고침 진행 중... ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let popup_inner = popup_block.inner(popup_area);
    f.render_widget(popup_block, popup_area);

    let popup_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(popup_inner);

    let message = Paragraph::new(app.refresh_progress.clone())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    f.render_widget(message, popup_chunks[0]);

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .percent((app.refresh_percentage * 100.0) as u16)
        .label(format!("{:.1}%", app.refresh_percentage * 100.0));
    f.render_widget(progress, popup_chunks[1]);

    let help = Paragraph::new("Press ESC to cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(help, popup_chunks[2]);
}

fn draw_copy_confirm_popup(f: &mut Frame, app: &App) {
    if let Some(copy_info) = &app.copy_info {
        let popup_area = if copy_info.from_left_to_right {
            panel_centered_rect(50, 25, f.area(), true)
        } else {
            panel_centered_rect(50, 25, f.area(), false)
        };

        f.render_widget(Clear, popup_area);

        let title = if copy_info.from_left_to_right {
            " ▶️ Copy to RIGHT panel "
        } else {
            " ◀️ Copy to LEFT panel "
        };

        let popup_block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let popup_inner = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);

        let popup_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(popup_inner);

        draw_copy_paths(f, copy_info, popup_chunks[1], popup_area.width);
        draw_copy_info(f, copy_info, popup_chunks[3]);
        draw_copy_buttons(f, popup_chunks[5]);
    }
}

fn draw_copy_paths(f: &mut Frame, copy_info: &CopyInfo, area: Rect, popup_width: u16) {
    let max_path_width = popup_width.saturating_sub(4) as usize;
    let left_path = truncate_path(&copy_info.source_path.display().to_string(), max_path_width);
    let right_path = truncate_path(&copy_info.target_path.display().to_string(), max_path_width);

    let paths = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            left_path,
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![Span::styled("to", Style::default().fg(Color::Gray))]),
        Line::from(vec![Span::styled(
            right_path,
            Style::default().fg(Color::Yellow),
        )]),
    ])
    .alignment(Alignment::Center);
    f.render_widget(paths, area);
}

fn draw_copy_info(f: &mut Frame, copy_info: &CopyInfo, area: Rect) {
    let file_text = if copy_info.file_count == 1 {
        format!("{} file", copy_info.file_count)
    } else {
        format!("{} files", copy_info.file_count)
    };

    let folder_text = if copy_info.folder_count == 0 {
        "".to_string()
    } else if copy_info.folder_count == 1 {
        format!("{} folder", copy_info.folder_count)
    } else {
        format!("{} folders", copy_info.folder_count)
    };

    let size_text = format_file_size(Some(copy_info.total_bytes));

    let mut info_lines = vec![Line::from(vec![Span::styled(
        file_text,
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )])];

    if !folder_text.is_empty() {
        info_lines.push(Line::from(vec![Span::styled(
            folder_text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]));
    }

    info_lines.push(Line::from(vec![Span::styled(
        size_text,
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )]));

    let info = Paragraph::new(info_lines).alignment(Alignment::Center);
    f.render_widget(info, area);
}

fn draw_copy_buttons(f: &mut Frame, area: Rect) {
    let buttons = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" - OK    "),
        Span::styled(
            "Esc",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" - Cancel"),
    ])])
    .alignment(Alignment::Center);
    f.render_widget(buttons, area);
}

fn draw_file_view(f: &mut Frame, app: &mut App) {
    let paragraph = Paragraph::new(app.file_diff.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("File Diff (Press Enter to go back)"),
    );

    f.render_widget(paragraph, f.area());
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn panel_centered_rect(percent_x: u16, percent_y: u16, r: Rect, left_panel: bool) -> Rect {
    let content_area = Rect {
        x: r.x,
        y: r.y + 3,
        width: r.width,
        height: r.height.saturating_sub(3),
    };

    let panel_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(content_area);

    let target_panel = if left_panel {
        panel_chunks[0]
    } else {
        panel_chunks[1]
    };

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(target_panel);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
