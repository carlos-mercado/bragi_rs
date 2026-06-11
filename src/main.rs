use music::*;
use std::time::{Duration, Instant};
use std::{io, thread};
use std::io::BufReader;
use std::fs::{File};
use std::path::Path;
//use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{ channel };
use std::sync::mpsc::{Sender, Receiver, RecvTimeoutError};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::{ Text },
    DefaultTerminal,
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{ Color, Style },
    widgets::{
        Block, 
        List, 
        ListState, 
        StatefulWidget, 
        Widget, 
        Paragraph,
        LineGauge,
    },
    symbols,
};

//use ratatui_image::{Image, picker::Picker, Resize};

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::new().run(terminal))
}

#[derive(PartialEq)]
enum VimMode { Search, Normal }

#[derive(PartialEq)]
enum PlaybackMode { 
    Paused, 
    Playing, 
    NotPlaying 
}

enum MusicStreamEvent {
    NewSongEvent(TrackDetails),
    PlaybackEvent(PlaybackMode) // Pause / Play
}

pub struct App {
    counter: u32,
    exit: bool,
    songs: Vec<TrackDetails>,
    selected: Option<TrackDetails>,
    play_start: Option<Instant>,
    elapsed_before_paused: Duration,
    sender: Sender<MusicStreamEvent>,
    receiver: Option<Receiver<MusicStreamEvent>>,
    mode: VimMode,
    playback_mode: Arc<Mutex<PlaybackMode>>,
    audio_handle: rodio::MixerDeviceSink,
    search_buff: String,
    unfiltered_songs: Vec<TrackDetails>,
    last_key: char,
    key_pressed_time: Instant,
}

impl App {
    pub fn new() -> App {
        let mut songs_vec : Vec<TrackDetails> = vec![];
        get_music_files(Path::new("/home/carlos/Music"), &mut songs_vec).unwrap();
        songs_vec.sort();
        let songs_vec_clone = songs_vec.clone();
        let audio_handle = rodio::DeviceSinkBuilder::open_default_sink()
            .expect("Could not find default audio stream");
        let (sender, receiver): (Sender<MusicStreamEvent>, Receiver<MusicStreamEvent>) = channel();
        let play_start = None;
        let elapsed_before_paused = Duration::from_secs(0);
        let playback_mode = Arc::new(Mutex::new(PlaybackMode::NotPlaying));


        let mut app = App {
            counter: 0,
            exit: false,
            songs: songs_vec,
            selected: None,
            play_start,
            elapsed_before_paused,
            sender,
            receiver: Some(receiver),
            mode: VimMode::Normal,
            playback_mode,
            audio_handle,
            search_buff: String::new(),
            unfiltered_songs: songs_vec_clone,
            last_key: ' ',
            key_pressed_time: std::time::Instant::now(),
        };

        app.playback();
        return app; 
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
        if event::poll(Duration::from_millis(500))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.mode == VimMode::Normal {
            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('j') => self.increment_counter(),
                KeyCode::Char('k') => self.decrement_counter(),
                KeyCode::Char('/') => self.mode = VimMode::Search,
                KeyCode::Char('g') => {
                    let timeout = Duration::from_millis(300);
                    if self.last_key == 'g' && self.key_pressed_time.elapsed() < timeout {
                        self.counter = 0;
                        self.last_key = ' ';
                    }
                    else {
                        self.last_key = 'g';
                        self.key_pressed_time = Instant::now();
                    }
                },
                KeyCode::Char('G') => {
                    self.counter = ( self.songs.len() - 1 ) as u32;
                }
                KeyCode::Char('p') => {
                    //pause play
                    let binding = Arc::clone(&self.playback_mode);
                    let mut state = binding.lock().unwrap();
                    match *state {
                        PlaybackMode::NotPlaying => { return; },
                        PlaybackMode::Playing => {
                            *state = PlaybackMode::Paused;
                            self.sender.send(MusicStreamEvent::PlaybackEvent( PlaybackMode::Paused ))
                                .expect("Could not send through channel");
                            self.elapsed_before_paused += self.play_start.unwrap().elapsed();
                        },
                        PlaybackMode::Paused => {  
                            *state = PlaybackMode::Playing;
                            self.sender.send(MusicStreamEvent::PlaybackEvent( PlaybackMode::Playing ))
                                .expect("Could not send through channel");

                            self.play_start = Some(Instant::now())
                        },
                    }
                },
                KeyCode::Esc => {
                    self.search_buff.clear();
                    self.mode = VimMode::Normal;
                    self.songs = self.unfiltered_songs.clone();
                },
                KeyCode::Enter => {
                    if self.songs.is_empty() { return; }
                    let binding = Arc::clone(&self.playback_mode);
                    let mut state = binding.lock().unwrap();
                    self.selected = Some(self.songs[self.counter as usize].clone());
                    let selected = self.selected.clone();
                    self.sender.send(MusicStreamEvent::NewSongEvent(selected.unwrap()))
                        .expect("Could not send through channel");
                    *state = PlaybackMode::Playing;
                    self.play_start = Some(Instant::now());
                },
                _ => {}
            }
        }
        else {
            match key_event.code {
                KeyCode::Char(c) => {
                    self.search_buff.push(c);
                    let result: Vec<TrackDetails> = self.unfiltered_songs
                        .iter()
                        .cloned()
                        .filter(|x| {
                            x.artist.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.album.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.title.to_lowercase().contains(&self.search_buff.to_lowercase())
                        })
                        .collect();

                    self.counter = 0;
                    self.songs = result;
                }
                KeyCode::Backspace => {
                    if self.search_buff.is_empty() { return; }
                    self.search_buff.pop();
                    let result: Vec<TrackDetails> = self.unfiltered_songs
                        .iter()
                        .cloned()
                        .filter(|x| {
                            x.artist.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.album.to_lowercase().contains(&self.search_buff.to_lowercase()) ||
                            x.title.to_lowercase().contains(&self.search_buff.to_lowercase())
                        })
                        .collect();
                    self.counter = 0;
                    self.songs = result;
                },
                KeyCode::Enter => {
                    self.search_buff.clear();
                    self.mode = VimMode::Normal;
                },
                KeyCode::Esc => {
                    self.counter = 0;
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

    fn get_time_elapsed(&self) -> Duration {
        self.elapsed_before_paused + self.play_start.unwrap_or(Instant::now()).elapsed()
    }

    fn playback(&mut self) {
        let Some(receiver) = self.receiver.take() else { return; };
        let mixer = self.audio_handle.mixer().clone();
        let binding = Arc::clone(&self.playback_mode);

        let _thread_handle = thread::spawn(move || {
            let mut current_track = match receiver.recv() {
                Ok(MusicStreamEvent::NewSongEvent(track_info)) => track_info,
                Ok(MusicStreamEvent::PlaybackEvent(_)) => return,
                Err(_) => return,
            };

            loop {
                let song_path = current_track.song_path.clone();
                let file = BufReader::new(File::open(song_path).unwrap());
                let mut song_time_remaining = Duration::from_secs(current_track.duration);
                let player = rodio::play(&mixer, file).unwrap();

                let mut is_paused = false;
                loop {
                    let now = std::time::Instant::now();

                    // if paused, wait indefinitely
                    // don't touch song_time_remaining

                    let event = if is_paused {
                        match receiver.recv() {
                            Ok(e) => Ok(e),
                            Err(_) => Err(RecvTimeoutError::Disconnected),
                        }
                    } else {
                        receiver.recv_timeout(song_time_remaining)
                    };

                    match event {
                        Ok(MusicStreamEvent::NewSongEvent(new_track_info)) => {
                            std::mem::drop(player);
                            current_track = new_track_info;
                            break;
                        },
                        Ok(MusicStreamEvent::PlaybackEvent(mode)) => {
                            match mode {
                                PlaybackMode::Paused => {
                                    player.pause();
                                    is_paused = true;
                                    song_time_remaining = song_time_remaining.saturating_sub(now.elapsed());
                                },
                                PlaybackMode::Playing => {
                                    player.play();
                                    is_paused = false;
                                },
                                _ => (),
                            }
                        },
                        Err(RecvTimeoutError::Timeout) => {
                            // fisnished the song naturally, 
                            // wait for a new one to start over again
                            let mut state = binding.lock().unwrap();
                            *state = PlaybackMode::NotPlaying;
                        },
                        Err(_) => {},
                    }
                }
            }
        });

    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::vertical([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ]);

        let [selection_area, lower_area] = chunks.areas(area);
        let lower_area_chunks = Layout::vertical([
            Constraint::Percentage(90),
            Constraint::Percentage(10),
        ]);

        let [info_area, progress_bar_area] = lower_area_chunks.areas(lower_area);


        let mut selection_state = ListState::default()
            .with_selected(Some(self.counter as usize));


        let music_preview = Block::bordered()
            .title_top("Now Playing");


        let binding = Arc::clone(&self.playback_mode);
        let state = binding.lock().unwrap();
        if let Some(selected_track) = &self.selected && ( *state == PlaybackMode::Playing || *state == PlaybackMode::Paused ) {
            Paragraph::new(Text::from(selected_track)).centered()
                .block(music_preview)
                .render(info_area, buf);
        }
        std::mem::drop(state);

        let selection_string = match self.mode {
            VimMode::Normal => String::from("Playlist"),
            VimMode::Search => format!("Searching: {}", self.search_buff),
        };

        let music_selection = List::new(self.songs.clone())
                .block(Block::bordered().title_top(selection_string))
                .style(ratatui::style::Style::default().fg(Color::White))
                .highlight_style(Style::new().italic())
                .highlight_symbol(">>");

        let binding = Arc::clone(&self.playback_mode);
        let playback_state = binding.lock().unwrap();
        if *playback_state == PlaybackMode::Playing || *playback_state == PlaybackMode::Paused {
            std::mem::drop(playback_state);
            let progress_bar = LineGauge::default()
                .filled_style(Style::new().white().on_black().bold())
                .label("")
                .filled_symbol(symbols::line::THICK_HORIZONTAL)
                .ratio(
                    self.get_time_elapsed().as_secs_f64() / self.selected.clone().unwrap().duration as f64
                );
            progress_bar.render(progress_bar_area, buf);
        }


        StatefulWidget::render(
            music_selection,
            selection_area,
            buf,
            &mut selection_state,
        );
    }
}
