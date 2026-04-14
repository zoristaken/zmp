CREATE TABLE IF NOT EXISTS song (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT NOT NULL,
    artist       TEXT NOT NULL,
    release_year INTEGER,
    album        TEXT,
    remix        TEXT,
    search_blob  TEXT,
    file_path    TEXT,
    extension    TEXT,
    duration     INTEGER,
    file_size    INTEGER NOT NULL,
    file_modified_millis INTEGER NOT NULL,
    UNIQUE(file_path)
);

CREATE TABLE IF NOT EXISTS filter
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    name                    TEXT NOT NULL,
    UNIQUE(name)
);

CREATE TABLE IF NOT EXISTS song_filter
(
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id                 INT NOT NULL,
    filter_id               INT NOT NULL,
    FOREIGN KEY (song_id)   REFERENCES song (id) ON UPDATE CASCADE ON DELETE CASCADE,
    FOREIGN KEY (filter_id) REFERENCES filter (id) ON UPDATE CASCADE ON DELETE CASCADE,
    UNIQUE(song_id, filter_id)
);

CREATE TABLE IF NOT EXISTS setting
(
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);


CREATE INDEX IF NOT EXISTS idx_song_sort_order
    ON song (title, artist, album, release_year, id);

CREATE INDEX IF NOT EXISTS idx_song_filter_filter_id_id
    ON song_filter (filter_id, id);

CREATE INDEX IF NOT EXISTS idx_song_filter_song_id_id
    ON song_filter (song_id, id);
