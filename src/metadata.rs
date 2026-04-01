use crate::{config::SUPPORTED_EXTENSIONS, song::Song};
use std::path::{Path, PathBuf};

use id3::TagLike;
use lofty::file::AudioFile;
use lofty::tag::TagType;

use lofty::{
    config::ParsingMode,
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey, Tag, items::Timestamp},
};
use walkdir::WalkDir;

#[derive(Debug)]
struct SongMetadata {
    title: String,
    artist: String,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    path: String,
    duration: u64,
    remix: Option<String>,
}

pub struct MetadataParser {}

impl MetadataParser {
    pub fn new() -> Self {
        Self {}
    }

    fn read_metadata(file_path: &Path) -> anyhow::Result<SongMetadata> {
        let mut song = SongMetadata {
            title: "".to_string(),
            artist: "".to_string(),
            album: None,
            year: None,
            genre: None,
            remix: None,
            duration: 0,
            path: file_path.to_string_lossy().to_string(),
        };

        let tagged_file = Probe::open(file_path)
            .expect("ERROR: Bad path provided!")
            .read()
            .expect("ERROR: Failed to read file!");

        let tag = match tagged_file.primary_tag() {
            Some(primary_tag) => primary_tag,
            None => {
                return Self::parse_filename_fallback(
                    file_path,
                    tagged_file.properties().duration().as_secs(),
                );
            }
        };

        song.album = tag.album().map(|s| s.into_owned());
        song.artist = tag.artist().map(|s| s.into_owned()).unwrap_or_default();
        song.title = tag.title().map(|s| s.into_owned()).unwrap_or_default();
        song.genre = tag.genre().map(|s| s.into_owned());
        song.remix = tag.get_string(ItemKey::Remixer).map(|s| s.to_string());
        song.year = Self::get_date(tag);
        song.duration = tagged_file.properties().duration().as_secs();

        if song.year.unwrap_or_default() == 0 && tag.tag_type() == TagType::Id3v2
            || tag.tag_type() == TagType::Id3v1
        {
            let tag = id3::Tag::read_from_path(file_path)?;
            song.year = tag.year();
        }

        // If we filled artist and song, we are done
        if song.title != "" && song.artist != "" {
            return Ok(song);
        }

        Self::parse_filename_fallback(file_path, song.duration)
    }

    fn get_date(tag: &Tag) -> Option<i32> {
        let t = tag
            .get_string(ItemKey::RecordingDate)
            .or_else(|| tag.get_string(ItemKey::Year))
            .or_else(|| tag.get_string(ItemKey::ReleaseDate))
            .or_else(|| tag.get_string(ItemKey::OriginalReleaseDate))
            .and_then(|d| {
                Timestamp::parse(&mut d.as_bytes(), ParsingMode::Relaxed)
                    .ok()
                    .flatten()
            })?
            .year;

        Some(t.into())
    }

    fn parse_filename_fallback(file_path: &Path, duration: u64) -> anyhow::Result<SongMetadata> {
        let mut song = SongMetadata {
            title: "".to_string(),
            artist: "".to_string(),
            album: None,
            year: None,
            genre: None,
            remix: None,
            duration: duration,
            path: file_path.to_string_lossy().to_string(),
        };

        //artist - song .extension
        if let Some(file_name) = file_path
            .with_extension("")
            .file_stem()
            .and_then(|s| s.to_str())
        {
            let parts: Vec<&str> = file_name.splitn(2, " - ").collect();
            if parts.len() == 2 {
                if song.artist == "" {
                    song.artist = parts[0].to_string();
                }
                song.title = parts[1].to_string();
            } else {
                song.title = file_name.to_string();
            }

            println!(
                "parsing fallback hit! parts<{:#?}> title:{:?} artist:{:?}",
                parts, song.title, song.artist
            );
        }

        Ok(song)
    }

    pub async fn parse_song_metadata(&self, music_folder_path: &str) -> Vec<Song> {
        let music_folder_abs: PathBuf =
            std::fs::canonicalize(music_folder_path).expect("Failed to canonicalize music folder");

        let mut songs: Vec<Song> = vec![];
        for entry in WalkDir::new(music_folder_abs)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext = ext.to_lowercase();
                if SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                    if let Some(metadata) = Self::read_metadata(path).ok() {
                        let m_title = metadata.title;
                        let m_artist = metadata.artist;
                        let m_album = metadata.album.unwrap_or_default();
                        let m_release_year = metadata.year.unwrap_or_default();
                        let m_remix = metadata.remix.unwrap_or_default();

                        let search_blob = [
                            m_title.as_str(),
                            m_artist.as_str(),
                            m_album.as_str(),
                            m_release_year.to_string().as_str(),
                            m_remix.as_str(),
                        ]
                        .into_iter()
                        .filter(|s| !s.is_empty() && *s != "0" && *s != "-")
                        .map(|s| s.trim().to_lowercase())
                        .collect::<Vec<_>>()
                        .join(" ");

                        songs.push(Song {
                            id: 0,
                            title: m_title,
                            artist: m_artist,
                            release_year: m_release_year,
                            album: m_album,
                            remix: m_remix,
                            search_blob: search_blob,
                            file_path: metadata.path,
                            duration: metadata.duration as i64,
                        });
                    }
                }
            }
        }
        songs
    }
}
