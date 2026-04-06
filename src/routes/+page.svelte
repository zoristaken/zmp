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

  let currentSeekSeconds = 0;
  let isSeeking = false;
  let isProgrammaticSeekReset = false;

  let volumeTimeout: ReturnType<typeof setTimeout> | undefined;
  let playbackInterval: ReturnType<typeof setInterval> | undefined;
  let searchTimeout: ReturnType<typeof setTimeout> | undefined;

  let hasInitialized = false;
  let lastSearchedQuery = "";

  function formatDuration(durationSeconds: number): string {
    const totalSeconds = Math.max(0, Math.floor(durationSeconds));
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }

  function formatDate(year: number): string {
    return year > 0 ? String(year) : "—";
  }

  function stopPlaybackTicker() {
    if (playbackInterval) {
      clearInterval(playbackInterval);
      playbackInterval = undefined;
    }
  }

  async function saveSeekProgress() {
    try {
      await invoke("save_current_song_seek", {
        seekValue: currentSeekSeconds,
      });
    } catch (err) {
      console.error("Failed to save seek position:", err);
    }
  }

  function startPlaybackTicker() {
    stopPlaybackTicker();

    playbackInterval = setInterval(() => {
      if (!isPlaying || isSeeking || !currentSong) return;

      if (currentSeekSeconds < currentSong.duration) {
        currentSeekSeconds += 1;
        void saveSeekProgress();
        return;
      }

      stopPlaybackTicker();
      resetSeekUi();
      void next();
    }, 1000);
  }

  function resetSeekUi() {
    isProgrammaticSeekReset = true;
    currentSeekSeconds = 0;

    queueMicrotask(() => {
      isProgrammaticSeekReset = false;
    });
  }

  async function refreshLoadedSongs() {
    songs = await invoke<Song[]>("get_loaded_songs");
  }

  async function refreshCurrentSong() {
    currentSong = await invoke<Song | null>("get_current_song");
  }

  async function refreshSavedSeek() {
    try {
      currentSeekSeconds = await invoke<number>("get_current_song_seek");
    } catch (err) {
      console.error("Failed to get saved seek position:", err);
    }
  }

  async function syncPlaybackState() {
    try {
      const paused = await invoke<boolean>("get_is_paused");
      isPlaying = !paused;

      if (isPlaying) {
        startPlaybackTicker();
      } else {
        stopPlaybackTicker();
      }
    } catch (err) {
      console.error("Failed to sync playback state:", err);
    }
  }

  async function handleTrackChange(newIndex: number | null) {
    selectedIndex = newIndex;

    stopPlaybackTicker();
    resetSeekUi();
    await refreshCurrentSong();
    await refreshSavedSeek();
    await syncPlaybackState();
  }

  async function playSelectedSong(index: number) {
    try {
      await invoke("play_song_at", { index });
    } catch (err) {
      console.error("Failed to play selected song:", err);
    }
  }

  async function play() {
    try {
      await invoke("play_pause");
      await syncPlaybackState();
      await invoke("set_play_pause", { isPlaying });
    } catch (err) {
      console.error("Failed to toggle play/pause:", err);
    }
  }

  async function previous() {
    try {
      await invoke("previous_song");
    } catch (err) {
      console.error("Failed to go to previous song:", err);
    }
  }

  async function next() {
    try {
      await invoke("next_song");
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

  async function performSearch(autoplayFirst = false) {
    try {
      const count = await invoke<number>("search_songs", {
        query: searchQuery,
      });

      searchResultCount = count;
      lastSearchedQuery = searchQuery;
      await refreshLoadedSongs();

      if (autoplayFirst && count > 0) {
        await playSelectedSong(0);
      }
    } catch (err) {
      console.error("Failed to search songs:", err);
    }
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();

      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = undefined;
      }

      void performSearch(true);
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

    if (volumeTimeout) clearTimeout(volumeTimeout);

    volumeTimeout = setTimeout(async () => {
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

  let isDraggingSeek = false;

  function onSeekInput() {
    if (isProgrammaticSeekReset || !currentSong) return;
    isSeeking = true;
    isDraggingSeek = true;
    stopPlaybackTicker();
  }

  async function commitSeek() {
    if (isProgrammaticSeekReset || !currentSong) {
      isSeeking = false;
      isDraggingSeek = false;
      return;
    }

    try {
      await invoke("set_current_song_seek", {
        seekValue: currentSeekSeconds,
      });
      await saveSeekProgress();
    } catch (err) {
      console.error("Failed to seek song:", err);
      resetSeekUi();
    } finally {
      isSeeking = false;
      isDraggingSeek = false;

      if (isPlaying) {
        startPlaybackTicker();
      }
    }
  }

  async function onSeekChange() {
    if (!isDraggingSeek) return;
    await commitSeek();
  }

  async function onSeekPointerUp() {
    if (!isDraggingSeek) return;
    await commitSeek();
  }

  $: if (hasInitialized && searchQuery !== lastSearchedQuery) {
    if (searchTimeout) clearTimeout(searchTimeout);

    searchTimeout = setTimeout(() => {
      void performSearch(false);
    }, 150);
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        await invoke<number>("init");

        unlisten = await listen<TrackChangedPayload>(
          "track-changed",
          async (event) => {
            await handleTrackChange(event.payload.currentIndex);
          },
        );

        const savedSearch = await invoke<string>("get_saved_search_blob");
        searchQuery = savedSearch;

        const count = await invoke<number>("load");
        searchResultCount = count;
        lastSearchedQuery = searchQuery;

        const savedVolume = await invoke<number>("get_volume");
        const savedShuffle = await invoke<boolean>("get_random");
        const savedRepeat = await invoke<boolean>("get_repeat");
        const initialIndex = await invoke<number | null>("get_current_index");

        volume = Math.round(savedVolume * 100);
        previousVolume = volume > 0 ? volume : 50;
        isMuted = volume === 0;

        isShuffle = savedShuffle;
        isRepeat = savedRepeat;

        await refreshLoadedSongs();
        await refreshCurrentSong();
        await refreshSavedSeek();
        await handleTrackChange(initialIndex);
        await syncPlaybackState();

        hasInitialized = true;
      } catch (err) {
        console.error("Failed to initialize player:", err);
      }
    })();

    return () => {
      if (volumeTimeout) clearTimeout(volumeTimeout);
      if (searchTimeout) clearTimeout(searchTimeout);
      if (playbackInterval) clearInterval(playbackInterval);
      if (unlisten) unlisten();
    };
  });
</script>

<div class="app-shell">
  <div class="main-panel">
    <div class="search-row">
      <div class="search" data-tauri-drag-region>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search songs, artist, album..."
          on:keydown={handleSearchKeydown}
        />
        <button class="search-button" on:click={() => performSearch(true)}>
          Search
        </button>
      </div>
    </div>

    <div class="song-list">
      <div class="song-list-body">
        <div class="song-list-header">
          <div>#</div>
          <div>Title</div>
          <div>Artist</div>
          <div>Album</div>
          <div>Date</div>
          <div class="header-duration">
            {searchResultCount}
            {searchResultCount === 1 ? " song" : " songs"}
          </div>
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

            <div class="artist-cell">{song.artist}</div>
            <div class="album-cell">{song.album}</div>
            <div class="date-cell">{formatDate(song.release_year)}</div>
            <div class="duration-cell">{formatDuration(song.duration)}</div>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <div class="bottom-bar">
    <div class="seek-row">
      <span class="seek-time">{formatDuration(currentSeekSeconds)}</span>
      <input
        class="seek-slider"
        type="range"
        min="0"
        max={currentSong ? currentSong.duration : 0}
        step="1"
        bind:value={currentSeekSeconds}
        on:input={onSeekInput}
        on:pointerup={onSeekPointerUp}
        aria-label="Seek"
      />
      <span class="seek-time">
        {currentSong ? formatDuration(currentSong.duration) : "0:00"}
      </span>
    </div>

    <div class="now-playing">
      {#if currentSong}
        <div class="now-playing-content">
          <div class="now-playing-header">
            <div class="now-playing-title">{currentSong.title}</div>
          </div>

          <div class="now-playing-meta">{currentSong.artist}</div>
          <div class="now-playing-album">{currentSong.album}</div>

          {#if currentSong.remix}
            <div class="now-playing-remix">{currentSong.remix}</div>
          {/if}
        </div>
      {:else}
        <div class="now-playing-content">
          <div class="now-playing-header">
            <div class="now-playing-title">Nothing playing</div>
          </div>
          <div class="now-playing-meta">Choose a song from the list</div>
        </div>
      {/if}
    </div>

    <div class="bottom-controls-row">
      <div class="controls">
        <button on:click={shuffle} class:active={isShuffle} title="Shuffle">
          🔀
        </button>
        <button on:click={previous} title="Previous">⏮</button>
        <button
          class="play"
          on:click={play}
          title={isPlaying ? "Pause" : "Play"}
        >
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
</div>

<style>
  :global(html, body) {
    margin: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: #121212;
    color: white;
    font-family: Arial, sans-serif;
  }

  :global(body) {
    overscroll-behavior: none;
  }

  :global(#app) {
    width: 100%;
    height: 100vh;
    overflow: hidden;
  }

  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 1rem;
    padding: 1rem;
    box-sizing: border-box;
    background: #121212;
    color: white;
    overflow: hidden;
  }

  .main-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.75rem;
    overflow: hidden;
  }

  .search-row {
    width: 100%;
  }

  .search {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: center;
  }

  .search input {
    width: 100%;
    min-width: 0;
    padding: 0.85rem 1rem;
    border: 1px solid #444;
    border-radius: 999px;
    font-size: 1rem;
    outline: none;
    background: #1f1f1f;
    color: white;
    box-sizing: border-box;
  }

  .search-button {
    border: none;
    border-radius: 999px;
    background: #222;
    color: white;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.85rem 1.25rem;
    width: auto;
    height: auto;
    white-space: nowrap;
  }

  .song-list {
    min-height: 0;
    width: 100%;
    background: #181818;
    border-radius: 12px;
    overflow: hidden;
  }

  .song-list-body {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .song-list-header,
  .song-row {
    display: grid;
    grid-template-columns:
      56px
      minmax(220px, 2.4fr)
      minmax(180px, 1.7fr)
      minmax(220px, 2fr)
      90px
      120px;
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

  .header-duration {
    text-align: right;
    white-space: nowrap;
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
  .duration-cell,
  .date-cell {
    color: #b3b3b3;
    font-size: 0.95rem;
  }

  .duration-cell {
    text-align: right;
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
    width: 100%;
    display: grid;
    grid-template-rows: auto auto auto;
    gap: 1rem;
    align-items: stretch;
    background: #181818;
    border-radius: 12px;
    padding: 1.25rem;
    box-sizing: border-box;
  }

  .seek-row {
    display: grid;
    grid-template-columns: 52px minmax(0, 1fr) 52px;
    align-items: center;
    gap: 0.75rem;
  }

  .seek-time {
    color: #b3b3b3;
    font-size: 0.9rem;
    text-align: center;
    white-space: nowrap;
  }

  .seek-slider {
    width: 100%;
  }

  .now-playing {
    min-width: 0;
    display: flex;
    align-items: stretch;
  }

  .now-playing-content {
    min-width: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.35rem;
  }

  .now-playing-header {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    min-width: 0;
  }

  .now-playing-title {
    font-weight: 700;
    font-size: 1.05rem;
    line-height: 1.3;
    white-space: normal;
    word-break: break-word;
    overflow-wrap: anywhere;
    flex: 1;
  }

  .now-playing-meta,
  .now-playing-album,
  .now-playing-remix {
    color: #b3b3b3;
    font-size: 0.95rem;
    line-height: 1.35;
    white-space: normal;
    word-break: break-word;
    overflow-wrap: anywhere;
  }

  .bottom-controls-row {
    position: relative;
    min-height: 60px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .controls {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: center;
    flex-wrap: nowrap;
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
    flex-shrink: 0;
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
    position: absolute;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .volume input {
    width: 110px;
    max-width: 100%;
  }

  .volume span {
    color: #b3b3b3;
    font-size: 0.9rem;
    min-width: 42px;
    text-align: right;
  }

  @media (max-width: 980px) {
    .bottom-controls-row {
      min-height: auto;
      flex-direction: column;
      gap: 1rem;
    }

    .volume {
      position: static;
    }
  }
</style>
