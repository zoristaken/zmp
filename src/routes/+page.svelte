<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, tick } from "svelte";
  import {
    Play,
    Pause,
    SkipBack,
    SkipForward,
    Shuffle,
    Repeat,
    Volume2,
    Volume1,
    VolumeX,
    Search,
    Settings2,
    X,
    Tags,
    Plus,
  } from "lucide-svelte";

  type Filter = {
    id: number;
    name: string;
  };

  type SongFilter = {
    id: number;
    song_id: number;
    filter_id: number;
  };

  type Song = {
    song: {
      id: number;
      title: string;
      artist: string;
      release_year: number;
      album: string;
      remix: string;
      search_blob: string;
      file_path: string;
      duration: number;
      extension: string;
    };
    filters: Filter[];
  };

  type TrackChangedPayload = {
    currentIndex: number | null;
  };

  type KeybindAction =
    | "playPause"
    | "previous"
    | "next"
    | "repeat"
    | "shuffle"
    | "mute"
    | "toggleSearch"
    | "toggleSettings";

  type KeybindMap = Record<KeybindAction, string>;

  const defaultKeybinds: KeybindMap = {
    playPause: "Space",
    previous: "A",
    next: "D",
    repeat: "R",
    shuffle: "S",
    mute: "M",
    toggleSearch: "Ctrl+E",
    toggleSettings: "Z",
  };

  const keybindLabels: Record<KeybindAction, string> = {
    playPause: "Play / Pause",
    previous: "Previous",
    next: "Next",
    repeat: "Repeat",
    shuffle: "Shuffle",
    mute: "Mute / Unmute",
    toggleSearch: "Focus / Unfocus search",
    toggleSettings: "Open / Close settings",
  };

  const getKeybindCommands: Record<KeybindAction, string> = {
    playPause: "get_play_pause_keybind",
    previous: "get_previous_keybind",
    next: "get_next_keybind",
    repeat: "get_repeat_keybind",
    shuffle: "get_shuffle_keybind",
    mute: "get_mute_keybind",
    toggleSearch: "get_focus_search_keybind",
    toggleSettings: "get_settings_keybind",
  };

  const setKeybindCommands: Record<KeybindAction, string> = {
    playPause: "set_play_pause_keybind",
    previous: "set_previous_keybind",
    next: "set_next_keybind",
    repeat: "set_repeat_keybind",
    shuffle: "set_shuffle_keybind",
    mute: "set_mute_keybind",
    toggleSearch: "set_focus_search_keybind",
    toggleSettings: "set_settings_keybind",
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
  let selectedSongId: number | null = null;
  let currentSong: Song | null = null;

  let allFilters: Filter[] = [];

  let currentSeekSeconds = 0;
  let isSeeking = false;
  let isProgrammaticSeekReset = false;
  let isDraggingSeek = false;

  let volumeTimeout: ReturnType<typeof setTimeout> | undefined;
  let playbackInterval: ReturnType<typeof setInterval> | undefined;
  let searchTimeout: ReturnType<typeof setTimeout> | undefined;
  let songRowElements: Array<HTMLDivElement | null> = [];

  let hasInitialized = false;
  let lastSearchedQuery = "";

  let searchInput: HTMLInputElement | null = null;
  let volumeSlider: HTMLInputElement | null = null;
  let isSettingsOpen = false;
  let captureAction: KeybindAction | null = null;
  let keybinds: KeybindMap = { ...defaultKeybinds };
  let activeSettingsSection: "general" | "keybinds" = "general";
  let musicFolderPath = "";
  let isPickingMusicFolder = false;
  let hasProcessedMusicFolder = false;
  let isInitialSetupRequired = false;
  let hasLoadedSetupState = false;
  let isMusicFolderConfirmOpen = false;
  let pendingMusicFolderPath: string | null = null;
  let songListLimit = 10000;
  let songListLimitInput = "10000";
  let isSavingSongListLimit = false;
  let songListLimitMessage = "";
  let songListLimitMessageKind: "success" | "error" | null = null;
  let setupMessage = "";
  let setupMessageKind: "success" | "error" | "info" | null = null;

  let isSongFilterMenuOpen = false;
  let songFilterTargetSong: Song | null = null;
  let songFilterLinksForTarget: SongFilter[] = [];
  let isAssigningSongFilter = false;
  let isRemovingSongFilter = false;
  let songFilterMessage = "";

  let isFilterLibraryMenuOpen = false;
  let newFilterInput = "";
  let isSavingGlobalFilter = false;
  let isRemovingGlobalFilter = false;
  let filterLibraryMessage = "";

  function hasMusicFolderPath(path: string): boolean {
    return path.trim().length > 0;
  }

  function requiresInitialSetup(): boolean {
    return !hasMusicFolderPath(musicFolderPath) || !hasProcessedMusicFolder;
  }

  function canCloseInitialSettings(): boolean {
    return !isInitialSetupRequired;
  }

  async function refreshSetupState() {
    try {
      musicFolderPath = await invoke<string>("get_music_folder_path");
    } catch (err) {
      console.error("Failed to load music folder path:", err);
      musicFolderPath = "";
    }

    try {
      hasProcessedMusicFolder = await invoke<boolean>(
        "has_processed_music_folder",
      );
    } catch (err) {
      console.error("Failed to load processed music folder flag:", err);
      hasProcessedMusicFolder = false;
    }

    try {
      songListLimit = await invoke<number>("get_song_list_limit");
      songListLimitInput = String(songListLimit);
    } catch (err) {
      console.error("Failed to load song list limit:", err);
      songListLimit = 10000;
      songListLimitInput = "10000";
    }

    isInitialSetupRequired = requiresInitialSetup();
    hasLoadedSetupState = true;
  }

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

  async function ensureSelectedSongIsVisible() {
    await tick();

    const index = songs.findIndex((song) => song.song.id === selectedSongId);
    if (index < 0) return;

    const row = songRowElements[index];
    row?.scrollIntoView({
      block: "center",
      inline: "nearest",
      behavior: "smooth",
    });
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

  async function refreshAllFilters() {
    try {
      allFilters = await invoke<Filter[]>("get_filters");
    } catch (err) {
      console.error("Failed to load filters:", err);
      allFilters = [];
    }
  }

  function startPlaybackTicker() {
    stopPlaybackTicker();

    playbackInterval = setInterval(() => {
      if (!isPlaying || isSeeking || !currentSong) return;

      if (currentSeekSeconds < currentSong.song.duration) {
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
      const paused = await invoke<boolean>("get_is_player_paused");
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
    await refreshLoadedSongs();
    await refreshSavedSeek();
    await syncPlaybackState();

    if (currentSong) {
      selectedSongId = currentSong.song.id;

      const visibleSelectedIndex = songs.findIndex(
        (song) => song.song.id === currentSong?.song.id,
      );
      selectedIndex = visibleSelectedIndex >= 0 ? visibleSelectedIndex : null;
    } else if (newIndex !== null && songs[newIndex]) {
      selectedSongId = songs[newIndex].song.id;
    } else {
      selectedSongId = null;
      selectedIndex = null;
    }

    await ensureSelectedSongIsVisible();
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
      const nextIsPlaying = !isPlaying;

      await invoke("set_play_pause", { isPlaying: nextIsPlaying });
      isPlaying = nextIsPlaying;

      if (isPlaying) {
        startPlaybackTicker();
      } else {
        stopPlaybackTicker();
        await saveSeekProgress();
      }
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
      const nextValue = !isShuffle;
      await invoke("set_random", { isRandom: nextValue });
      isShuffle = nextValue;
    } catch (err) {
      console.error("Failed to toggle shuffle:", err);
    }
  }

  async function repeat() {
    try {
      const nextValue = !isRepeat;
      await invoke("set_repeat", { isRepeat: nextValue });
      isRepeat = nextValue;
    } catch (err) {
      console.error("Failed to toggle repeat:", err);
    }
  }

  async function performSearch() {
    const previousSelectedSongId = selectedSongId;
    const previousSelectedIndex = selectedIndex;

    try {
      const count = await invoke<number>("search_songs", {
        query: searchQuery,
      });

      searchResultCount = count;
      lastSearchedQuery = searchQuery;
      await refreshLoadedSongs();

      let nextSelectedIndex: number | null = null;

      if (previousSelectedSongId !== null) {
        const visibleSelectedIndex = songs.findIndex(
          (song) => song.song.id === previousSelectedSongId,
        );
        nextSelectedIndex =
          visibleSelectedIndex >= 0 ? visibleSelectedIndex : null;
      }

      if (
        nextSelectedIndex === null &&
        previousSelectedIndex !== null &&
        previousSelectedIndex >= 0 &&
        previousSelectedIndex < songs.length
      ) {
        nextSelectedIndex = previousSelectedIndex;
      }

      if (nextSelectedIndex === null && count > 0) {
        nextSelectedIndex = 0;
      }

      selectedIndex = nextSelectedIndex;
      selectedSongId =
        nextSelectedIndex !== null && songs[nextSelectedIndex]
          ? songs[nextSelectedIndex].song.id
          : null;

      await ensureSelectedSongIsVisible();

      if (nextSelectedIndex !== null) {
        await playSelectedSong(nextSelectedIndex);
      }
    } catch (err) {
      console.error("Failed to search songs:", err);
    }
  }

  async function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();

      if (searchTimeout) {
        clearTimeout(searchTimeout);
        searchTimeout = undefined;
      }

      await performSearch();
      searchInput?.blur();
    }
  }

  function queueVolumePersist(nextVolume: number) {
    if (volumeTimeout) clearTimeout(volumeTimeout);

    volumeTimeout = setTimeout(async () => {
      try {
        await invoke("set_volume", { volume: nextVolume / 100 });
      } catch (err) {
        console.error("Failed to set volume:", err);
      }
    }, 50);
  }

  function setVolumeValue(nextVolume: number) {
    const clamped = Math.max(0, Math.min(100, Math.round(nextVolume)));
    volume = clamped;

    if (clamped === 0) {
      isMuted = true;
    } else {
      isMuted = false;
      previousVolume = clamped;
    }

    queueVolumePersist(clamped);
  }

  function changeVolume(event: Event) {
    const value = Number((event.target as HTMLInputElement).value);
    setVolumeValue(value);
  }

  function handleVolumeWheel(event: WheelEvent) {
    event.preventDefault();

    const delta = event.deltaY < 0 ? 2 : -2;
    setVolumeValue(volume + delta);
    volumeSlider?.blur();
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

  function isEditableTarget(target: EventTarget | null): boolean {
    const element = target as HTMLElement | null;
    if (!element) return false;

    const tag = element.tagName?.toLowerCase();
    return (
      element.isContentEditable ||
      tag === "input" ||
      tag === "textarea" ||
      tag === "select"
    );
  }

  function normalizeKeyName(key: string): string {
    const lower = key.toLowerCase();

    if (lower === " ") return "Space";
    if (lower === "esc") return "Escape";
    if (lower === "control") return "Ctrl";
    if (lower === "meta") return "Meta";
    if (lower === "alt") return "Alt";
    if (lower === "shift") return "Shift";
    if (lower.length === 1) return lower.toUpperCase();

    if (lower.startsWith("arrow")) {
      return `Arrow${lower.slice(5, 6).toUpperCase()}${lower.slice(6)}`;
    }

    return key.charAt(0).toUpperCase() + key.slice(1);
  }

  function isModifierOnlyKey(key: string): boolean {
    return ["Control", "Shift", "Alt", "Meta"].includes(key);
  }

  function keyEventToCombo(event: KeyboardEvent): string {
    const parts: string[] = [];

    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Meta");

    const normalizedKey = normalizeKeyName(event.key);

    if (!["Ctrl", "Alt", "Shift", "Meta"].includes(normalizedKey)) {
      parts.push(normalizedKey);
    }

    return parts.join("+");
  }

  async function loadKeybinds() {
    const nextKeybinds = { ...defaultKeybinds };

    for (const action of Object.keys(defaultKeybinds) as KeybindAction[]) {
      try {
        const value = await invoke<string | null>(getKeybindCommands[action]);
        if (value !== null && value !== undefined) {
          nextKeybinds[action] = value;
        }
      } catch (err) {
        console.error(`Failed to load keybind for ${action}:`, err);
      }
    }

    keybinds = nextKeybinds;
  }

  async function persistKeybind(action: KeybindAction, combo: string) {
    try {
      await invoke(setKeybindCommands[action], { keybind: combo });
    } catch (err) {
      console.error(`Failed to save keybind for ${action}:`, err);
    }
  }

  function clearSongListLimitMessage() {
    songListLimitMessage = "";
    songListLimitMessageKind = null;
  }

  function clearSetupMessage() {
    setupMessage = "";
    setupMessageKind = null;
  }

  function normalizePickedPath(path: string): string {
    const trimmed = path.trim();

    if (!trimmed.startsWith("file://")) {
      return trimmed;
    }

    try {
      const url = new URL(trimmed);
      const decodedPath = decodeURIComponent(url.pathname);

      if (/^\/[A-Za-z]:\//.test(decodedPath)) {
        return decodedPath.slice(1);
      }

      return decodedPath;
    } catch (err) {
      console.error("Failed to normalize picked folder path:", err);
      return trimmed;
    }
  }

  function extractPickedPath(value: unknown): string | null {
    if (typeof value === "string") {
      const normalizedPath = normalizePickedPath(value);
      return normalizedPath.length > 0 ? normalizedPath : null;
    }

    if (Array.isArray(value)) {
      for (const entry of value) {
        const normalizedPath = extractPickedPath(entry);
        if (normalizedPath) {
          return normalizedPath;
        }
      }

      return null;
    }

    if (!value || typeof value !== "object") {
      return null;
    }

    const record = value as Record<string, unknown>;
    for (const key of ["path", "filePath", "Path", "Url", "url"]) {
      const normalizedPath = extractPickedPath(record[key]);
      if (normalizedPath) {
        return normalizedPath;
      }
    }

    return null;
  }

  function closeMusicFolderConfirm() {
    isMusicFolderConfirmOpen = false;
    pendingMusicFolderPath = null;
  }

  async function applyMusicFolderSelection(folderPath: string) {
    await invoke("set_music_folder_path", { path: folderPath });
    await invoke("set_processed_music_folder", { flag: false });
    musicFolderPath = folderPath;
    hasProcessedMusicFolder = false;
    isInitialSetupRequired = requiresInitialSetup();
    setupMessage = "Processing selected folder...";
    setupMessageKind = "info";
    await tick();

    await invoke("process_music_folder");
    await refreshSetupState();

    searchQuery = "";
    lastSearchedQuery = "";

    const count = await invoke<number>("load");
    searchResultCount = count;

    const savedVolume = await invoke<number>("get_volume");
    const savedShuffle = await invoke<boolean>("get_random");
    const savedRepeat = await invoke<boolean>("get_repeat");
    const initialIndex = await invoke<number | null>("get_current_index");
    const savedIsPlaying = await invoke<boolean>("get_play_pause");

    volume = Math.round(savedVolume * 100);
    previousVolume = volume > 0 ? volume : 50;
    isMuted = volume === 0;
    isShuffle = savedShuffle;
    isRepeat = savedRepeat;
    isPlaying = savedIsPlaying;

    await refreshAllFilters();
    await handleTrackChange(initialIndex);

    setupMessage = "Music folder processed successfully.";
    setupMessageKind = "success";

    if (canCloseInitialSettings()) {
      closeSettings();
    } else {
      isSettingsOpen = true;
      activeSettingsSection = "general";
    }
  }

  async function confirmMusicFolderReplacement() {
    if (!pendingMusicFolderPath) {
      return;
    }

    const folderPath = pendingMusicFolderPath;
    closeMusicFolderConfirm();
    await applyMusicFolderSelection(folderPath);
  }

  function parseSongListLimitInput(): number | null {
    const parsed = Number.parseInt(songListLimitInput.trim(), 10);

    if (!Number.isInteger(parsed) || parsed <= 0) {
      return null;
    }

    return parsed;
  }

  async function saveSongListLimit() {
    const nextLimit = parseSongListLimitInput();

    if (nextLimit === null) {
      songListLimitMessage = "Enter a whole number greater than 0.";
      songListLimitMessageKind = "error";
      return;
    }

    if (nextLimit === songListLimit) {
      songListLimitInput = String(nextLimit);
      songListLimitMessage = "Already saved.";
      songListLimitMessageKind = "success";
      return;
    }

    isSavingSongListLimit = true;
    clearSongListLimitMessage();

    try {
      await invoke("set_song_list_limit", { limit: nextLimit });
      songListLimit = nextLimit;
      songListLimitInput = String(nextLimit);
      songListLimitMessage = "Saved.";
      songListLimitMessageKind = "success";

      if (hasInitialized && !requiresInitialSetup()) {
        searchResultCount = await invoke<number>("search_songs", {
          query: searchQuery,
        });
      }
    } catch (err) {
      console.error("Failed to save song list limit:", err);
      songListLimitMessage = "Failed to save song list limit.";
      songListLimitMessageKind = "error";
    } finally {
      isSavingSongListLimit = false;
    }
  }

  async function handleSongListLimitKeydown(event: KeyboardEvent) {
    if (event.key !== "Enter") return;

    event.preventDefault();
    await saveSongListLimit();
  }

  function openSettings() {
    isSettingsOpen = true;
    captureAction = null;
    activeSettingsSection = "general";
  }

  function closeSettings() {
    if (!canCloseInitialSettings()) {
      activeSettingsSection = "general";
      return;
    }

    isSettingsOpen = false;
    captureAction = null;
  }

  function toggleSettings() {
    if (isSettingsOpen) {
      closeSettings();
    } else {
      openSettings();
    }
  }

  function startKeyCapture(action: KeybindAction) {
    captureAction = action;
  }

  function clearKeybind(action: KeybindAction) {
    if (keybinds[action] === "") {
      captureAction = null;
      return;
    }

    keybinds = {
      ...keybinds,
      [action]: "",
    };

    void persistKeybind(action, "");
    captureAction = null;
  }

  async function resetKeybinds() {
    captureAction = null;

    for (const action of Object.keys(defaultKeybinds) as KeybindAction[]) {
      const defaultValue = defaultKeybinds[action];
      const currentValue = keybinds[action];

      if (currentValue !== defaultValue) {
        await persistKeybind(action, defaultValue);
      }
    }

    keybinds = { ...defaultKeybinds };
  }

  function toggleSearchFocus() {
    if (!searchInput) return;

    const isSearchFocused = document.activeElement === searchInput;

    if (isSearchFocused) {
      searchInput.blur();
      return;
    }

    searchInput.focus();
    searchInput.select();
  }

  function blurActiveElement() {
    const activeElement = document.activeElement;
    if (activeElement instanceof HTMLElement) {
      activeElement.blur();
    }
  }

  async function setKeybind(action: KeybindAction, combo: string) {
    const updated = { ...keybinds };
    const actionsToPersist: Array<[KeybindAction, string]> = [];

    for (const existingAction of Object.keys(updated) as KeybindAction[]) {
      if (existingAction !== action && updated[existingAction] === combo) {
        updated[existingAction] = "";
        actionsToPersist.push([existingAction, ""]);
      }
    }

    updated[action] = combo;
    actionsToPersist.push([action, combo]);

    keybinds = updated;

    for (const [persistAction, persistCombo] of actionsToPersist) {
      await persistKeybind(persistAction, persistCombo);
    }
  }

  async function runKeybindAction(action: KeybindAction) {
    switch (action) {
      case "playPause":
        await play();
        break;
      case "previous":
        await previous();
        break;
      case "next":
        await next();
        break;
      case "repeat":
        await repeat();
        break;
      case "shuffle":
        await shuffle();
        break;
      case "mute":
        await toggleMute();
        break;
      case "toggleSearch":
        toggleSearchFocus();
        return;
      case "toggleSettings":
        toggleSettings();
        return;
    }

    blurActiveElement();
  }

  async function pickMusicFolder() {
    if (isPickingMusicFolder) return;

    isPickingMusicFolder = true;
    clearSetupMessage();

    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select music folder",
        defaultPath: musicFolderPath || undefined,
      });

      if (selected === null) {
        return;
      }

      const folderPath = extractPickedPath(selected);

      if (!folderPath) {
        console.error("Unexpected music folder selection payload:", selected);
        setupMessage =
          "Couldn't read the selected folder. Please try choosing it again.";
        setupMessageKind = "error";
        return;
      }

      if (hasProcessedMusicFolder) {
        pendingMusicFolderPath = folderPath;
        isMusicFolderConfirmOpen = true;
        return;
      }

      await applyMusicFolderSelection(folderPath);
    } catch (err) {
      console.error("Failed to pick music folder:", err);
      setupMessage =
        err instanceof Error ? err.message : "Failed to process music folder.";
      setupMessageKind = "error";
      isInitialSetupRequired = requiresInitialSetup();
      isSettingsOpen = true;
      activeSettingsSection = "general";
    } finally {
      isPickingMusicFolder = false;
    }
  }

  async function handleGlobalKeydown(event: KeyboardEvent) {
    if (captureAction) {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        captureAction = null;
        return;
      }

      if (isModifierOnlyKey(event.key)) {
        return;
      }

      const combo = keyEventToCombo(event);
      if (!combo) return;

      await setKeybind(captureAction, combo);
      captureAction = null;
      return;
    }

    const combo = keyEventToCombo(event);
    const matchedEntry = (
      Object.entries(keybinds) as Array<[KeybindAction, string]>
    ).find(([, value]) => value && value === combo);

    if (
      isSettingsOpen ||
      isSongFilterMenuOpen ||
      isFilterLibraryMenuOpen ||
      isMusicFolderConfirmOpen
    ) {
      if (event.key === "Escape") {
        event.preventDefault();

        if (isMusicFolderConfirmOpen) {
          closeMusicFolderConfirm();
        } else if (isSongFilterMenuOpen) {
          closeSongFilterMenu();
        } else if (isFilterLibraryMenuOpen) {
          closeFilterLibraryMenu();
        } else {
          closeSettings();
        }

        return;
      }

      if (
        matchedEntry?.[0] === "toggleSettings" &&
        isSettingsOpen &&
        canCloseInitialSettings()
      ) {
        event.preventDefault();
        event.stopPropagation();
        toggleSettings();
      }

      return;
    }

    if (!matchedEntry) return;

    const targetIsEditable = isEditableTarget(event.target);
    const action = matchedEntry[0];

    if (targetIsEditable && action !== "toggleSearch") {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    await runKeybindAction(matchedEntry[0]);
  }

  async function openSongFilterMenu(song: Song) {
    songFilterTargetSong = song;
    songFilterMessage = "";
    songFilterLinksForTarget = [];
    isSongFilterMenuOpen = true;

    try {
      songFilterLinksForTarget = await invoke<SongFilter[]>(
        "get_filters_for_song",
        {
          songId: song.song.id,
        },
      );
    } catch (err) {
      console.error("Failed to load song filters:", err);
      songFilterMessage = "Failed to load filters for this song.";
    }
  }

  function closeSongFilterMenu() {
    isSongFilterMenuOpen = false;
    songFilterTargetSong = null;
    songFilterLinksForTarget = [];
    songFilterMessage = "";
    isAssigningSongFilter = false;
    isRemovingSongFilter = false;
  }

  function openFilterLibraryMenu() {
    newFilterInput = "";
    filterLibraryMessage = "";
    isFilterLibraryMenuOpen = true;
  }

  function closeFilterLibraryMenu() {
    isFilterLibraryMenuOpen = false;
    newFilterInput = "";
    filterLibraryMessage = "";
    isSavingGlobalFilter = false;
    isRemovingGlobalFilter = false;
  }

  function mapSongFiltersToFilters(songFilters: SongFilter[]): Filter[] {
    return songFilters
      .map((songFilter) =>
        allFilters.find((filter) => filter.id === songFilter.filter_id),
      )
      .filter((filter): filter is Filter => Boolean(filter));
  }

  function setSongFilterLinksForTarget(songFilters: SongFilter[]) {
    songFilterLinksForTarget = songFilters;
  }

  function updateSongFiltersLocally(songId: number, filters: Filter[]) {
    songs = songs.map((entry) =>
      entry.song.id === songId ? { ...entry, filters } : entry,
    );

    if (currentSong?.song.id === songId) {
      currentSong = {
        ...currentSong,
        filters,
      };
    }

    if (songFilterTargetSong?.song.id === songId) {
      songFilterTargetSong = {
        ...songFilterTargetSong,
        filters,
      };
    }
  }

  function removeFilterFromAllSongsLocally(filterId: number) {
    songs = songs.map((entry) => ({
      ...entry,
      filters: entry.filters.filter((filter) => filter.id !== filterId),
    }));

    if (currentSong) {
      currentSong = {
        ...currentSong,
        filters: currentSong.filters.filter((filter) => filter.id !== filterId),
      };
    }

    if (songFilterTargetSong) {
      songFilterTargetSong = {
        ...songFilterTargetSong,
        filters: songFilterTargetSong.filters.filter(
          (filter) => filter.id !== filterId,
        ),
      };
    }

    songFilterLinksForTarget = songFilterLinksForTarget.filter(
      (link) => link.filter_id !== filterId,
    );
  }

  async function assignExistingFilterToSong(filter: Filter) {
    if (!songFilterTargetSong || isAssigningSongFilter) return;

    const alreadyAssigned = songFilterTargetSong.filters.some(
      (existing) => existing.id === filter.id,
    );

    if (alreadyAssigned) {
      songFilterMessage = `"${filter.name}" is already on this song.`;
      return;
    }

    isAssigningSongFilter = true;
    songFilterMessage = "";

    try {
      const targetSongId = songFilterTargetSong.song.id;

      const savedOk = await invoke<boolean>("add_filter_to_song", {
        songId: targetSongId,
        filterId: filter.id,
      });

      if (!savedOk) {
        throw new Error("Backend reported add_filter_to_song = false");
      }

      const savedSongFilters = await invoke<SongFilter[]>(
        "get_filters_for_song",
        {
          songId: targetSongId,
        },
      );

      const wasSaved = savedSongFilters.some(
        (saved) => saved.filter_id === filter.id,
      );

      if (!wasSaved) {
        throw new Error("Assigned filter was not returned after saving.");
      }

      setSongFilterLinksForTarget(savedSongFilters);

      const savedFilters = mapSongFiltersToFilters(savedSongFilters);
      updateSongFiltersLocally(targetSongId, savedFilters);
      songFilterMessage = `Added "${filter.name}".`;
    } catch (err) {
      console.error("Failed to assign filter to song:", err);
      songFilterMessage = "Failed to add filter to song.";
    } finally {
      isAssigningSongFilter = false;
    }
  }

  async function removeFilterFromSong(filter: Filter) {
    if (!songFilterTargetSong || isRemovingSongFilter) return;

    const songFilterLink = songFilterLinksForTarget.find(
      (link) => link.filter_id === filter.id,
    );

    if (!songFilterLink) {
      songFilterMessage = `Could not find the link for "${filter.name}".`;
      return;
    }

    isRemovingSongFilter = true;
    songFilterMessage = "";

    try {
      const targetSongId = songFilterTargetSong.song.id;

      const removedOk = await invoke<boolean>("remove_filter_from_song", {
        songFilterId: songFilterLink.id,
      });

      if (!removedOk) {
        throw new Error("Backend reported remove_filter_from_song = false");
      }

      const savedSongFilters = await invoke<SongFilter[]>(
        "get_filters_for_song",
        {
          songId: targetSongId,
        },
      );

      const stillExists = savedSongFilters.some(
        (link) => link.id === songFilterLink.id,
      );

      if (stillExists) {
        throw new Error("Removed filter link still exists after delete.");
      }

      setSongFilterLinksForTarget(savedSongFilters);

      const savedFilters = mapSongFiltersToFilters(savedSongFilters);
      updateSongFiltersLocally(targetSongId, savedFilters);

      songFilterMessage = `Removed "${filter.name}".`;
    } catch (err) {
      console.error("Failed to remove filter from song:", err);
      songFilterMessage = "Failed to remove filter from song.";
    } finally {
      isRemovingSongFilter = false;
    }
  }

  async function createOrUpdateFilter() {
    const trimmed = newFilterInput.trim();
    if (!trimmed || isSavingGlobalFilter) return;

    isSavingGlobalFilter = true;
    filterLibraryMessage = "";

    try {
      const savedOk = await invoke<boolean>("create_filter", {
        filterName: trimmed,
      });

      if (!savedOk) {
        throw new Error("Backend reported create_filter = false");
      }

      const savedFilters = await invoke<Filter[]>("get_filters");
      allFilters = savedFilters;

      const wasSaved = savedFilters.some(
        (filter) => filter.name.toLowerCase() === trimmed.toLowerCase(),
      );

      if (!wasSaved) {
        throw new Error("Created filter was not returned after saving.");
      }

      filterLibraryMessage = `Saved "${trimmed}".`;
      newFilterInput = "";
    } catch (err) {
      console.error("Failed to create filter:", err);
      filterLibraryMessage = "Failed to save filter.";
    } finally {
      isSavingGlobalFilter = false;
    }
  }

  async function removeGlobalFilter(filter: Filter) {
    if (isRemovingGlobalFilter) return;

    isRemovingGlobalFilter = true;
    filterLibraryMessage = "";

    try {
      const removedOk = await invoke<boolean>("remove_filter", {
        filterId: filter.id,
      });

      if (!removedOk) {
        throw new Error("Backend reported remove_filter = false");
      }

      const savedFilters = await invoke<Filter[]>("get_filters");
      const stillExists = savedFilters.some((saved) => saved.id === filter.id);

      if (stillExists) {
        throw new Error("Removed filter still exists after delete.");
      }

      allFilters = savedFilters;
      removeFilterFromAllSongsLocally(filter.id);
      filterLibraryMessage = `Removed "${filter.name}".`;
    } catch (err) {
      console.error("Failed to remove filter:", err);
      filterLibraryMessage = "Failed to remove filter.";
    } finally {
      isRemovingGlobalFilter = false;
    }
  }

  async function handleCreateFilterKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      await createOrUpdateFilter();
    }
  }

  function availableFiltersForSong(song: Song | null): Filter[] {
    if (!song) return [];

    const usedIds = new Set(song.filters.map((filter) => filter.id));
    return allFilters.filter((filter) => !usedIds.has(filter.id));
  }

  $: if (hasLoadedSetupState && isInitialSetupRequired) {
    isSettingsOpen = true;
    activeSettingsSection = "general";
    captureAction = null;
  }

  $: if (hasInitialized && searchQuery !== lastSearchedQuery) {
    if (searchTimeout) clearTimeout(searchTimeout);

    searchTimeout = setTimeout(() => {
      void performSearch();
    }, 150);
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;

    window.addEventListener("keydown", handleGlobalKeydown);

    void (async () => {
      try {
        await loadKeybinds();

        unlisten = await listen<TrackChangedPayload>(
          "track-changed",
          async (event) => {
            await handleTrackChange(event.payload.currentIndex);
          },
        );

        const savedSearch = await invoke<string>("get_saved_search_blob");
        searchQuery = savedSearch;
        lastSearchedQuery = searchQuery;

        await refreshSetupState();

        if (isInitialSetupRequired) {
          isSettingsOpen = true;
          activeSettingsSection = "general";
          songs = [];
          currentSong = null;
          searchResultCount = 0;
          hasInitialized = true;
          return;
        }

        const count = await invoke<number>("load");
        searchResultCount = count;

        const savedVolume = await invoke<number>("get_volume");
        const savedShuffle = await invoke<boolean>("get_random");
        const savedRepeat = await invoke<boolean>("get_repeat");
        const initialIndex = await invoke<number | null>("get_current_index");
        const savedIsPlaying = await invoke<boolean>("get_play_pause");

        volume = Math.round(savedVolume * 100);
        previousVolume = volume > 0 ? volume : 50;
        isMuted = volume === 0;

        isShuffle = savedShuffle;
        isRepeat = savedRepeat;
        isPlaying = savedIsPlaying;

        await refreshLoadedSongs();
        await refreshAllFilters();
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
      window.removeEventListener("keydown", handleGlobalKeydown);

      if (volumeTimeout) clearTimeout(volumeTimeout);
      if (searchTimeout) clearTimeout(searchTimeout);
      if (playbackInterval) clearInterval(playbackInterval);
      if (unlisten) unlisten();
    };
  });
</script>

<div class="app-shell">
  <div
    class="app-content"
    class:app-disabled={isSettingsOpen ||
      isSongFilterMenuOpen ||
      isFilterLibraryMenuOpen ||
      isMusicFolderConfirmOpen}
  >
    <div class="main-panel">
      <div class="search-row">
        <div class="search-toolbar">
          <button
            class="settings-button"
            on:click={openSettings}
            title="Settings"
            aria-label="Open settings"
          >
            <Settings2 size={18} />
          </button>

          <button
            class="settings-button"
            on:click={openFilterLibraryMenu}
            title="Manage filters"
            aria-label="Manage filters"
          >
            <Tags size={18} />
          </button>

          <div class="search" data-tauri-drag-region>
            <input
              bind:this={searchInput}
              type="text"
              bind:value={searchQuery}
              placeholder="Search songs, artist, album..."
              on:keydown={handleSearchKeydown}
            />
            <button class="search-button" on:click={() => performSearch()}>
              <Search size={16} />
              <span>Search</span>
            </button>
          </div>
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
            <div>Ext</div>
            <div class="header-duration">
              {searchResultCount}
              {searchResultCount === 1 ? " song" : " songs"}
            </div>
          </div>

          {#each songs as songEntry, i (songEntry.song.id)}
            <div
              bind:this={songRowElements[i]}
              class:selected={selectedSongId === songEntry.song.id}
              class="song-row"
              role="button"
              tabindex="0"
              title={`Play ${songEntry.song.title}`}
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
                <div class="song-title">{songEntry.song.title}</div>

                {#if songEntry.song.remix}
                  <div class="song-subtitle">{songEntry.song.remix}</div>
                {/if}

                <div class="row-meta-under">
                  <button
                    class="song-inline-filter-button icon-only"
                    title="Manage filters for this song"
                    aria-label={`Manage filters for ${songEntry.song.title}`}
                    on:click|stopPropagation={() =>
                      openSongFilterMenu(songEntry)}
                  >
                    <Tags size={13} />
                  </button>

                  {#if songEntry.filters.length > 0}
                    <div class="song-tags inline-song-tags">
                      {#each songEntry.filters as filter (filter.id)}
                        <span class="song-tag">{filter.name}</span>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>

              <div class="artist-cell">{songEntry.song.artist}</div>
              <div class="album-cell">{songEntry.song.album}</div>
              <div class="date-cell">
                {formatDate(songEntry.song.release_year)}
              </div>
              <div class="extension-cell">{songEntry.song.extension}</div>
              <div class="duration-cell">
                {formatDuration(songEntry.song.duration)}
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <div class="bottom-bar">
      <div class="bottom-layout">
        <div class="now-playing">
          {#if currentSong}
            <div class="now-playing-content">
              <div class="now-playing-header">
                <div class="now-playing-title">{currentSong.song.title}</div>
              </div>

              <div class="now-playing-meta">{currentSong.song.artist}</div>
              <div class="now-playing-album">{currentSong.song.album}</div>

              {#if currentSong.song.remix}
                <div class="now-playing-remix">{currentSong.song.remix}</div>
              {/if}

              {#if currentSong.filters.length > 0}
                <div class="now-playing-tags">
                  {#each currentSong.filters as filter (filter.id)}
                    <span class="song-tag now-playing-tag">{filter.name}</span>
                  {/each}
                </div>
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

        <div class="center-player">
          <div class="controls">
            <button
              on:click={shuffle}
              class:active={isShuffle}
              class="control-button secondary"
              title="Shuffle"
              aria-label="Shuffle"
            >
              <Shuffle size={18} strokeWidth={2.2} />
            </button>

            <button
              on:click={previous}
              class="control-button secondary"
              title="Previous"
              aria-label="Previous"
            >
              <SkipBack size={19} strokeWidth={2.2} />
            </button>

            <button
              class="control-button play"
              on:click={play}
              title={isPlaying ? "Pause" : "Play"}
              aria-label={isPlaying ? "Pause" : "Play"}
            >
              {#if isPlaying}
                <Pause size={20} strokeWidth={2.6} fill="currentColor" />
              {:else}
                <Play size={20} strokeWidth={2.6} fill="currentColor" />
              {/if}
            </button>

            <button
              on:click={next}
              class="control-button secondary"
              title="Next"
              aria-label="Next"
            >
              <SkipForward size={19} strokeWidth={2.2} />
            </button>

            <button
              on:click={repeat}
              class:active={isRepeat}
              class="control-button secondary"
              title="Repeat"
              aria-label="Repeat"
            >
              <Repeat size={18} strokeWidth={2.2} />
            </button>
          </div>

          <div class="seek-row">
            <span class="seek-time">{formatDuration(currentSeekSeconds)}</span>
            {#if currentSong}
              <input
                class="seek-slider"
                type="range"
                min="0"
                max={currentSong.song.duration}
                step="1"
                bind:value={currentSeekSeconds}
                on:input={onSeekInput}
                on:change={onSeekChange}
                on:pointerup={onSeekPointerUp}
                aria-label="Seek"
              />
            {:else}
              <input
                class="seek-slider"
                type="range"
                min="0"
                max="0"
                value="0"
                disabled
                aria-label="Seek"
              />
            {/if}
            <span class="seek-time">
              {currentSong ? formatDuration(currentSong.song.duration) : "0:00"}
            </span>
          </div>
        </div>

        <div class="volume">
          <button
            class="volume-button"
            on:click={toggleMute}
            title="Mute"
            aria-label="Mute"
          >
            {#if isMuted || volume === 0}
              <VolumeX size={18} strokeWidth={2.2} />
            {:else if volume < 50}
              <Volume1 size={18} strokeWidth={2.2} />
            {:else}
              <Volume2 size={18} strokeWidth={2.2} />
            {/if}
          </button>

          <input
            bind:this={volumeSlider}
            type="range"
            min="0"
            max="100"
            bind:value={volume}
            on:input={changeVolume}
            on:wheel={handleVolumeWheel}
            aria-label="Volume"
          />

          <span>{volume}%</span>
        </div>
      </div>
    </div>
  </div>

  {#if isSongFilterMenuOpen && songFilterTargetSong}
    <div
      class="settings-overlay"
      role="presentation"
      on:click={(event) => {
        if (event.target === event.currentTarget) {
          closeSongFilterMenu();
        }
      }}
    >
      <div
        class="settings-modal filter-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="song-filter-title"
      >
        <div class="settings-header">
          <div class="settings-title-wrap">
            <div class="settings-icon">
              <Tags size={18} />
            </div>
            <div>
              <h2 id="song-filter-title">Manage song filters</h2>
              <p>Add or remove saved filters for this song.</p>
            </div>
          </div>

          <button
            class="settings-close"
            on:click={closeSongFilterMenu}
            title="Close"
            aria-label="Close song filter menu"
          >
            <X size={18} />
          </button>
        </div>

        <div class="filter-modal-content song-filter-layout">
          <div class="filter-song-summary">
            <div class="filter-song-title">
              {songFilterTargetSong.song.title}
            </div>
            <div class="filter-song-meta">
              {songFilterTargetSong.song.artist}
              {#if songFilterTargetSong.song.album}
                · {songFilterTargetSong.song.album}
              {/if}
            </div>
          </div>

          <div class="filter-save-message message-slot">
            {songFilterMessage || "\u00A0"}
          </div>

          <div class="filter-existing fixed-current-filters">
            <div class="filter-existing-label">Current filters</div>

            {#if songFilterTargetSong.filters.length > 0}
              <div class="list-panel fixed-three-list">
                <div class="stacked-filter-list">
                  {#each songFilterTargetSong.filters as filter (filter.id)}
                    <div class="stacked-filter-row">
                      <div class="stacked-filter-label">
                        <span class="song-tag">{filter.name}</span>
                      </div>
                      <button
                        class="current-filter-remove"
                        on:click={() => removeFilterFromSong(filter)}
                        disabled={isRemovingSongFilter}
                        title={`Remove ${filter.name}`}
                        aria-label={`Remove ${filter.name}`}
                      >
                        <X size={12} />
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="list-panel fixed-three-list empty-list-panel">
                <div class="filter-empty padded-empty">
                  No filters on this song yet.
                </div>
              </div>
            {/if}
          </div>

          <div class="filter-existing grow-panel">
            <div class="filter-existing-label">Available filters</div>

            {#if availableFiltersForSong(songFilterTargetSong).length > 0}
              <div class="list-panel fill-list-panel">
                <div class="stacked-filter-list">
                  {#each availableFiltersForSong(songFilterTargetSong) as filter (filter.id)}
                    <button
                      class="available-filter-row"
                      on:click={() => assignExistingFilterToSong(filter)}
                      disabled={isAssigningSongFilter}
                    >
                      <span class="available-filter-name">{filter.name}</span>
                      <span class="available-filter-action" aria-hidden="true">
                        <Plus size={14} />
                      </span>
                    </button>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="list-panel fill-list-panel empty-list-panel">
                <div class="filter-empty padded-empty">
                  No available filters. Create one in the filter library first.
                </div>
              </div>
            {/if}
          </div>
        </div>

        <div class="settings-footer">
          <button
            class="footer-button secondary-button"
            on:click={closeSongFilterMenu}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if isFilterLibraryMenuOpen}
    <div
      class="settings-overlay"
      role="presentation"
      on:click={(event) => {
        if (event.target === event.currentTarget) {
          closeFilterLibraryMenu();
        }
      }}
    >
      <div
        class="settings-modal filter-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="filter-library-title"
      >
        <div class="settings-header">
          <div class="settings-title-wrap">
            <div class="settings-icon">
              <Tags size={18} />
            </div>
            <div>
              <h2 id="filter-library-title">Filter library</h2>
              <p>Create filters that can later be attached to songs.</p>
            </div>
          </div>

          <button
            class="settings-close"
            on:click={closeFilterLibraryMenu}
            title="Close"
            aria-label="Close filter library"
          >
            <X size={18} />
          </button>
        </div>

        <div class="filter-modal-content library-filter-layout">
          <div class="filter-input-row">
            <input
              type="text"
              bind:value={newFilterInput}
              placeholder="Type a filter name..."
              on:keydown={handleCreateFilterKeydown}
            />
            <button
              class="footer-button"
              on:click={createOrUpdateFilter}
              disabled={isSavingGlobalFilter}
            >
              <Plus size={16} />
              <span>{isSavingGlobalFilter ? "Saving..." : "Save filter"}</span>
            </button>
          </div>

          <div class="filter-save-message message-slot">
            {filterLibraryMessage || "\u00A0"}
          </div>

          <div class="filter-existing grow-panel">
            <div class="filter-existing-label">Saved filters</div>

            {#if allFilters.length > 0}
              <div class="list-panel fill-list-panel">
                <div class="stacked-filter-list">
                  {#each allFilters as filter (filter.id)}
                    <div class="stacked-filter-row">
                      <div class="stacked-filter-label">
                        <span class="song-tag">{filter.name}</span>
                      </div>
                      <button
                        class="current-filter-remove danger-remove"
                        on:click={() => removeGlobalFilter(filter)}
                        disabled={isRemovingGlobalFilter}
                        title={`Delete ${filter.name}`}
                        aria-label={`Delete ${filter.name}`}
                      >
                        <X size={12} />
                      </button>
                    </div>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="list-panel fill-list-panel empty-list-panel">
                <div class="filter-empty padded-empty">
                  No filters created yet.
                </div>
              </div>
            {/if}
          </div>
        </div>

        <div class="settings-footer">
          <button
            class="footer-button secondary-button"
            on:click={closeFilterLibraryMenu}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if isMusicFolderConfirmOpen && pendingMusicFolderPath}
    <div class="settings-overlay confirm-overlay" role="presentation">
      <div
        class="settings-modal warning-modal confirm-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="music-folder-confirm-title"
      >
        <div class="settings-header">
          <div class="settings-title-wrap">
            <div class="settings-icon warning-icon">
              <Settings2 size={18} />
            </div>
            <div>
              <h2 id="music-folder-confirm-title">Replace music library?</h2>
              <p>
                Rebuilding the library removes existing song-filter
                associations.
              </p>
            </div>
          </div>

          <button
            class="settings-close"
            on:click={closeMusicFolderConfirm}
            title="Cancel"
            aria-label="Cancel music folder change"
          >
            <X size={18} />
          </button>
        </div>

        <div class="settings-list">
          <div class="settings-card warning-card">
            <div class="settings-card-title">Selected folder</div>
            <div class="settings-card-text">{pendingMusicFolderPath}</div>
            <div class="settings-card-text">
              Your saved filters will stay, but their links to songs will be
              cleared because the library is rebuilt from the selected folder.
            </div>
          </div>
        </div>

        <div class="settings-footer">
          <button
            class="footer-button secondary-button"
            on:click={closeMusicFolderConfirm}
            type="button"
          >
            Cancel
          </button>

          <button
            class="footer-button danger-button"
            on:click={confirmMusicFolderReplacement}
            type="button"
          >
            Replace library
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if isSettingsOpen}
    <div
      class="settings-overlay"
      role="presentation"
      on:click={(event) => {
        if (event.target === event.currentTarget && canCloseInitialSettings()) {
          closeSettings();
        }
      }}
    >
      <div
        class="settings-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <div class="settings-header">
          <div class="settings-title-wrap">
            <div class="settings-icon">
              <Settings2 size={18} />
            </div>
            <div>
              <h2 id="settings-title">Settings</h2>
              <p>General app settings and keyboard shortcuts.</p>
            </div>
          </div>

          <button
            class="settings-close"
            on:click={closeSettings}
            title="Close"
            aria-label="Close settings"
            disabled={!canCloseInitialSettings()}
          >
            <X size={18} />
          </button>
        </div>

        <div class="settings-sections">
          <button
            class:active={activeSettingsSection === "general"}
            class="settings-section-button"
            on:click={() => (activeSettingsSection = "general")}
          >
            General
          </button>

          <button
            class:active={activeSettingsSection === "keybinds"}
            class="settings-section-button"
            on:click={() => {
              if (canCloseInitialSettings()) {
                activeSettingsSection = "keybinds";
              }
            }}
            disabled={!canCloseInitialSettings()}
          >
            Keybinds
          </button>
        </div>

        {#if activeSettingsSection === "general"}
          <div class="settings-list">
            <div class="settings-card">
              <div class="settings-card-title">Music folder</div>
              <div class="settings-card-text">
                {musicFolderPath || "No folder selected yet."}
              </div>

              {#if setupMessage}
                <div
                  class:error={setupMessageKind === "error"}
                  class:info={setupMessageKind === "info"}
                  class:success={setupMessageKind === "success"}
                  class="settings-status"
                >
                  {setupMessage}
                </div>
              {/if}

              <div class="settings-card-actions">
                <button
                  class="footer-button"
                  on:click={pickMusicFolder}
                  disabled={isPickingMusicFolder}
                  type="button"
                >
                  {isPickingMusicFolder ? "Choosing..." : "Choose folder"}
                </button>
              </div>
            </div>

            <div class="settings-card">
              <div class="settings-card-title">Song list limit</div>
              <div class="settings-card-text">
                Caps how many songs can be loaded into the current list and
                search results. Higher values show more songs but can make
                refreshes slower.
              </div>

              <label class="settings-field">
                <span class="settings-field-label">Max songs</span>
                <input
                  class="settings-input"
                  type="text"
                  inputmode="numeric"
                  pattern="[0-9]*"
                  bind:value={songListLimitInput}
                  on:input={clearSongListLimitMessage}
                  on:keydown={handleSongListLimitKeydown}
                />
              </label>

              <div class="settings-card-text">
                Current saved limit: {songListLimit.toLocaleString()} songs.
              </div>

              {#if songListLimitMessage}
                <div
                  class:error={songListLimitMessageKind === "error"}
                  class:success={songListLimitMessageKind === "success"}
                  class="settings-status"
                >
                  {songListLimitMessage}
                </div>
              {/if}

              <div class="settings-card-actions">
                <button
                  class="footer-button"
                  on:click={saveSongListLimit}
                  disabled={isSavingSongListLimit}
                  type="button"
                >
                  {isSavingSongListLimit ? "Saving..." : "Save limit"}
                </button>
              </div>
            </div>
          </div>
        {:else if activeSettingsSection === "keybinds"}
          <div class="settings-list">
            {#each Object.keys(keybindLabels) as actionKey}
              <div class="keybind-row">
                <div class="keybind-info">
                  <div class="keybind-name">
                    {keybindLabels[actionKey as KeybindAction]}
                  </div>
                  <div class="keybind-help">
                    {#if captureAction === actionKey}
                      Press a key combination now. Press Escape to cancel.
                    {:else}
                      {keybinds[actionKey as KeybindAction] ||
                        "No shortcut set"}
                    {/if}
                  </div>
                </div>

                <div class="keybind-actions">
                  <button
                    class:capturing={captureAction === actionKey}
                    class="keybind-button"
                    on:click={() => startKeyCapture(actionKey as KeybindAction)}
                  >
                    {captureAction === actionKey ? "Listening..." : "Set"}
                  </button>

                  <button
                    class="keybind-button secondary-button"
                    on:click={() => clearKeybind(actionKey as KeybindAction)}
                  >
                    Clear
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div class="settings-footer">
          {#if activeSettingsSection === "keybinds"}
            <button
              class="footer-button secondary-button"
              on:click={resetKeybinds}
            >
              Reset defaults
            </button>
          {:else}
            <div></div>
          {/if}

          <button
            class="footer-button"
            on:click={closeSettings}
            disabled={!canCloseInitialSettings()}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  {/if}
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
    position: fixed;
    inset: 0;
    overflow: hidden;
  }

  :global(*:focus-visible) {
    outline: none;
    box-shadow: none;
  }

  .app-shell {
    position: fixed;
    inset: 0;
    padding: 1rem;
    box-sizing: border-box;
    background: #121212;
    color: white;
    overflow: hidden;
  }

  .app-content {
    width: 100%;
    height: 100%;
    min-height: 0;
    min-width: 0;
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 1rem;
    overflow: hidden;
  }

  .app-content.app-disabled {
    pointer-events: none;
    user-select: none;
  }

  .main-panel {
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 0.75rem;
  }

  .search-row {
    width: 100%;
    min-height: 0;
  }

  .search-toolbar {
    width: 100%;
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    gap: 0.75rem;
    align-items: center;
  }

  .settings-button {
    width: 46px;
    height: 46px;
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1b1b1b;
    color: #f2f2f2;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease,
      color 0.18s ease;
  }

  .settings-button:hover {
    background: #242424;
    border-color: #454545;
    color: #ffffff;
  }

  .settings-button:active {
    transform: scale(0.98);
  }

  .settings-button:disabled,
  .settings-close:disabled,
  .settings-section-button:disabled,
  .footer-button:disabled {
    opacity: 0.65;
    cursor: not-allowed;
    transform: none;
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

  .search input:focus {
    border-color: #5a5a5a;
  }

  .search-button {
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1b1b1b;
    color: #f2f2f2;
    cursor: pointer;
    font-size: 0.95rem;
    font-weight: 600;
    padding: 0.8rem 1.1rem;
    width: auto;
    height: auto;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease;
  }

  .search-button:hover {
    background: #242424;
    border-color: #454545;
  }

  .search-button:active {
    transform: scale(0.98);
  }

  .song-list {
    min-height: 0;
    min-width: 0;
    width: 100%;
    background: #181818;
    border-radius: 12px;
    overflow: hidden;
  }

  .song-list-body {
    height: 100%;
    min-height: 0;
    min-width: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .song-list-header,
  .song-row {
    display: grid;
    grid-template-columns:
      clamp(2.5rem, 4vw, 3.5rem)
      minmax(0, 2.4fr)
      minmax(0, 1.8fr)
      minmax(0, 1.8fr)
      clamp(4rem, 8vw, 6rem)
      clamp(3.5rem, 6vw, 5rem)
      clamp(3.5rem, 7vw, 4.5rem);
    gap: clamp(0.4rem, 1vw, 1rem);
    align-items: center;
    padding: 0.9rem 1rem;
    box-sizing: border-box;
    width: 100%;
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

  .song-list-header > div,
  .song-row > div {
    min-width: 0;
  }

  .header-duration {
    text-align: right;
    white-space: nowrap;
  }

  .song-row {
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
  .extension-cell,
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

  .row-meta-under {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.45rem 0.55rem;
    margin-top: 0.45rem;
  }

  .song-inline-filter-button {
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1d1d1d;
    color: #d7d7d7;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.74rem;
    font-weight: 600;
    padding: 0.25rem 0.6rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      color 0.18s ease,
      transform 0.18s ease;
  }

  .song-inline-filter-button.icon-only {
    width: 28px;
    height: 28px;
    padding: 0;
    justify-content: center;
    flex: 0 0 auto;
  }

  .song-inline-filter-button:hover {
    background: #262626;
    border-color: #474747;
    color: #ffffff;
  }

  .song-inline-filter-button:active {
    transform: scale(0.98);
  }

  .song-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .inline-song-tags {
    margin-top: 0;
  }

  .song-tag {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    padding: 0.18rem 0.5rem;
    border-radius: 999px;
    background: #262626;
    border: 1px solid #343434;
    color: #d8d8d8;
    font-size: 0.72rem;
    line-height: 1.2;
    white-space: nowrap;
  }

  .bottom-bar {
    width: 100%;
    min-height: 130px;
    background: #181818;
    border-radius: 12px;
    padding: 1rem 1.25rem;
    box-sizing: border-box;
    overflow: hidden;
  }

  .bottom-layout {
    position: relative;
    min-height: 98px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    overflow: hidden;
  }

  .now-playing {
    min-width: 0;
    width: min(340px, 28vw);
    display: flex;
    align-items: center;
    overflow: hidden;
    z-index: 1;
    margin-right: max(1rem, 22vw);
  }

  .now-playing-content {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 0.35rem;
    overflow: hidden;
  }

  .now-playing-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
    overflow: hidden;
  }

  .now-playing-title {
    font-weight: 700;
    font-size: clamp(0.98rem, 1vw, 1.05rem);
    line-height: 1.3;
  }

  .now-playing-meta,
  .now-playing-album,
  .now-playing-remix {
    color: #b3b3b3;
    font-size: clamp(0.88rem, 0.9vw, 0.95rem);
    line-height: 1.3;
  }

  .now-playing-title,
  .now-playing-meta,
  .now-playing-album,
  .now-playing-remix {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .now-playing-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.2rem;
  }

  .now-playing-tag {
    background: #1f2a21;
    border-color: #2b4430;
    color: #dff7e8;
  }

  .center-player {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    width: min(42vw, 720px);
    min-width: 420px;
    display: grid;
    grid-template-rows: auto auto;
    justify-items: center;
    gap: 0.8rem;
    z-index: 2;
    pointer-events: none;
  }

  .center-player > * {
    pointer-events: auto;
  }

  .controls {
    display: flex;
    gap: 0.55rem;
    align-items: center;
    justify-content: center;
    flex-wrap: nowrap;
  }

  .control-button,
  .volume-button {
    appearance: none;
    border: none;
    background: transparent;
    color: #b3b3b3;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition:
      color 0.16s ease,
      transform 0.16s ease,
      background-color 0.16s ease,
      opacity 0.16s ease;
  }

  .control-button.secondary {
    width: 36px;
    height: 36px;
  }

  .control-button.secondary:hover,
  .volume-button:hover {
    color: #ffffff;
    transform: scale(1.06);
  }

  .control-button.secondary.active {
    color: #1db954;
  }

  .control-button.play {
    width: 50px;
    height: 50px;
    border-radius: 999px;
    background: #ffffff;
    color: #111111;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.24);
  }

  .control-button.play:hover {
    background: #f8f8f8;
    color: #000000;
    transform: scale(1.06);
  }

  .control-button:active,
  .volume-button:active {
    transform: scale(0.96);
  }

  .seek-row {
    width: 100%;
    max-width: 560px;
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

  .volume {
    width: min(260px, 22vw);
    min-width: 180px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    align-self: end;
    z-index: 1;
    margin-left: max(1rem, 22vw);
  }

  .volume-button {
    width: 36px;
    height: 36px;
    flex: 0 0 auto;
  }

  .volume input {
    width: clamp(90px, 10vw, 120px);
    min-width: 90px;
    max-width: 120px;
  }

  .volume span {
    color: #b3b3b3;
    font-size: 0.9rem;
    min-width: 42px;
    text-align: right;
    flex: 0 0 auto;
  }

  .settings-overlay {
    position: fixed;
    inset: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: grid;
    justify-items: center;
    align-items: center;
    padding: 1rem;
    box-sizing: border-box;
    background: rgba(0, 0, 0, 0.55);
    z-index: 30;
  }

  .settings-modal {
    width: min(720px, 100%);
    height: min(880px, calc(100dvh - 2rem));
    max-width: 100%;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr) auto;
    overflow: hidden;
    background: #181818;
    border: 1px solid #2d2d2d;
    border-radius: 16px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    padding: 1rem;
    box-sizing: border-box;
  }

  .filter-modal {
    width: min(620px, 100%);
    height: min(720px, calc(100dvh - 2rem));
    max-height: min(720px, calc(100dvh - 2rem));
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
  }

  .warning-modal {
    width: min(560px, 100%);
  }

  .confirm-overlay {
    z-index: 50;
  }

  .confirm-modal {
    z-index: 51;
  }

  .filter-modal-content {
    min-height: 0;
    height: 100%;
    overflow: hidden;
  }

  .song-filter-layout,
  .library-filter-layout {
    display: grid;
    gap: 1rem;
    min-height: 0;
    height: 100%;
  }

  .song-filter-layout {
    grid-template-rows: auto auto auto minmax(0, 1fr);
  }

  .library-filter-layout {
    grid-template-rows: auto auto minmax(0, 1fr);
  }

  .settings-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .settings-title-wrap {
    display: flex;
    gap: 0.9rem;
    align-items: flex-start;
  }

  .settings-icon {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: #222222;
    color: #f4f4f4;
    display: grid;
    place-items: center;
    flex: 0 0 auto;
  }

  .warning-icon {
    background: #3a2618;
    color: #ffd8b2;
  }

  .settings-title-wrap h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .settings-title-wrap p {
    margin: 0.25rem 0 0;
    color: #b3b3b3;
    font-size: 0.92rem;
  }

  .settings-close {
    width: 38px;
    height: 38px;
    border: 1px solid #303030;
    border-radius: 999px;
    background: #202020;
    color: #f2f2f2;
    cursor: pointer;
    display: grid;
    place-items: center;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease;
  }

  .settings-close:hover {
    background: #2a2a2a;
    border-color: #484848;
  }

  .settings-sections {
    display: flex;
    gap: 0.55rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  .settings-section-button {
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1b1b1b;
    color: #d3d3d3;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    padding: 0.6rem 0.95rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      color 0.18s ease,
      transform 0.18s ease;
  }

  .settings-section-button:hover {
    background: #242424;
    border-color: #454545;
    color: #ffffff;
  }

  .settings-section-button.active {
    background: #1f3a2a;
    border-color: #2c6b45;
    color: #dff7e8;
  }

  .settings-list {
    min-height: 0;
    overflow: auto;
    display: grid;
    gap: 0.75rem;
    padding-right: 0.1rem;
    align-content: start;
    align-items: start;
  }

  .settings-card {
    background: #1d1d1d;
    border: 1px solid #2a2a2a;
    border-radius: 12px;
    padding: 1rem;
    display: grid;
    gap: 0.75rem;
  }

  .settings-card-title {
    font-weight: 600;
  }

  .settings-card-text {
    color: #b3b3b3;
    word-break: break-word;
  }

  .settings-card-actions {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .warning-card {
    border-color: #4a2e1f;
    background: #241812;
  }

  .settings-field {
    display: grid;
    gap: 0.45rem;
  }

  .settings-field-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: #d8d8d8;
  }

  .settings-input {
    width: min(100%, 14rem);
    border: 1px solid #343434;
    border-radius: 10px;
    background: #131313;
    color: #f5f5f5;
    padding: 0.75rem 0.9rem;
    font: inherit;
  }

  .settings-input:focus {
    outline: none;
    border-color: #4f8b61;
    box-shadow: 0 0 0 3px rgba(79, 139, 97, 0.18);
  }

  .settings-status {
    font-size: 0.9rem;
    color: #b3b3b3;
  }

  .settings-status.success {
    color: #9fd7b0;
  }

  .settings-status.info {
    color: #9ec8ff;
  }

  .settings-status.error {
    color: #f3a6a6;
  }

  .danger-button {
    background: #8e2f22;
    color: #fff5f2;
  }

  .danger-button:hover {
    background: #a93b2b;
  }

  .keybind-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 1rem;
    align-items: center;
    background: #1d1d1d;
    border: 1px solid #2a2a2a;
    border-radius: 12px;
    padding: 0.9rem 1rem;
  }

  .keybind-info {
    min-width: 0;
  }

  .keybind-name {
    font-weight: 600;
    margin-bottom: 0.3rem;
  }

  .keybind-help {
    color: #b3b3b3;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .keybind-actions {
    display: flex;
    gap: 0.55rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .keybind-button,
  .footer-button,
  .secondary-button {
    border: 1px solid #323232;
    border-radius: 999px;
    background: #1b1b1b;
    color: #f2f2f2;
    cursor: pointer;
    font-size: 0.92rem;
    font-weight: 600;
    padding: 0.7rem 1rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      transform 0.18s ease,
      color 0.18s ease;
  }

  .keybind-button:hover,
  .footer-button:hover,
  .secondary-button:hover {
    background: #242424;
    border-color: #454545;
  }

  .keybind-button:active,
  .footer-button:active,
  .secondary-button:active {
    transform: scale(0.98);
  }

  .keybind-button.capturing {
    background: #1f3a2a;
    border-color: #2c6b45;
    color: #dff7e8;
  }

  .secondary-button {
    background: transparent;
    color: #d3d3d3;
  }

  .settings-footer {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-top: 1rem;
    flex-wrap: wrap;
  }

  .filter-modal .settings-footer {
    justify-content: flex-end;
    align-items: center;
    flex-wrap: nowrap;
    flex-direction: row;
  }

  .filter-modal .settings-footer .footer-button {
    width: auto;
    min-width: 110px;
    flex: 0 0 auto;
  }

  .filter-song-summary {
    margin-bottom: 0;
    padding: 0.9rem 1rem;
    background: #1d1d1d;
    border: 1px solid #2a2a2a;
    border-radius: 12px;
  }

  .filter-song-title {
    font-weight: 700;
    font-size: 1rem;
    margin-bottom: 0.25rem;
  }

  .filter-song-meta {
    color: #b3b3b3;
    font-size: 0.92rem;
  }

  .filter-input-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: center;
  }

  .filter-input-row input {
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

  .filter-input-row input:focus {
    border-color: #5a5a5a;
  }

  .filter-save-message {
    color: #d6d6d6;
    font-size: 0.9rem;
  }

  .message-slot {
    min-height: 1.25rem;
    display: flex;
    align-items: center;
  }

  .filter-existing {
    background: #1d1d1d;
    border: 1px solid #2a2a2a;
    border-radius: 12px;
    padding: 0.9rem 1rem;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
  }

  .grow-panel {
    min-height: 0;
  }

  .fixed-current-filters {
    min-height: 0;
  }

  .filter-existing-label {
    font-weight: 600;
    margin-bottom: 0.7rem;
  }

  .filter-empty {
    color: #b3b3b3;
    font-size: 0.92rem;
  }

  .padded-empty {
    padding: 0.85rem;
  }

  .list-panel {
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    border: 1px solid #2a2a2a;
    border-radius: 10px;
    background: #181818;
  }

  .fill-list-panel {
    height: 100%;
    min-height: 0;
    max-height: none;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .fixed-three-list {
    height: 145px;
    max-height: 145px;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }

  .empty-list-panel {
    display: block;
  }

  .stacked-filter-list {
    display: block;
  }

  .stacked-filter-row,
  .available-filter-row {
    border-bottom: 1px solid #202020;
  }

  .stacked-filter-list > :last-child {
    border-bottom: none;
  }

  .stacked-filter-row {
    min-height: 48px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 32px;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.85rem;
    box-sizing: border-box;
    background: transparent;
  }

  .stacked-filter-label {
    min-width: 0;
    display: flex;
    align-items: center;
  }

  .stacked-filter-label .song-tag {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .available-filter-row {
    width: 100%;
    min-height: 48px;
    border-left: none;
    border-right: none;
    border-top: none;
    border-radius: 0;
    background: transparent;
    color: #f2f2f2;
    cursor: pointer;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 28px;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.85rem;
    box-sizing: border-box;
    text-align: left;
    transition:
      background 0.18s ease,
      color 0.18s ease;
  }

  .available-filter-row:hover {
    background: #242424;
  }

  .available-filter-row:active {
    background: #2a2a2a;
  }

  .available-filter-row:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .available-filter-name {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 0.84rem;
    font-weight: 600;
  }

  .available-filter-action {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    justify-self: end;
    flex: 0 0 auto;
  }

  .current-filter-remove {
    width: 24px;
    height: 24px;
    border: 1px solid #3a3a3a;
    border-radius: 999px;
    background: #202020;
    color: #d9d9d9;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    justify-self: end;
    padding: 0;
    margin-left: 0.35rem;
    margin-right: 0.25rem;
    transition:
      background 0.18s ease,
      border-color 0.18s ease,
      color 0.18s ease,
      transform 0.18s ease;
  }

  .current-filter-remove:hover {
    background: #2a2a2a;
    border-color: #505050;
    color: #ffffff;
  }

  .current-filter-remove:active {
    transform: scale(0.97);
  }

  .current-filter-remove:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .danger-remove:hover {
    background: #3a1f1f;
    border-color: #7a3636;
    color: #ffdede;
  }

  @media (max-width: 1180px) {
    .center-player {
      width: min(46vw, 660px);
      min-width: 380px;
    }

    .now-playing {
      width: min(300px, 26vw);
    }

    .volume {
      width: min(220px, 20vw);
      min-width: 170px;
    }
  }

  @media (max-width: 1100px) {
    .song-list-header,
    .song-row {
      grid-template-columns:
        clamp(2.25rem, 4vw, 3rem)
        minmax(0, 2.6fr)
        minmax(0, 1.8fr)
        minmax(0, 1.5fr)
        clamp(3.75rem, 7vw, 5rem)
        clamp(3rem, 5vw, 4rem)
        clamp(3.25rem, 6vw, 4rem);
    }
  }

  @media (max-width: 980px) {
    .bottom-bar {
      min-height: unset;
      height: auto;
    }

    .bottom-layout {
      position: static;
      display: grid;
      grid-template-columns: 1fr;
      gap: 1rem;
    }

    .center-player {
      position: static;
      left: auto;
      top: auto;
      transform: none;
      width: 100%;
      min-width: 0;
      max-width: 100%;
      order: 1;
      pointer-events: auto;
    }

    .now-playing {
      width: 100%;
      margin-right: 0;
      order: 2;
    }

    .volume {
      width: 100%;
      min-width: 0;
      margin-left: 0;
      justify-content: center;
      align-self: center;
      padding-bottom: 0;
      order: 3;
    }

    .seek-row {
      max-width: 100%;
    }
  }

  @media (max-width: 820px) {
    .song-list-header,
    .song-row {
      grid-template-columns:
        2.25rem
        minmax(0, 2.8fr)
        minmax(0, 1.8fr)
        minmax(0, 1.2fr)
        4.25rem
        3.5rem;
    }

    .song-list-header > :nth-child(6),
    .song-row > :nth-child(6) {
      display: none;
    }

    .keybind-row {
      grid-template-columns: 1fr;
      align-items: stretch;
    }

    .keybind-actions {
      justify-content: flex-start;
    }
  }

  @media (max-width: 640px) {
    .search-toolbar {
      grid-template-columns: 1fr;
    }

    .settings-button {
      width: 100%;
      border-radius: 12px;
      height: 44px;
    }

    .search {
      grid-template-columns: 1fr;
    }

    .search-button {
      width: 100%;
      justify-content: center;
    }

    .song-list-header,
    .song-row {
      grid-template-columns:
        2rem
        minmax(0, 2.8fr)
        minmax(0, 1.6fr)
        3.75rem
        3.5rem;
      padding: 0.8rem 0.75rem;
    }

    .song-list-header > :nth-child(4),
    .song-row > :nth-child(4) {
      display: none;
    }

    .settings-modal:not(.filter-modal):not(.warning-modal) .settings-footer {
      flex-direction: column;
    }

    .settings-modal:not(.filter-modal):not(.warning-modal) .footer-button {
      width: 100%;
    }

    .filter-input-row {
      grid-template-columns: 1fr;
    }

    .filter-modal {
      height: calc(100dvh - 2rem);
      max-height: calc(100dvh - 2rem);
    }

    .filter-modal .settings-footer {
      flex-direction: row;
      justify-content: flex-end;
    }

    .filter-modal .settings-footer .footer-button {
      width: auto;
    }

    .fixed-three-list {
      height: 145px;
      max-height: 145px;
    }
  }
</style>
