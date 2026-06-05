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
}

impl From<TrackDetails> for Text<'static> {
    fn from(track: TrackDetails) -> Self {
        Text::from(format!(
            "{} - {} ({}) [Track {}]",
            track.artist,
            track.title,
            track.date,
            track.track_no
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
                songs.push(get_audio_metadata(&path).unwrap());
            }

        }
    }

    Ok(())
}

fn get_audio_metadata(path: &Path) -> Result<TrackDetails, Box<dyn std::error::Error>> {
    let tagged_file : TaggedFile = read_from_path(path)?;
    let tag = tagged_file.primary_tag().unwrap();
    let title = tag.title().unwrap().to_string();
    let artist = tag.artist().unwrap().to_string();
    let date = tag.date().unwrap().to_string();
    let track_no = tag.track().unwrap();

    Ok(TrackDetails {artist, title, date, track_no})
}
