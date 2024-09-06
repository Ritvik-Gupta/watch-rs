use crossterm::event::{self as term_event, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{palette::tailwind, Color, Stylize},
    symbols,
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
    Frame, Terminal,
};
use std::time::Instant;
use std::{io, time::Duration};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tui_textarea::TextArea;

use super::TICK_RATE;

#[derive(Default, Clone, Copy, EnumIter, Display, FromRepr)]
enum QueryEditTab {
    #[strum(to_string = "Setup Tab")]
    SETUP,

    #[default]
    #[strum(to_string = "Main Tab")]
    MAIN,
}

impl QueryEditTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    /// A block surrounding the tab's content
    fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::ROUNDED)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    const fn palette(self) -> tailwind::Palette {
        match self {
            Self::SETUP => tailwind::BLUE,
            Self::MAIN => tailwind::EMERALD,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum QueryMode {
    NORMAL,
    EDITOR,
    SUBMIT,
}

pub struct QueryState {
    pub setup_commands: String,
    pub main_commands: String,
}

pub struct QueryTui {
    state: QueryState,
    editing_tab: QueryEditTab,
    running_mode: QueryMode,
}

impl QueryTui {
    pub fn new(commands: Option<String>) -> Self {
        Self {
            state: QueryState {
                main_commands: commands.unwrap_or_else(|| String::new()),
                setup_commands: String::new(),
            },
            editing_tab: QueryEditTab::default(),
            running_mode: QueryMode::NORMAL,
        }
    }

    pub fn run_app(
        mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<Option<QueryState>, std::io::Error> {
        let mut last_tick = Instant::now();
        let mut setup_textarea = TextArea::from(self.state.setup_commands.lines());
        let mut main_textarea = TextArea::from(self.state.main_commands.lines());

        loop {
            terminal.draw(|f| {
                self.draw_ui(
                    f,
                    match self.editing_tab {
                        QueryEditTab::MAIN => &mut main_textarea,
                        QueryEditTab::SETUP => &mut setup_textarea,
                    },
                )
            })?;

            let timeout = TICK_RATE
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if term_event::poll(timeout)? {
                match term_event::read()? {
                    Event::Key(key) => match key {
                        KeyEvent {
                            modifiers: KeyModifiers::CONTROL,
                            code: KeyCode::Char('c'),
                            ..
                        } => return Ok(None),
                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Enter,
                            ..
                        } if self.running_mode == QueryMode::SUBMIT => {
                            self.state.main_commands = main_textarea.lines().join("\n");
                            self.state.setup_commands = setup_textarea.lines().join("\n");

                            return Ok(Some(self.state));
                        }

                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Enter,
                            ..
                        } if self.running_mode == QueryMode::NORMAL => {
                            self.running_mode = QueryMode::SUBMIT;
                        }
                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Char('i'),
                            ..
                        } if self.running_mode != QueryMode::EDITOR => {
                            self.running_mode = QueryMode::EDITOR;
                        }
                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Esc,
                            ..
                        } if self.running_mode == QueryMode::EDITOR => {
                            self.running_mode = QueryMode::NORMAL;
                        }

                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Left,
                            ..
                        } if self.running_mode == QueryMode::NORMAL => {
                            self.editing_tab = self.editing_tab.previous();
                        }
                        KeyEvent {
                            modifiers: KeyModifiers::NONE,
                            code: KeyCode::Right,
                            ..
                        } if self.running_mode == QueryMode::NORMAL => {
                            self.editing_tab = self.editing_tab.next();
                        }

                        _ if self.running_mode == QueryMode::EDITOR => {
                            match self.editing_tab {
                                QueryEditTab::MAIN => main_textarea.input(key),
                                QueryEditTab::SETUP => setup_textarea.input(key),
                            };
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if last_tick.elapsed() >= TICK_RATE {
                last_tick = Instant::now();
            }
        }
    }

    fn title_widget() -> impl Widget {
        "Watch Query".bold()
    }

    fn footer_widget(&self) -> impl Widget {
        let mut components = Vec::new();
        match self.running_mode {
            QueryMode::NORMAL => {
                components.push("◄ ► to change tab");
                components.push("(I) to enter insert mode");
            }
            QueryMode::EDITOR => {
                components.push("↲ Esc to pause editor");
            }
            QueryMode::SUBMIT => {}
        }
        components.push("Press Ctrl+C to quit");

        Line::raw(components.join(" │ ")).centered()
    }

    fn tabs_widget(&self) -> impl Widget {
        let titles = QueryEditTab::iter().map(QueryEditTab::title);
        let highlight_style = (Color::default(), self.editing_tab.palette().c700);
        let selected_tab_index = self.editing_tab as usize;

        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
    }

    fn draw_ui(&mut self, f: &mut Frame, editing_textarea: &mut TextArea) {
        use Constraint::{Fill, Length, Min, Percentage};

        let area = f.area();

        let vertical = Layout::vertical([Fill(1), Percentage(90), Fill(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        f.render_widget(QueryTui::title_widget(), title_area);
        f.render_widget(self.tabs_widget(), tabs_area);

        let block = self.editing_tab.block();

        if self.running_mode == QueryMode::EDITOR {
            editing_textarea.set_block(block);
            f.render_widget(&*editing_textarea, inner_area);
        } else {
            f.render_widget(
                Paragraph::new(editing_textarea.lines().join("\n")).block(block),
                inner_area,
            );
        }

        f.render_widget(self.footer_widget(), footer_area);
    }
}
