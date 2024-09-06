use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::env;
use std::{
    error::Error,
    fs::File,
    io::{self, Write},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use subprocess::{Popen, PopenConfig, Redirection};

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        panic!("Provide a command to 'watch' for");
    }

    let mut watcher = Watcher::new(&args[1]);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let result = watcher.run_app(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("{:?}", err)
    }

    Ok(())
}

struct Watcher {
    command: Vec<String>,
    content: String,
    last_refresh_timestamp: u128,
    file_logger: File,
}

impl Watcher {
    fn new(command: &str) -> Self {
        Self {
            command: command.split(' ').map(String::from).collect(),
            content: String::new(),
            last_refresh_timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            file_logger: File::create("./watcher.log").unwrap(),
        }
    }

    fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw_ui(f))?;
            self.run_command()?;

            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                if let Event::Key(key) = ev {
                    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
                        return Ok(());
                    }
                } else if let Event::Mouse(mouse) = ev {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => {}
                        MouseEventKind::ScrollUp => {}
                        _ => {}
                    }
                }
            }
        }
    }

    fn run_command(&mut self) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        if (timestamp - self.last_refresh_timestamp) <= 1000 {
            return Ok(());
        }

        self.last_refresh_timestamp = timestamp;

        let mut p = Popen::create(
            &self.command,
            PopenConfig {
                stdout: Redirection::Pipe,
                ..Default::default()
            },
        )
        .unwrap();

        // Obtain the output from the standard streams.
        let (out, _) = p.communicate(None)?;

        if let Ok(exit_status) = p.wait() {
            self.content = out.unwrap();
            writeln!(
                &mut self.file_logger,
                "Got {} stdout lines [ status: {} ] at timestamp: {}",
                self.content.len(),
                exit_status.success(),
                timestamp
            )?;
        }

        Ok(())
    }

    fn draw_ui(&mut self, f: &mut Frame) {
        // Wrapping block for a group
        // Just draw the block and the group on the same area and build the group
        // with at least a margin of 1
        let area = f.area();

        // Surrounding block
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Main block with round corners")
            .title_alignment(Alignment::Center)
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
                "Styled title",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Right);

        let para = Paragraph::new(Text::raw(&self.content))
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(para, chunks[0]);

        // Bottom two inner blocks
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
            .split(chunks[1]);

        // Bottom left block with all default borders
        let block = Block::default().title("With borders").borders(Borders::ALL);
        f.render_widget(block, bottom_chunks[0]);

        // Bottom right block with styled left and right border
        let block = Block::default()
            .title("TIME")
            .border_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Thick);
        f.render_widget(block, bottom_chunks[1]);
    }
}
