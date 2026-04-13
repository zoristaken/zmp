use crate::filter::Filter;
use crate::search_blob::build_search_blob;
use crate::song::Song;
use std::borrow::Cow;
use std::fs::File;
use std::path::{Path, PathBuf};

use id3::TagLike;
use lofty::aac::AacFile;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::id3::v2::Id3v2Tag;
use lofty::iff::wav::WavFile;
use lofty::mp4::{Atom, AtomData, AtomIdent, Ilst, Mp4File};
use lofty::mpeg::MpegFile;
use lofty::ogg::{OpusFile, SpeexFile, VorbisFile};
use lofty::tag::{ItemValue, TagType};
use lofty::{ape::ApeFile, flac::FlacFile};

use lofty::{
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use walkdir::WalkDir;

const FILTER_TAG_KEY: &str = "ZMP_FILTERS";

#[derive(Debug)]
struct SongMetadata {
    title: String,
    artist: String,
    album: Option<String>,
    year: Option<i32>,
    path: String,
    duration: u64,
    remix: Option<String>,
    ext: String,
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

    fn should_fetch_metadata_year_id3(&self, year: i32, tag_type: TagType) -> bool {
        return year == 0 && (tag_type == TagType::Id3v2 || tag_type == TagType::Id3v1);
    }

    fn read_metadata(&self, file_path: &Path) -> anyhow::Result<SongMetadata> {
        let mut song = SongMetadata {
            title: String::new(),
            artist: String::new(),
            album: None,
            year: None,
            remix: None,
            duration: 0,
            path: file_path.to_string_lossy().to_string(),
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

    pub fn parse_song_metadata(&self, music_folder_path: &Path) -> anyhow::Result<Vec<Song>> {
        let music_folder_abs: PathBuf = std::fs::canonicalize(music_folder_path)?;

        let mut songs: Vec<Song> = vec![];
        for entry in WalkDir::new(music_folder_abs)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            match self.read_metadata(entry.path()) {
                Ok(metadata) => {
                    let m_title = metadata.title;
                    let m_artist = metadata.artist;
                    let m_album = metadata.album.unwrap_or_default();
                    let m_release_year = metadata.year.unwrap_or_default();
                    let m_remix = metadata.remix.unwrap_or_default();

                    let search_blob = build_search_blob([
                        &m_title,
                        &m_artist,
                        &m_album,
                        &m_release_year.to_string(),
                        &m_remix,
                    ]);

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
                        extension: metadata.ext,
                    });
                }
                Err(_) => {
                    continue;
                }
            }
        }
        Ok(songs)
    }

    pub fn add_song_filters_metadata(
        &self,
        file_path: &Path,
        filters: Vec<Filter>,
    ) -> anyhow::Result<()> {
        let filters_string = filters
            .iter()
            .map(|f| f.name.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let file_type = self.detect_file_type(file_path)?;

        match file_type {
            FileType::Aac | FileType::Mpeg | FileType::Wav => {
                self.write_id3_filters(file_path, FILTER_TAG_KEY, &filters_string)?
            }
            FileType::Flac => {
                self.write_flac_filters(file_path, FILTER_TAG_KEY, &filters_string)?
            }
            FileType::Vorbis => {
                self.write_vorbis_filters(file_path, FILTER_TAG_KEY, &filters_string)?
            }
            FileType::Opus => {
                self.write_opus_filters(file_path, FILTER_TAG_KEY, &filters_string)?
            }
            FileType::Speex => {
                self.write_speex_filters(file_path, FILTER_TAG_KEY, &filters_string)?
            }
            FileType::Mp4 => self.write_mp4_filters(file_path, FILTER_TAG_KEY, &filters_string)?,
            FileType::Ape => self.write_ape_filters(file_path, FILTER_TAG_KEY, &filters_string)?,
            _ => {
                return Err(anyhow::anyhow!(
                    "Custom metadata is not supported for {:?} files",
                    file_type
                ));
            }
        }

        Ok(())
    }

    pub fn get_song_filters_metadata(&self, file_path: &Path) -> anyhow::Result<Vec<Filter>> {
        let file_type = self.detect_file_type(file_path)?;

        let filters_string = match file_type {
            FileType::Aac | FileType::Mpeg | FileType::Wav => {
                self.read_id3_filters(file_path, FILTER_TAG_KEY)?
            }
            FileType::Flac => self.read_flac_filters(file_path, FILTER_TAG_KEY)?,
            FileType::Vorbis => self.read_vorbis_filters(file_path, FILTER_TAG_KEY)?,
            FileType::Opus => self.read_opus_filters(file_path, FILTER_TAG_KEY)?,
            FileType::Speex => self.read_speex_filters(file_path, FILTER_TAG_KEY)?,
            FileType::Mp4 => self.read_mp4_filters(file_path, FILTER_TAG_KEY)?,
            FileType::Ape => self.read_ape_filters(file_path, FILTER_TAG_KEY)?,
            _ => return Ok(vec![]),
        };

        Ok(Self::parse_filters_string(filters_string.as_deref()))
    }

    fn detect_file_type(&self, file_path: &Path) -> anyhow::Result<FileType> {
        Probe::open(file_path)?
            .guess_file_type()?
            .file_type()
            .ok_or_else(|| anyhow::anyhow!("Failed to detect file type for {:?}", file_path))
    }

    fn parse_filters_string(filters_string: Option<&str>) -> Vec<Filter> {
        filters_string
            .unwrap_or_default()
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|name| Filter {
                id: 0,
                name: name.to_string(),
            })
            .collect()
    }

    fn write_id3_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        match self.detect_file_type(file_path)? {
            FileType::Aac => {
                let mut reader = File::open(file_path)?;
                let mut file = AacFile::read_from(&mut reader, ParseOptions::new())?;
                let mut tag = file.id3v2_mut().map(std::mem::take).unwrap_or_default();
                self.upsert_id3_filters_tag(&mut tag, key, value);
                file.set_id3v2(tag);
                file.save_to_path(file_path, WriteOptions::default())?;
                Ok(())
            }
            FileType::Mpeg => {
                let mut reader = File::open(file_path)?;
                let mut file = MpegFile::read_from(&mut reader, ParseOptions::new())?;
                let mut tag = file.id3v2_mut().map(std::mem::take).unwrap_or_default();
                self.upsert_id3_filters_tag(&mut tag, key, value);
                file.set_id3v2(tag);
                file.save_to_path(file_path, WriteOptions::default())?;
                Ok(())
            }
            FileType::Wav => {
                let mut reader = File::open(file_path)?;
                let mut file = WavFile::read_from(&mut reader, ParseOptions::new())?;
                let mut tag = file.id3v2_mut().map(std::mem::take).unwrap_or_default();
                self.upsert_id3_filters_tag(&mut tag, key, value);
                file.set_id3v2(tag);
                file.save_to_path(file_path, WriteOptions::default())?;
                Ok(())
            }
            file_type => Err(anyhow::anyhow!(
                "Expected an ID3-backed file, got {:?}",
                file_type
            )),
        }
    }

    fn write_flac_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = FlacFile::read_from(&mut reader, ParseOptions::new())?;
        let mut tag = file
            .vorbis_comments_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        let _ = tag.remove(key);
        if !value.is_empty() {
            tag.insert(key.to_string(), value.to_string());
        }
        file.set_vorbis_comments(tag);
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn write_vorbis_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = VorbisFile::read_from(&mut reader, ParseOptions::new())?;
        let tag = file.vorbis_comments_mut();
        let _ = tag.remove(key);
        if !value.is_empty() {
            tag.insert(key.to_string(), value.to_string());
        }
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn write_opus_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = OpusFile::read_from(&mut reader, ParseOptions::new())?;
        let tag = file.vorbis_comments_mut();
        let _ = tag.remove(key);
        if !value.is_empty() {
            tag.insert(key.to_string(), value.to_string());
        }
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn write_speex_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = SpeexFile::read_from(&mut reader, ParseOptions::new())?;
        let tag = file.vorbis_comments_mut();
        let _ = tag.remove(key);
        if !value.is_empty() {
            tag.insert(key.to_string(), value.to_string());
        }
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn write_mp4_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = Mp4File::read_from(&mut reader, ParseOptions::new())?;
        let mut ilst = file
            .ilst_mut()
            .map(std::mem::take)
            .unwrap_or_else(Ilst::new);
        let ident = AtomIdent::Freeform {
            mean: Cow::Borrowed("com.apple.iTunes"),
            name: Cow::Owned(key.to_string()),
        };
        let _ = ilst.remove(&ident);
        if !value.is_empty() {
            ilst.insert(Atom::new(ident, AtomData::UTF8(value.to_string())));
        }
        file.set_ilst(ilst);
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn write_ape_filters(&self, file_path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
        let mut reader = File::open(file_path)?;
        let mut file = ApeFile::read_from(&mut reader, ParseOptions::new())?;
        let mut tag = file.ape_mut().map(std::mem::take).unwrap_or_default();
        tag.remove(key);
        if !value.is_empty() {
            tag.insert(lofty::ape::ApeItem::new(
                key.to_string(),
                ItemValue::Text(value.to_string()),
            )?);
        }
        file.set_ape(tag);
        file.save_to_path(file_path, WriteOptions::default())?;
        Ok(())
    }

    fn read_id3_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        match self.detect_file_type(file_path)? {
            FileType::Aac => {
                let mut reader = File::open(file_path)?;
                let file = AacFile::read_from(&mut reader, ParseOptions::new())?;
                Ok(file
                    .id3v2()
                    .and_then(|tag| tag.get_user_text(key))
                    .map(str::to_string))
            }
            FileType::Mpeg => {
                let mut reader = File::open(file_path)?;
                let file = MpegFile::read_from(&mut reader, ParseOptions::new())?;
                Ok(file
                    .id3v2()
                    .and_then(|tag| tag.get_user_text(key))
                    .map(str::to_string))
            }
            FileType::Wav => {
                let mut reader = File::open(file_path)?;
                let file = WavFile::read_from(&mut reader, ParseOptions::new())?;
                Ok(file
                    .id3v2()
                    .and_then(|tag| tag.get_user_text(key))
                    .map(str::to_string))
            }
            file_type => Err(anyhow::anyhow!(
                "Expected an ID3-backed file, got {:?}",
                file_type
            )),
        }
    }

    fn read_flac_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = FlacFile::read_from(&mut reader, ParseOptions::new())?;
        Ok(file
            .vorbis_comments()
            .and_then(|tag| tag.get(key))
            .map(str::to_string))
    }

    fn read_vorbis_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = VorbisFile::read_from(&mut reader, ParseOptions::new())?;
        Ok(file.vorbis_comments().get(key).map(str::to_string))
    }

    fn read_opus_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = OpusFile::read_from(&mut reader, ParseOptions::new())?;
        Ok(file.vorbis_comments().get(key).map(str::to_string))
    }

    fn read_speex_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = SpeexFile::read_from(&mut reader, ParseOptions::new())?;
        Ok(file.vorbis_comments().get(key).map(str::to_string))
    }

    fn read_mp4_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = Mp4File::read_from(&mut reader, ParseOptions::new())?;
        let ident = AtomIdent::Freeform {
            mean: Cow::Borrowed("com.apple.iTunes"),
            name: Cow::Borrowed(key),
        };

        Ok(file.ilst().and_then(|ilst| {
            ilst.get(&ident).and_then(|atom| {
                atom.data().find_map(|data| match data {
                    AtomData::UTF8(value) | AtomData::UTF16(value) => Some(value.clone()),
                    _ => None,
                })
            })
        }))
    }

    fn read_ape_filters(&self, file_path: &Path, key: &str) -> anyhow::Result<Option<String>> {
        let mut reader = File::open(file_path)?;
        let file = ApeFile::read_from(&mut reader, ParseOptions::new())?;
        Ok(file
            .ape()
            .and_then(|tag| tag.get(key))
            .and_then(|item| item.value().text())
            .map(str::to_string))
    }

    fn upsert_id3_filters_tag(&self, tag: &mut Id3v2Tag, key: &str, value: &str) {
        tag.remove_user_text(key);
        if !value.is_empty() {
            let _ = tag.insert_user_text(key.to_string(), value.to_string());
        }
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
