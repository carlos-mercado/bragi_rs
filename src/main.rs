use std::io;
use std::path::Path;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, 
    Frame, 
    buffer::Buffer, 
    layout::{Constraint, Layout, Rect}, 
    style::{ Color, Style }, 
    widgets::{Block, List, ListState, StatefulWidget, Widget, Paragraph}
};

use ratatui::prelude::Line;

use music::*;

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

pub struct App {
    counter: u32,
    exit: bool,
    songs: Vec<TrackDetails>,
    selected: Option<TrackDetails>,
}

impl App {
    pub fn new() -> App {
        let mut songs_vec : Vec<TrackDetails> = vec![];
        get_music_files(Path::new("/home/carlos/Music"), &mut songs_vec).unwrap();

        App {
            counter: 0,
            exit: false,
            songs: songs_vec,
            selected: None,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('j') => self.increment_counter(),
            KeyCode::Char('k') => self.decrement_counter(),
            KeyCode::Enter => self.selected = Some(self.songs[self.counter as usize].clone()),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
        self.counter = std::cmp::min(( self.songs.len() - 1 ) as u32, self.counter + 1);
    }

    fn decrement_counter(&mut self) {
        self.counter = std::cmp::max(0 as i32, ( self.counter as i32 ) - 1) as u32;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ]);

        let [selection_area, preview_area] = chunks.areas(area);


        let mut selection_state = ListState::default()
            .with_selected(Some(self.counter as usize));


        let music_preview = Block::bordered()
            .title_top("Now Playing");


        if let Some(selected_track) = &self.selected {
            Paragraph::new(Line::from(selected_track)).centered()
                .block(music_preview)
                .render(preview_area, buf);

        }

        let music_selection = List::new(self.songs.clone())
                .block(Block::bordered().title_top("Playlist"))
                .style(ratatui::style::Style::default().fg(Color::White))
                .highlight_style(Style::new().italic())
                .highlight_symbol(">>");

        StatefulWidget::render(
            music_selection,
            selection_area,
            buf,
            &mut selection_state,
        );
    }
}
