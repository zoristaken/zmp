use std::path::Path;

use id3::{Tag, TagLike, Version};
use tempfile::tempdir;

use zmp_lib::metadata::MetadataParser;

fn write_test_wav(path: &Path, duration_secs: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let total_samples = 8_000 * duration_secs;
    for _ in 0..total_samples {
        writer.write_sample(0i16).unwrap();
    }

    writer.finalize().unwrap();
}

fn write_id3_tag(
    path: &Path,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    year: Option<i32>,
    remixer: Option<&str>,
) {
    let mut tag = Tag::new();

    if let Some(title) = title {
        tag.set_title(title);
    }

    if let Some(artist) = artist {
        tag.set_artist(artist);
    }

    if let Some(album) = album {
        tag.set_album(album);
    }

    if let Some(year) = year {
        tag.set_year(year);
    }

    if let Some(remixer) = remixer {
        tag.add_frame(id3::Frame::with_content(
            "TPE4",
            id3::Content::Text(remixer.to_string()),
        ));
    }

    tag.write_to_path(path, Version::Id3v24).unwrap();
}

#[test]
fn parse_song_metadata_reads_tagged_title_artist_album_year_and_remix() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("ignored fallback - ignored title.wav");

    write_test_wav(&file, 2);
    write_id3_tag(
        &file,
        Some("Teardrop"),
        Some("Massive Attack"),
        Some("Mezzanine"),
        Some(1998),
        Some("Mad Professor"),
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.id, 0);
    assert_eq!(song.title, "Teardrop");
    assert_eq!(song.artist, "Massive Attack");
    assert_eq!(song.album, "Mezzanine");
    assert_eq!(song.release_year, 1998);
    assert_eq!(song.remix, "Mad Professor");
    assert_eq!(song.duration, 2);
    assert_eq!(
        song.search_blob,
        "teardrop massive attack mezzanine 1998 mad professor"
    );
}

#[test]
fn parse_song_metadata_prefers_tags_over_filename_fallback_when_tags_exist() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Wrong Artist - Wrong Title.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        Some("Windowlicker"),
        Some("Aphex Twin"),
        Some("Windowlicker"),
        Some(1999),
        Some("zor"),
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.title, "Windowlicker");
    assert_eq!(song.artist, "Aphex Twin");
    assert_eq!(song.album, "Windowlicker");
    assert_eq!(song.release_year, 1999);
    assert_eq!(song.remix, "zor");
    assert_eq!(
        song.search_blob,
        "windowlicker aphex twin windowlicker 1999 zor"
    );
}

#[test]
fn parse_song_metadata_falls_back_to_filename_when_title_and_artist_tags_are_missing() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Boards of Canada - Roygbiv.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        None,
        None,
        Some("Music Has the Right to Children"),
        Some(1998),
        None,
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.title, "Roygbiv");
    assert_eq!(song.artist, "Boards of Canada");
    assert_eq!(song.album, "Music Has the Right to Children");
    assert_eq!(song.release_year, 1998);
    assert_eq!(song.remix, "");
    assert_eq!(
        song.search_blob,
        "roygbiv boards of canada music has the right to children 1998"
    );
}

#[test]
fn parse_song_metadata_falls_back_only_for_missing_title() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Burial - Archangel.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        None,
        Some("Burial"),
        Some("Untrue"),
        Some(2007),
        Some("zor"),
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.title, "Archangel");
    assert_eq!(song.artist, "Burial");
    assert_eq!(song.album, "Untrue");
    assert_eq!(song.release_year, 2007);
    assert_eq!(song.remix, "zor");
    assert_eq!(song.search_blob, "archangel burial untrue 2007 zor");
}

#[test]
fn parse_song_metadata_falls_back_only_for_missing_artist() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Massive Attack - Angel.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        Some("Angel"),
        None,
        Some("Mezzanine"),
        Some(1998),
        None,
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.title, "Angel");
    assert_eq!(song.artist, "Massive Attack");
    assert_eq!(song.album, "Mezzanine");
    assert_eq!(song.release_year, 1998);
    assert_eq!(song.remix, "");
    assert_eq!(song.search_blob, "angel massive attack mezzanine 1998");
}

#[test]
fn parse_song_metadata_ignores_empty_zero_and_dash_values_in_search_blob() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Artist Name - Song Title.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        Some("Song Title"),
        Some("Artist Name"),
        Some("-"),
        Some(0),
        Some("-"),
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);

    let song = &songs[0];
    assert_eq!(song.title, "Song Title");
    assert_eq!(song.artist, "Artist Name");
    assert_eq!(song.album, "-");
    assert_eq!(song.release_year, 0);
    assert_eq!(song.remix, "-");

    // album/remix = "-" and year = 0 should be excluded from search_blob
    assert_eq!(song.search_blob, "song title artist name");
}

#[test]
fn parse_song_metadata_reads_multiple_tagged_files() {
    let dir = tempdir().unwrap();

    let first_path = dir.path().join("a.wav");
    let second_path = dir.path().join("b.wav");

    write_test_wav(&first_path, 1);
    write_test_wav(&second_path, 2);

    write_id3_tag(
        &first_path,
        Some("Teardrop"),
        Some("Massive Attack"),
        Some("Mezzanine"),
        Some(1998),
        None,
    );

    write_id3_tag(
        &second_path,
        Some("Xtal"),
        Some("Aphex Twin"),
        Some("Selected Ambient Works 85-92"),
        Some(1992),
        None,
    );

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 2);
}

#[test]
fn parse_song_metadata_skips_broken_supported_files_and_keeps_valid_tagged_files() {
    let dir = tempdir().unwrap();

    let valid = dir.path().join("valid.wav");
    let broken = dir.path().join("broken.wav");

    write_test_wav(&valid, 1);
    write_id3_tag(
        &valid,
        Some("Archangel"),
        Some("Burial"),
        Some("Untrue"),
        Some(2007),
        None,
    );

    std::fs::write(&broken, b"not really a wav file").unwrap();

    let parser = MetadataParser::new();
    let songs = parser.parse_song_metadata(dir.path()).unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Archangel");
    assert_eq!(songs[0].artist, "Burial");
}
