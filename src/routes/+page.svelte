<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  type Song = {
    id: number;
    title: string;
    artist: string;
    release_year: number;
    album: string;
    remix: string;
    search_blob: string;
    file_path: string;
    duration: number;
  };

  type TrackChangedPayload = {
    currentIndex: number | null;
  };

  let isPlaying = false;
  let isShuffle = false;
  let isRepeat = false;

  let volume = 50;
  let previousVolume = 50;
  let isMuted = false;

  let searchQuery = "";
  let searchResultCount = 0;
  let songs: Song[] = [];
  let selectedIndex: number | null = null;
  let currentSong: Song | null = null;

  let timeout: ReturnType<typeof setTimeout> | undefined;

  function formatDuration(durationSeconds: number): string {
    const totalSeconds = Math.max(0, Math.floor(durationSeconds));
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  async function refreshLoadedSongs() {
    songs = await invoke<Song[]>("get_loaded_songs");
  }

  async function refreshCurrentSong() {
    currentSong = await invoke<Song | null>("get_current_song");
  }

  async function playSelectedSong(index: number) {
    try {
      await invoke("play_song_at", { index });
      isPlaying = true;
      await refreshCurrentSong();
    } catch (err) {
      console.error("Failed to play selected song:", err);
    }
  }

  async function play() {
    try {
      await invoke("play_pause");
      const paused = await invoke<boolean>("get_is_paused");
      isPlaying = !paused;
    } catch (err) {
      console.error("Failed to toggle play/pause:", err);
    }
  }

  async function previous() {
    try {
      await invoke("previous_song");
      isPlaying = true;
      await refreshCurrentSong();
    } catch (err) {
      console.error("Failed to go to previous song:", err);
    }
  }

  async function next() {
    try {
      await invoke("next_song");
      isPlaying = true;
      await refreshCurrentSong();
    } catch (err) {
      console.error("Failed to go to next song:", err);
    }
  }

  async function shuffle() {
    try {
      await invoke("set_random");
      isShuffle = !isShuffle;
    } catch (err) {
      console.error("Failed to toggle shuffle:", err);
    }
  }

  async function repeat() {
    try {
      await invoke("set_repeat");
      isRepeat = !isRepeat;
    } catch (err) {
      console.error("Failed to toggle repeat:", err);
    }
  }

  async function searchSongs() {
    try {
      const count = await invoke<number>("search_songs", {
        query: searchQuery,
      });
      searchResultCount = count;
      await refreshLoadedSongs();
      await refreshCurrentSong();
    } catch (err) {
      console.error("Failed to search songs:", err);
    }
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      void searchSongs();
    }
  }

  function changeVolume(event: Event) {
    const value = Number((event.target as HTMLInputElement).value);
    volume = value;

    if (value === 0) {
      isMuted = true;
    } else {
      isMuted = false;
      previousVolume = value;
    }

    if (timeout) clearTimeout(timeout);

    timeout = setTimeout(async () => {
      try {
        await invoke("set_volume", { volume: value / 100 });
      } catch (err) {
        console.error("Failed to set volume:", err);
      }
    }, 50);
  }

  async function toggleMute() {
    try {
      if (isMuted || volume === 0) {
        volume = previousVolume || 50;
        isMuted = false;
      } else {
        previousVolume = volume || 50;
        volume = 0;
        isMuted = true;
      }

      await invoke("set_volume", { volume: volume / 100 });
    } catch (err) {
      console.error("Failed to toggle mute:", err);
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        unlisten = await listen<TrackChangedPayload>(
          "track-changed",
          async (event) => {
            selectedIndex = event.payload.currentIndex;
            await refreshCurrentSong();
          },
        );

        const savedSearch = await invoke<string>("get_saved_search_blob");
        searchQuery = savedSearch;

        const count = await invoke<number>("load");
        searchResultCount = count;

        const savedVolume = await invoke<number>("get_volume");
        const savedShuffle = await invoke<boolean>("get_random");
        const savedRepeat = await invoke<boolean>("get_repeat");
        const paused = await invoke<boolean>("get_is_paused");

        volume = Math.round(savedVolume * 100);
        previousVolume = volume > 0 ? volume : 50;
        isMuted = volume === 0;

        isShuffle = savedShuffle;
        isRepeat = savedRepeat;
        isPlaying = !paused;

        await refreshLoadedSongs();
        await refreshCurrentSong();
      } catch (err) {
        console.error("Failed to initialize player:", err);
      }
    })();

    return () => {
      if (timeout) clearTimeout(timeout);
      if (unlisten) unlisten();
    };
  });
</script>

<div class="container">
  <h1>Music Player</h1>

  <div class="search">
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="Search songs, artist, album..."
      on:keydown={handleSearchKeydown}
    />
    <button class="search-button" on:click={searchSongs}>Search</button>
  </div>

  <p class="search-results">
    {searchResultCount}
    {searchResultCount === 1 ? "song" : "songs"}
  </p>

  <div class="song-list">
    <div class="song-list-body">
      <div class="song-list-header">
        <div>#</div>
        <div>Title</div>
        <div>Album</div>
        <div>Artist</div>
        <div>Duration</div>
      </div>

      {#each songs as song, i}
        <div
          class:selected={selectedIndex === i}
          class="song-row"
          role="button"
          tabindex="0"
          title={`Play ${song.title}`}
          on:click={() => playSelectedSong(i)}
          on:keydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              void playSelectedSong(i);
            }
          }}
        >
          <div class="index">{i + 1}</div>

          <div class="title-cell">
            <div class="song-title">{song.title}</div>
            {#if song.remix}
              <div class="song-subtitle">{song.remix}</div>
            {/if}
          </div>

          <div class="album-cell">{song.album}</div>
          <div class="artist-cell">{song.artist}</div>
          <div class="duration-cell">{formatDuration(song.duration)}</div>
        </div>
      {/each}
    </div>
  </div>

  <div class="bottom-bar">
    <div class="now-playing">
      {#if currentSong}
        <div class="cover-placeholder">♪</div>

        <div class="now-playing-text">
          <div class="now-playing-title">{currentSong.title}</div>
          <div class="now-playing-meta">
            {currentSong.artist} • {currentSong.album}
          </div>
        </div>

        <div class="now-playing-duration">
          {formatDuration(currentSong.duration)}
        </div>
      {:else}
        <div class="cover-placeholder">♪</div>

        <div class="now-playing-text">
          <div class="now-playing-title">Nothing playing</div>
          <div class="now-playing-meta">Choose a song from the list</div>
        </div>
      {/if}
    </div>

    <div class="controls">
      <button on:click={shuffle} class:active={isShuffle} title="Shuffle"
        >🔀</button
      >
      <button on:click={previous} title="Previous">⏮</button>
      <button class="play" on:click={play} title={isPlaying ? "Pause" : "Play"}>
        {isPlaying ? "⏸" : "▶"}
      </button>
      <button on:click={next} title="Next">⏭</button>
      <button on:click={repeat} class:active={isRepeat} title="Repeat">
        {#if isRepeat}
          🔂
        {:else}
          🔁
        {/if}
      </button>
    </div>

    <div class="volume">
      <button on:click={toggleMute} title="Mute">
        {#if isMuted || volume === 0}
          🔇
        {:else if volume < 50}
          🔉
        {:else}
          🔊
        {/if}
      </button>

      <input
        type="range"
        min="0"
        max="100"
        bind:value={volume}
        on:input={changeVolume}
        aria-label="Volume"
      />

      <span>{volume}%</span>
    </div>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    background: #121212;
    color: white;
    font-family: Arial, sans-serif;
  }

  .container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    gap: 1.25rem;
    padding: 2rem 2rem 1.5rem;
    background: #121212;
    color: white;
    box-sizing: border-box;
  }

  h1 {
    margin: 0;
  }

  .search {
    display: flex;
    gap: 0.75rem;
    align-items: center;
  }

  .search input {
    width: 320px;
    padding: 0.75rem 1rem;
    border: 1px solid #444;
    border-radius: 999px;
    font-size: 1rem;
    outline: none;
    background: #1f1f1f;
    color: white;
  }

  .search-button {
    border: none;
    border-radius: 999px;
    background: #222;
    color: white;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.75rem 1.25rem;
    width: auto;
    height: auto;
  }

  .search-results {
    margin: 0;
    color: #b3b3b3;
    font-size: 0.95rem;
  }

  .song-list {
    width: min(1000px, 95vw);
    background: #181818;
    border-radius: 12px;
    overflow: hidden;
  }

  .song-list-body {
    max-height: 52.5rem;
    overflow-y: auto;
  }

  .song-list-header,
  .song-row {
    display: grid;
    grid-template-columns: 56px 2.2fr 1.5fr 1.5fr 90px;
    gap: 1rem;
    align-items: center;
    padding: 0.9rem 1rem;
    box-sizing: border-box;
  }

  .song-list-header {
    color: #b3b3b3;
    font-size: 0.9rem;
    border-bottom: 1px solid #2a2a2a;
    background: #181818;
    position: sticky;
    top: 0;
    z-index: 2;
  }

  .song-row {
    width: 100%;
    border-bottom: 1px solid #202020;
    background: transparent;
    color: white;
    cursor: pointer;
    outline: none;
  }

  .song-row:hover {
    background: #242424;
  }

  .song-row.selected {
    background: #1f3a2a;
  }

  .song-row:focus-visible {
    box-shadow: inset 0 0 0 2px #1db954;
  }

  .index,
  .duration-cell {
    color: #b3b3b3;
    font-size: 0.95rem;
  }

  .title-cell,
  .album-cell,
  .artist-cell {
    min-width: 0;
  }

  .song-title,
  .album-cell,
  .artist-cell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .song-title {
    font-weight: 600;
  }

  .song-subtitle {
    color: #b3b3b3;
    font-size: 0.85rem;
    margin-top: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bottom-bar {
    width: min(1000px, 95vw);
    display: grid;
    grid-template-columns: minmax(220px, 1.2fr) auto minmax(180px, 1fr);
    gap: 1rem;
    align-items: center;
    background: #181818;
    border-radius: 12px;
    padding: 1rem;
    box-sizing: border-box;
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    min-width: 0;
  }

  .cover-placeholder {
    width: 52px;
    height: 52px;
    border-radius: 8px;
    background: #2a2a2a;
    display: grid;
    place-items: center;
    font-size: 1.4rem;
    color: #b3b3b3;
    flex-shrink: 0;
  }

  .now-playing-text {
    min-width: 0;
  }

  .now-playing-title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .now-playing-meta {
    color: #b3b3b3;
    font-size: 0.9rem;
    margin-top: 0.2rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .now-playing-duration {
    color: #b3b3b3;
    font-size: 0.9rem;
    margin-left: auto;
    flex-shrink: 0;
  }

  .controls {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: center;
  }

  .controls button,
  .volume button {
    border: none;
    border-radius: 50%;
    background: #222;
    color: white;
    cursor: pointer;
    font-size: 1.2rem;
    width: 48px;
    height: 48px;
  }

  .controls button.play {
    width: 60px;
    height: 60px;
    font-size: 1.5rem;
  }

  .controls button.active {
    background: #1db954;
  }

  .volume {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .volume input {
    width: 150px;
  }
</style>
