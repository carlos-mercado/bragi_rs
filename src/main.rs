use music::*;
//use rodio::MixerDeviceSink;
//use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::{io, thread};
use std::io::BufReader;
use std::fs::{File};
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver, RecvTimeoutError};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::{ Text },
    DefaultTerminal, 
    Frame, 
    buffer::Buffer, 
    layout::{Constraint, Layout, Rect}, 
    style::{ Color, Style }, 
    widgets::{Block, List, ListState, StatefulWidget, Widget, Paragraph}
};

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

#[derive(PartialEq)]
enum VimMode { Search, Normal }
// enum PlaybackMode { Pause, Play }

pub struct App {
    counter: u32,
    exit: bool,
    songs: Vec<TrackDetails>,
    selected: Option<TrackDetails>,
    sender: Sender<TrackDetails>,
    mode: VimMode,
    search_buff: String,
    unfiltered_songs: Vec<TrackDetails>,
}

impl App {
    pub fn new() -> App {
        let mut songs_vec : Vec<TrackDetails> = vec![];
        get_music_files(Path::new("/home/carlos/Music"), &mut songs_vec).unwrap();
        songs_vec.sort();
        let songs_vec_clone = songs_vec.clone();
        let handle = rodio::DeviceSinkBuilder::open_default_sink()
            .expect("Could not find default audio stream");

        let (sender, receiver): (Sender<TrackDetails>, Receiver<TrackDetails>) = channel();

        let _thread_handle = thread::spawn(move || {
            let mut current_track = match receiver.recv() {
                Ok(track_info) => track_info,
                Err(_) => return,
            };

            loop {

                let song_path = current_track.song_path.clone();
                let duration = Duration::from_secs(current_track.duration);
                let file = BufReader::new(File::open(song_path).unwrap());
                let player = rodio::play(handle.mixer(), file).unwrap();

                match receiver.recv_timeout(duration) {
                    Ok(new_track_info) => { 
                        // got a new track, 
                        // get rid of the current player
                        // and start up a new one next iteration
                        std::mem::drop(player);
                        current_track = new_track_info;
                    },
                    Err(RecvTimeoutError::Timeout) => {
                        // fisnished the song naturally, 
                        // wait for a new one to start over again
                        std::mem::drop(player);
                        current_track = match receiver.recv() {
                            Ok(track_info) => track_info,
                            Err(_) => return,
                        }
                    },
                    Err(_) => break,
                }

            }
        });

        App {
            counter: 0,
            exit: false,
            songs: songs_vec,
            selected: None,
            sender,
            mode: VimMode::Normal,
            search_buff: String::new(),
            unfiltered_songs: songs_vec_clone,
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
        if self.mode == VimMode::Normal {
            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('j') => self.increment_counter(),
                KeyCode::Char('k') => self.decrement_counter(),
                KeyCode::Char('/') => self.mode = VimMode::Search,
                KeyCode::Esc => {
                    self.search_buff.clear();
                    self.mode = VimMode::Normal;
                    self.songs = self.unfiltered_songs.clone();
                },
                KeyCode::Enter => {
                    self.selected = Some(self.songs[self.counter as usize].clone());
                    let selected = self.selected.clone();
                    self.sender.send(selected.unwrap())
                        .expect("Could not send through channel");
                },
                _ => {}
            }
        }
        else {

            match key_event.code {
                KeyCode::Char(c) => {
                    self.search_buff.push(c);
                    let result: Vec<TrackDetails> = self.songs
                        .iter()
                        .cloned()
                        .filter(|x| {
                            x.artist.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.album.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.title.to_lowercase().contains(&self.search_buff.to_lowercase())
                        })
                        .collect();
                    self.songs = result;
                }
                KeyCode::Backspace => {
                    self.search_buff.pop();
                    self.songs = self.unfiltered_songs.clone();
                    let result: Vec<TrackDetails> = self.songs
                        .iter()
                        .cloned()
                        .filter(|x| x.artist.contains(&self.search_buff))
                        .collect();
                    self.songs = result;
                },
                KeyCode::Enter => {
                    self.search_buff.clear();
                    self.mode = VimMode::Normal;
                },
                KeyCode::Esc => {
                    self.search_buff.clear();
                    self.mode = VimMode::Normal;
                    self.songs = self.unfiltered_songs.clone();
                },
                _ => {}
            }
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
            Paragraph::new(Text::from(selected_track)).centered()
                .block(music_preview)
                .render(preview_area, buf);

        }

        let selection_string = match self.mode {
            VimMode::Normal => String::from("Playlist"),
            VimMode::Search => format!("Searching: {}", self.search_buff),
        };

        let music_selection = List::new(self.songs.clone())
                .block(Block::bordered().title_top(selection_string))
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
