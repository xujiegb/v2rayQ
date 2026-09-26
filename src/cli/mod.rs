use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Nodes,
    Subscriptions,
    Profiles,
    Runtime,
    Status,
}

impl Page {
    const ALL: [Self; 5] = [
        Self::Nodes,
        Self::Subscriptions,
        Self::Profiles,
        Self::Runtime,
        Self::Status,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Nodes => "Nodes",
            Self::Subscriptions => "Subscriptions",
            Self::Profiles => "Profiles",
            Self::Runtime => "Runtime",
            Self::Status => "Status",
        }
    }
}

#[derive(Debug)]
pub struct App {
    page: Page,
    running: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            page: Page::Nodes,
            running: true,
        }
    }
}

pub fn run() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}

impl App {
    pub fn run(mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_event(event::read()?);
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        let Event::Key(key) = event else {
            return;
        };

        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Up | KeyCode::Char('k') => self.previous_page(),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.next_page(),
            KeyCode::BackTab => self.previous_page(),
            KeyCode::Char('1') => self.page = Page::Nodes,
            KeyCode::Char('2') => self.page = Page::Subscriptions,
            KeyCode::Char('3') => self.page = Page::Profiles,
            KeyCode::Char('4') => self.page = Page::Runtime,
            KeyCode::Char('5') => self.page = Page::Status,
            _ => {}
        }
    }

    fn next_page(&mut self) {
        let index = Page::ALL
            .iter()
            .position(|page| *page == self.page)
            .unwrap_or(0);

        self.page = Page::ALL[(index + 1) % Page::ALL.len()];
    }

    fn previous_page(&mut self) {
        let index = Page::ALL
            .iter()
            .position(|page| *page == self.page)
            .unwrap_or(0);

        self.page = Page::ALL[
            (index + Page::ALL.len() - 1) % Page::ALL.len()
        ];
    }

    fn render(&self, frame: &mut Frame) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .areas(frame.area());

        let [navigation, content] = Layout::horizontal([
            Constraint::Length(24),
            Constraint::Min(0),
        ])
        .areas(body);

        self.render_header(frame, header);
        self.render_navigation(frame, navigation);
        self.render_content(frame, content);
        self.render_footer(frame, footer);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new(Line::from(vec![
            Span::styled("v2rayQ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::raw(self.page.title()),
        ]))
        .block(Block::bordered());

        frame.render_widget(title, area);
    }

    fn render_navigation(&self, frame: &mut Frame, area: Rect) {
        let lines = Page::ALL
            .iter()
            .enumerate()
            .map(|(index, page)| {
                let label = format!("{}  {}", index + 1, page.title());

                if *page == self.page {
                    Line::from(Span::styled(
                        label,
                        Style::default()
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    ))
                } else {
                    Line::from(label)
                }
            })
            .collect::<Vec<_>>();

        let navigation = Paragraph::new(lines)
            .block(Block::bordered().title("Navigation"));

        frame.render_widget(navigation, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let lines = match self.page {
            Page::Nodes => vec![
                Line::from("No nodes loaded."),
                Line::from(""),
                Line::from("Node management will be shown here."),
            ],
            Page::Subscriptions => vec![
                Line::from("No subscriptions loaded."),
                Line::from(""),
                Line::from("Subscription management will be shown here."),
            ],
            Page::Profiles => vec![
                Line::from("No profiles loaded."),
                Line::from(""),
                Line::from("Profile management will be shown here."),
            ],
            Page::Runtime => vec![
                Line::from("System proxy: disabled"),
                Line::from("TUN: disabled"),
            ],
            Page::Status => vec![
                Line::from("sing-box: unknown"),
                Line::from("v2rayQ database: not connected"),
            ],
        };

        let content = Paragraph::new(lines)
            .block(Block::bordered().title(self.page.title()));

        frame.render_widget(content, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(
            "↑/↓ j/k navigate   Tab next   1-5 select   q/Esc quit",
        ))
        .block(Block::bordered());

        frame.render_widget(footer, area);
    }
}
