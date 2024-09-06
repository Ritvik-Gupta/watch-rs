use chrono::{DateTime, Local, Timelike};
use crossbeam_channel::Receiver;
use crossterm::event::{self as term_event, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{
        palette::tailwind::{self, Palette},
        Modifier, Style, Stylize,
    },
    text::{Span, Text},
    widgets::{block::Position, Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{fmt::Write, time::Instant};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use watch_rs::utils::OpenResult;

use crate::tui::TICK_RATE;

use super::{WatcherIterationOutput, WatcherOutputEvent};

pub struct WatcherTui {
    event_receiver: Receiver<WatcherOutputEvent>,
    should_close_watcher: Arc<AtomicBool>,
    current_event: WatcherIterationOutput,
}

impl WatcherTui {
    pub fn new(
        event_receiver: Receiver<WatcherOutputEvent>,
        should_close_watcher: Arc<AtomicBool>,
    ) -> Self {
        Self {
            event_receiver,
            should_close_watcher,
            current_event: WatcherIterationOutput {
                iteration: 0,
                output: String::new(),
            },
        }
    }

    pub fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> OpenResult<()> {
        use WatcherOutputEvent::*;

        let mut last_tick = Instant::now();

        loop {
            terminal.draw(|f| self.draw_ui(f))?;

            if let Ok(event) = self.event_receiver.try_recv() {
                self.current_event = match event {
                    SetupResult(res) => res,
                    IterationResult(res) => res,
                    End => return Ok(()),
                }
            }

            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if term_event::poll(timeout)? {
                let ev = term_event::read()?;
                if let Event::Key(key) = ev {
                    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                        self.should_close_watcher.store(true, Ordering::Release);
                    }
                }
            }

            if last_tick.elapsed() >= TICK_RATE {
                last_tick = Instant::now();
            }
        }
    }

    fn palette(&self) -> Palette {
        tailwind::LIME
    }

    fn draw_ui(&mut self, f: &mut Frame) {
        let render_time: DateTime<Local> = Local::now();

        // Wrapping block for a group
        // Just draw the block and the group on the same area and build the group
        // with at least a margin of 1
        let area = f.area();

        let time_string = format!(
            "{}.{:0^2}",
            render_time.format("%b %d %H:%M:%S"),
            render_time.nanosecond() / 10u32.pow(7)
        );

        // Surrounding block
        let block = Block::default()
            .borders(Borders::TOP | Borders::RIGHT)
            .title(format!("  {time_string}  ").fg(self.palette().c200))
            .title_alignment(Alignment::Right)
            .border_type(BorderType::Rounded);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
            .split(area);

        // Top right inner block with styled title aligned to the right
        let block = Block::default()
            .title(Span::styled(
                format!("  Itr: {}  ", self.current_event.iteration),
                Style::default()
                    .fg(self.palette().c200)
                    .bg(self.palette().c900)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Right);

        let para = Paragraph::new(Text::raw(&self.current_event.output))
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, chunks[0]);

        // Bottom two inner blocks
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(85), Constraint::Percentage(15)])
            .split(chunks[1]);

        // Bottom left block with all default borders
        let block = Block::default().title("With borders").borders(Borders::ALL);
        f.render_widget(block, bottom_chunks[0]);

        let mut extra_info = String::new();
        if let Ok(timezone) = iana_time_zone::get_timezone() {
            write!(&mut extra_info, " ⌛ {timezone}").unwrap();
        }

        // Bottom right block with styled left and right border
        let block = Block::default()
            .title(extra_info)
            .title_alignment(Alignment::Center)
            .title_position(Position::Top)
            .border_style(Style::default().fg(self.palette().c500))
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Thick);
        f.render_widget(block, bottom_chunks[1]);
    }
}
