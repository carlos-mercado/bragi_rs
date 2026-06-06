use std::io;
use std::fs::{self, DirEntry };
use std::path::{ Path, PathBuf };
use lofty::file::TaggedFile;
use lofty::{prelude::*, read_from_path};
use ratatui::prelude::{ Text, Line };

#[derive(Clone)]
pub struct TrackDetails {
    pub artist: String,
    pub title: String,
    pub date: String,
    pub track_no: u32,
    pub song_path: String,
    pub duration: u64,
}

impl From<TrackDetails> for Text<'static> {
    fn from(track: TrackDetails) -> Self {
        Text::from(format!(
            "{} - {} ({}) [Track {}]",
            track.artist,
            track.title,
            track.date,
            track.track_no,
        ))
    }
}

impl From<&TrackDetails> for Line<'static> {
    fn from(track: &TrackDetails) -> Self {
        Line::from(format!(
            "{} - {} ({}) [Track {}]",
            track.artist,
            track.title,
            track.date,
            track.track_no
        ))
    }
}

// this is recusive dfs with no cycle checks
// idk if cylces are possible to create in file systems 
// maybe with symlinks, but i don't wan't to deal 
// with that right now
pub fn get_music_files(path: &Path, songs: &mut Vec<TrackDetails>) -> io::Result<()> {
    let mut it : fs::ReadDir = fs::read_dir(path)?;

    while let Some(entry) = it.next() {
        let entry : DirEntry = entry?;

        if entry.metadata()?.is_dir() {
            get_music_files(&entry.path(), songs)?;
        }
        else {
            let path : PathBuf = entry.path();
            let file_type = path.extension()
                .and_then(|e| e.to_str())
                .unwrap();

            let is_audio = matches!(
                file_type.to_lowercase().as_str(),
                "mp3" | "flac" | "alac" | "wav" | "aac" | "ogg" | "m4a" | "aiff"
            );

            if is_audio {
                match get_audio_metadata(&path) {
                    Ok(ans) => songs.push(ans),
                    _ => {}
                }
            }

        }
    }

    Ok(())
}

fn get_audio_metadata(path: &Path) -> Result<TrackDetails, Box<dyn std::error::Error>> {
    let tagged_file : TaggedFile = read_from_path(path)?;
    let tag = tagged_file.primary_tag().unwrap();
    let title = tag.title().unwrap_or("Unknown Title".into()).to_string();
    let artist = tag.artist().unwrap_or("Unknown Artist".into()).to_string();
    let date = tag.date().unwrap_or(lofty::tag::items::Timestamp { year: (1900), month: (Some(1)), day: (Some(1)), hour: (Some(0)), minute: (Some(0)), second: (Some(0)) }).to_string();
    let track_no = tag.track().unwrap_or(0);
    let duration = tagged_file.properties().duration().as_secs();
    Ok(TrackDetails {artist, title, date, track_no, song_path: path.to_string_lossy().to_string(), duration})
}
