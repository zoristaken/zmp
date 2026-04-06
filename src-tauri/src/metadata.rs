use crate::song::Song;
use std::path::{Path, PathBuf};

use id3::TagLike;
use lofty::file::AudioFile;
use lofty::tag::TagType;

use lofty::{
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

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

struct FallbackMetadata {
    title: String,
    artist: String,
}

#[derive(Default)]
pub struct MetadataParser {}

impl MetadataParser {
    pub fn new() -> Self {
        Self {}
    }

    fn read_metadata(file_path: &Path) -> anyhow::Result<SongMetadata> {
        let mut song = SongMetadata {
            title: String::new(),
            artist: String::new(),
            album: None,
            year: None,
            genre: None,
            remix: None,
            duration: 0,
            path: file_path.to_string_lossy().to_string(),
        };

        let tagged_file = Probe::open(file_path)?.read()?;

        let tag = match tagged_file.primary_tag() {
            Some(primary_tag) => primary_tag,
            None => {
                let fallback_metadata = Self::parse_filename_fallback(file_path)?;
                song.artist = fallback_metadata.artist;
                song.title = fallback_metadata.title;
                song.duration = tagged_file.properties().duration().as_secs();
                return Ok(song);
            }
        };

        song.album = tag.album().map(|s| s.into_owned());
        song.artist = tag.artist().map(|s| s.into_owned()).unwrap_or_default();
        song.title = tag.title().map(|s| s.into_owned()).unwrap_or_default();
        song.genre = tag.genre().map(|s| s.into_owned());
        song.remix = tag.get_string(ItemKey::Remixer).map(|s| s.to_string());
        song.year = Some(tag.date().unwrap_or_default().year.into());
        song.duration = tagged_file.properties().duration().as_secs();

        if song.year.unwrap_or_default() == 0
            && (tag.tag_type() == TagType::Id3v2 || tag.tag_type() == TagType::Id3v1)
        {
            let tag = id3::Tag::read_from_path(file_path)?;
            song.year = tag.year();
        }

        if song.title.is_empty() || song.artist.is_empty() {
            let fallback_metadata = Self::parse_filename_fallback(file_path)?;
            if song.title.is_empty() {
                song.title = fallback_metadata.title;
            }

            if song.artist.is_empty() {
                song.artist = fallback_metadata.artist;
            }
        }
        Ok(song)
    }

    fn parse_filename_fallback(file_path: &Path) -> anyhow::Result<FallbackMetadata> {
        let no_ext = file_path.with_extension("");

        let file_name = no_ext
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to get file name"))?;

        let (artist, title) = file_name
            .split_once(" - ")
            .ok_or_else(|| anyhow::anyhow!("Failed to parse artist and title from file name"))?;

        Ok(FallbackMetadata {
            title: title.trim().to_string(),
            artist: artist.trim().to_string(),
        })
    }

    pub fn parse_song_metadata(&self, music_folder_path: &Path) -> anyhow::Result<Vec<Song>> {
        let music_folder_abs: PathBuf = std::fs::canonicalize(music_folder_path)?;

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
                    match Self::read_metadata(path) {
                        Ok(metadata) => {
                            let m_title = metadata.title;
                            let m_artist = metadata.artist;
                            let m_album = metadata.album.unwrap_or_default();
                            let m_release_year = metadata.year.unwrap_or_default();
                            let m_remix = metadata.remix.unwrap_or_default();

                            let search_blob = [
                                m_title.as_str(),
                                m_artist.as_str(),
                                &m_album,
                                &m_release_year.to_string(),
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
                                search_blob,
                                file_path: metadata.path,
                                duration: metadata.duration as i64,
                            });
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
            }
        }
        Ok(songs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_filename_fallback_parses_artist_and_title() {
        let path = Path::new("Massive Attack - Teardrop.mp3");

        let result = MetadataParser::parse_filename_fallback(path).unwrap();

        assert_eq!(result.artist, "Massive Attack");
        assert_eq!(result.title, "Teardrop");
    }

    #[test]
    fn parse_filename_fallback_trims_whitespace() {
        let path = Path::new("  Massive Attack   -   Teardrop  .mp3");

        let result = MetadataParser::parse_filename_fallback(path).unwrap();

        assert_eq!(result.artist, "Massive Attack");
        assert_eq!(result.title, "Teardrop");
    }

    #[test]
    fn parse_filename_fallback_errors_when_separator_missing() {
        let path = Path::new("Teardrop.mp3");

        let result = MetadataParser::parse_filename_fallback(path);

        assert!(result.is_err());
    }
}
