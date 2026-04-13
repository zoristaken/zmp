use crate::search_blob::build_search_blob;
use crate::song::Song;
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use id3::TagLike;
use lofty::file::AudioFile;
use lofty::tag::TagType;

use lofty::{
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use walkdir::WalkDir;

#[derive(Debug)]
struct SongMetadata {
    title: String,
    artist: String,
    album: Option<String>,
    year: Option<i32>,
    duration: u64,
    remix: Option<String>,
    ext: String,
}

struct FallbackMetadata {
    title: String,
    artist: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscoveredMusicFile {
    pub path: PathBuf,
    pub file_path: String,
    pub file_size: i64,
    pub file_modified_millis: i64,
}

#[derive(Default)]
pub struct MetadataParser {}

impl MetadataParser {
    pub fn new() -> Self {
        Self {}
    }

    fn should_fetch_metadata_year_id3(&self, year: i32, tag_type: TagType) -> bool {
        year == 0 && (tag_type == TagType::Id3v2 || tag_type == TagType::Id3v1)
    }

    fn read_metadata(&self, file_path: &Path) -> anyhow::Result<SongMetadata> {
        let mut song = SongMetadata {
            title: String::new(),
            artist: String::new(),
            album: None,
            year: None,
            remix: None,
            duration: 0,
            ext: file_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_lowercase(),
        };

        let tagged_file = Probe::open(file_path)?.read()?;

        let tag = match tagged_file.primary_tag() {
            Some(primary_tag) => primary_tag,
            None => {
                let fallback_metadata = self.parse_filename_fallback(file_path)?;
                song.artist = fallback_metadata.artist;
                song.title = fallback_metadata.title;
                song.duration = tagged_file.properties().duration().as_secs();
                return Ok(song);
            }
        };

        song.album = tag.album().map(|s| s.into_owned());
        song.artist = tag.artist().map(|s| s.into_owned()).unwrap_or_default();
        song.title = tag.title().map(|s| s.into_owned()).unwrap_or_default();
        song.remix = tag.get_string(ItemKey::Remixer).map(|s| s.to_string());
        song.year = Some(tag.date().unwrap_or_default().year.into());
        song.duration = tagged_file.properties().duration().as_secs();

        if self.should_fetch_metadata_year_id3(song.year.unwrap_or_default(), tag.tag_type()) {
            let tag = id3::Tag::read_from_path(file_path)?;
            song.year = tag.year();
        }

        if song.title.is_empty() || song.artist.is_empty() {
            let fallback_metadata = self.parse_filename_fallback(file_path)?;
            if song.title.is_empty() {
                song.title = fallback_metadata.title;
            }

            if song.artist.is_empty() {
                song.artist = fallback_metadata.artist;
            }
        }
        Ok(song)
    }

    fn parse_filename_fallback(&self, file_path: &Path) -> anyhow::Result<FallbackMetadata> {
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

    fn build_discovered_music_file(&self, file_path: &Path) -> anyhow::Result<DiscoveredMusicFile> {
        let metadata = std::fs::metadata(file_path)?;
        let file_size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
        let file_modified_millis = metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_millis()).ok())
            .unwrap_or(0);

        Ok(DiscoveredMusicFile {
            path: file_path.to_path_buf(),
            file_path: file_path.to_string_lossy().to_string(),
            file_size,
            file_modified_millis,
        })
    }

    pub fn discover_music_files(
        &self,
        music_folder_path: &Path,
    ) -> anyhow::Result<Vec<DiscoveredMusicFile>> {
        let music_folder_abs: PathBuf = std::fs::canonicalize(music_folder_path)?;
        let mut files = Vec::new();

        for entry in WalkDir::new(music_folder_abs)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            match self.build_discovered_music_file(entry.path()) {
                Ok(file) => files.push(file),
                Err(_) => continue,
            }
        }

        files.sort();
        Ok(files)
    }

    pub fn parse_discovered_music_file(
        &self,
        music_file: &DiscoveredMusicFile,
    ) -> anyhow::Result<Song> {
        let metadata = self.read_metadata(&music_file.path)?;
        let title = metadata.title;
        let artist = metadata.artist;
        let album = metadata.album.unwrap_or_default();
        let release_year = metadata.year.unwrap_or_default();
        let remix = metadata.remix.unwrap_or_default();

        let search_blob =
            build_search_blob([&title, &artist, &album, &release_year.to_string(), &remix]);

        Ok(Song {
            id: 0,
            title,
            artist,
            release_year,
            album,
            remix,
            search_blob,
            file_path: music_file.file_path.clone(),
            duration: metadata.duration as i64,
            extension: metadata.ext,
            file_size: music_file.file_size,
            file_modified_millis: music_file.file_modified_millis,
        })
    }

    pub fn parse_song_metadata(&self, music_folder_path: &Path) -> anyhow::Result<Vec<Song>> {
        let mut songs: Vec<Song> = vec![];
        for music_file in self.discover_music_files(music_folder_path)? {
            match self.parse_discovered_music_file(&music_file) {
                Ok(song) => songs.push(song),
                Err(_) => {
                    continue;
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

        let result = MetadataParser::new().parse_filename_fallback(path).unwrap();

        assert_eq!(result.artist, "Massive Attack");
        assert_eq!(result.title, "Teardrop");
    }

    #[test]
    fn parse_filename_fallback_trims_whitespace() {
        let path = Path::new("  Massive Attack   -   Teardrop  .mp3");

        let result = MetadataParser::new().parse_filename_fallback(path).unwrap();

        assert_eq!(result.artist, "Massive Attack");
        assert_eq!(result.title, "Teardrop");
    }

    #[test]
    fn parse_filename_fallback_errors_when_separator_missing() {
        let path = Path::new("Teardrop.mp3");

        let result = MetadataParser::new().parse_filename_fallback(path);

        assert!(result.is_err());
    }
}
