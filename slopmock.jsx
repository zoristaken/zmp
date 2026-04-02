import { useEffect, useMemo, useState } from "react";
import {
  Settings2,
  Play,
  SkipForward,
  SkipBack,
  Shuffle,
  Repeat2,
  Volume2,
  Plus,
  Moon,
  Sun,
} from "lucide-react";

const ACCENT = "indigo";
const DEFAULT_MODE = "dark";

const THEMES = {
  dark: {
    bg: "9 9 11",
    panel: "24 24 27",
    panelAlt: "39 39 42",
    panelHover: "63 63 70",
    border: "63 63 70",
    borderSoft: "82 82 91",
    text: "244 244 245",
    textMuted: "161 161 170",
    input: "39 39 42",
    inputBorder: "82 82 91",
    shadow: "0 10px 30px rgba(0,0,0,0.22)",
  },
  light: {
    bg: "244 244 245",
    panel: "255 255 255",
    panelAlt: "244 244 245",
    panelHover: "228 228 231",
    border: "212 212 216",
    borderSoft: "161 161 170",
    text: "24 24 27",
    textMuted: "113 113 122",
    input: "250 250 250",
    inputBorder: "212 212 216",
    shadow: "0 8px 24px rgba(0,0,0,0.08)",
  },
};

const ACCENTS = {
  indigo: {
    solid: "99 102 241",
    hover: "129 140 248",
    soft: "224 231 255",
    border: "129 140 248",
    ring: "99 102 241",
  },
  purple: {
    solid: "168 85 247",
    hover: "192 132 252",
    soft: "243 232 255",
    border: "192 132 252",
    ring: "168 85 247",
  },
  blue: {
    solid: "59 130 246",
    hover: "96 165 250",
    soft: "219 234 254",
    border: "96 165 250",
    ring: "59 130 246",
  },
  emerald: {
    solid: "16 185 129",
    hover: "52 211 153",
    soft: "209 250 229",
    border: "52 211 153",
    ring: "16 185 129",
  },
  rose: {
    solid: "244 63 94",
    hover: "251 113 133",
    soft: "255 228 230",
    border: "251 113 133",
    ring: "244 63 94",
  },
};

function getThemeStyle(mode, accent) {
  const base = THEMES[mode] ?? THEMES.dark;
  const tone = ACCENTS[accent] ?? ACCENTS.indigo;

  return {
    "--bg": base.bg,
    "--panel": base.panel,
    "--panel-alt": base.panelAlt,
    "--panel-hover": base.panelHover,
    "--border": base.border,
    "--border-soft": base.borderSoft,
    "--text": base.text,
    "--text-muted": base.textMuted,
    "--input": base.input,
    "--input-border": base.inputBorder,
    "--accent": tone.solid,
    "--accent-hover": tone.hover,
    "--accent-soft": tone.soft,
    "--accent-border": tone.border,
    "--accent-ring": tone.ring,
    "--app-shadow": base.shadow,
  };
}

function cls(...parts) {
  return parts.filter(Boolean).join(" ");
}

export default function MusicPlayerMock() {
  const sampleTags = ["Rock", "Pop", "Chill", "Instrumental", "Electronic", "Jazz", "Lo-fi", "Indie"];

  const songs = useMemo(() => {
    const randomDuration = () => {
      const m = Math.floor(Math.random() * 4) + 2;
      const s = Math.floor(Math.random() * 60)
        .toString()
        .padStart(2, "0");
      return `${m}:${s}`;
    };

    return Array.from({ length: 50 }, (_, i) => ({
      id: i + 1,
      title: `Song ${i + 1}`,
      artist: `Artist ${String.fromCharCode(65 + (i % 26))}`,
      tags: [...sampleTags]
        .sort(() => 0.5 - Math.random())
        .slice(0, Math.floor(Math.random() * 3) + 1),
      duration: randomDuration(),
      year: 2000 + (i % 25),
      album: `Album ${Math.floor(i / 3) + 1}`,
    }));
  }, []);

  const filters = useMemo(
    () =>
      Array.from({ length: 20 }, (_, i) => ({
        name: `Genre ${i + 1}`,
        count: Math.floor(Math.random() * 50),
      })),
    []
  );

  const history = useMemo(() => Array.from({ length: 20 }, (_, i) => `Played Song ${i + 1}`), []);

  const [currentSong, setCurrentSong] = useState(songs[0]);
  const [mode, setMode] = useState(DEFAULT_MODE);
  const [accent, setAccent] = useState(ACCENT);

  useEffect(() => {
    document.documentElement.style.colorScheme = mode;
  }, [mode]);

  const themeStyle = useMemo(() => getThemeStyle(mode, accent), [mode, accent]);
  const isDark = mode === "dark";

  const iconClass = "text-[rgb(var(--text-muted))]";
  const interactiveSurface = "hover:bg-[rgb(var(--panel-hover))]";
  const accentButton = cls(
    "border rounded-md shadow-sm transition",
    "bg-[rgb(var(--accent))] text-white border-[rgb(var(--accent-border))]",
    "hover:bg-[rgb(var(--accent-hover))]"
  );
  const secondaryButton = cls(
    "border rounded-md shadow-sm transition",
    "bg-[rgb(var(--panel-alt))] text-[rgb(var(--text))] border-[rgb(var(--border))]",
    "hover:bg-[rgb(var(--panel-hover))]"
  );

  return (
    <div
      style={themeStyle}
      data-theme={mode}
      data-accent={accent}
      className="h-screen w-full grid grid-cols-[18rem_1fr] grid-rows-[auto_1fr_auto] font-medium bg-[rgb(var(--bg))] text-[rgb(var(--text))]"
    >
      <div className="row-span-3 border-r border-[rgb(var(--border))] bg-[rgb(var(--panel))] flex flex-col min-h-0">
        <div className="flex-1 border-b border-[rgb(var(--border))] flex flex-col min-h-0">
          <div className="p-3 border-b border-[rgb(var(--border))] shrink-0 bg-[rgb(var(--panel))] text-[rgb(var(--text))] font-semibold">
            Filters
          </div>
          <div className="p-3 grid grid-cols-2 gap-2 overflow-y-auto min-h-0 flex-1">
            {filters.map((f, i) => (
              <button
                key={i}
                type="button"
                className="w-full h-9 text-xs px-2 py-2 rounded-md text-left truncate border border-[rgb(var(--border))] bg-[rgb(var(--panel-alt))] text-[rgb(var(--text))] hover:bg-[rgb(var(--panel-hover))]"
              >
                {f.name} ({f.count})
              </button>
            ))}
          </div>
          <div className="p-3 border-t border-[rgb(var(--border))] shrink-0">
            <button type="button" className={cls("w-full h-9 text-sm flex items-center justify-center gap-2", accentButton)}>
              <Plus size={16} strokeWidth={2} />
              <span>New</span>
            </button>
          </div>
        </div>

        <div className="shrink-0 border-b border-[rgb(var(--border))] flex flex-col">
          <div className="p-3 border-b border-[rgb(var(--border))] bg-[rgb(var(--panel))] font-semibold">Metadata</div>

          <div className="p-3 space-y-2 text-sm">
            <div>
              <div className="text-[11px] mb-1 text-[rgb(var(--text-muted))]">Song name</div>
              <input
                className="w-full border rounded px-2 py-1 h-8 bg-[rgb(var(--input))] border-[rgb(var(--input-border))] text-[rgb(var(--text))]"
                value={currentSong.title}
                readOnly
              />
            </div>

            <div>
              <div className="text-[11px] mb-1 text-[rgb(var(--text-muted))]">Artist name</div>
              <input
                className="w-full border rounded px-2 py-1 h-8 bg-[rgb(var(--input))] border-[rgb(var(--input-border))] text-[rgb(var(--text))]"
                value={currentSong.artist}
                readOnly
              />
            </div>

            <div>
              <div className="text-[11px] mb-1 text-[rgb(var(--text-muted))]">Release year</div>
              <input
                className="w-full border rounded px-2 py-1 h-8 bg-[rgb(var(--input))] border-[rgb(var(--input-border))] text-[rgb(var(--text))]"
                value={currentSong.year}
                readOnly
              />
            </div>

            <div>
              <div className="text-[11px] mb-1 text-[rgb(var(--text-muted))]">Album</div>
              <input
                className="w-full border rounded px-2 py-1 h-8 bg-[rgb(var(--input))] border-[rgb(var(--input-border))] text-[rgb(var(--text))]"
                value={currentSong.album}
                readOnly
              />
            </div>
          </div>

          <div className="p-3 border-t border-[rgb(var(--border))] flex gap-2">
            <button type="button" className={cls("flex-1 px-3 py-1.5 text-sm", secondaryButton)}>
              Reset
            </button>
            <button type="button" className={cls("flex-1 px-3 py-1.5 text-sm", accentButton)}>
              Save
            </button>
          </div>
        </div>

        <div className="flex-1 min-h-0 border-t border-[rgb(var(--border))] flex flex-col">
          <div className="sticky top-0 p-3 border-b border-[rgb(var(--border))] shrink-0 bg-[rgb(var(--panel))] font-semibold">
            History
          </div>
          <div className="p-3 space-y-1 overflow-y-auto min-h-0 flex-1">
            {history.map((h, i) => (
              <div key={i} className="text-sm cursor-pointer truncate text-[rgb(var(--text-muted))] hover:text-[rgb(var(--text))]">
                {h}
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="col-start-2 flex items-center gap-2 px-3 py-1.5 border-b border-[rgb(var(--border))] bg-[rgb(var(--panel))]">
        <button type="button" className={cls("p-1.5 rounded-md transition", interactiveSurface)}>
          <Settings2 size={16} strokeWidth={1.75} className={iconClass} />
        </button>
        <input
          placeholder="Search songs..."
          className="flex-1 border rounded-md px-2 py-0.5 text-sm bg-[rgb(var(--input))] border-[rgb(var(--input-border))] text-[rgb(var(--text))] placeholder:text-[rgb(var(--text-muted))] focus:outline-none focus:ring-2 focus:ring-[rgb(var(--accent-ring))]"
        />
        <div className="flex items-center gap-2">
          <label htmlFor="accent-select" className="sr-only">
            Accent color
          </label>
          <select
            id="accent-select"
            value={accent}
            onChange={(e) => setAccent(e.target.value)}
            className="h-7 rounded-md border border-[rgb(var(--input-border))] bg-[rgb(var(--input))] px-2 text-xs text-[rgb(var(--text))] focus:outline-none focus:ring-2 focus:ring-[rgb(var(--accent-ring))]"
            aria-label="Select accent color"
          >
            {Object.keys(ACCENTS).map((name) => (
              <option key={name} value={name}>
                {name.charAt(0).toUpperCase() + name.slice(1)}
              </option>
            ))}
          </select>

          <button
            type="button"
            onClick={() => setMode((prev) => (prev === "dark" ? "light" : "dark"))}
            className={cls("p-1.5 rounded-md transition", interactiveSurface)}
            aria-label={isDark ? "Switch to light mode" : "Switch to dark mode"}
            title={isDark ? "Light mode" : "Dark mode"}
          >
            {isDark ? <Sun size={16} strokeWidth={1.75} className={iconClass} /> : <Moon size={16} strokeWidth={1.75} className={iconClass} />}
          </button>
        </div>
      </div>

      <div className="col-start-2 p-3 overflow-y-auto space-y-2 min-h-0 bg-[rgb(var(--bg))]">
        {songs.map((song) => {
          const isSelected = currentSong.id === song.id;

          return (
            <button
              key={song.id}
              type="button"
              onClick={() => setCurrentSong(song)}
              className={cls(
                "w-full text-left rounded-lg shadow-sm transition px-3 py-2 border",
                isSelected
                  ? "bg-[rgb(var(--panel-alt))] border-[rgb(var(--accent-border))]"
                  : "bg-[rgb(var(--panel))] border-[rgb(var(--border))] hover:bg-[rgb(var(--panel-alt))] hover:border-[rgb(var(--border-soft))]"
              )}
              style={isSelected ? { boxShadow: "var(--app-shadow)" } : undefined}
            >
              <div className="flex justify-between items-center gap-3">
                <div className="min-w-0">
                  <div className="text-sm truncate text-[rgb(var(--text))] font-semibold">{song.title}</div>
                  <div className="text-xs truncate text-[rgb(var(--text-muted))]">{song.artist}</div>
                </div>
                <div className="text-xs shrink-0 text-[rgb(var(--text-muted))]">{song.duration}</div>
              </div>

              <div className="flex gap-1.5 mt-1.5 flex-wrap items-center">
                {song.tags.map((tag, idx) => (
                  <span
                    key={`${song.id}-${idx}`}
                    className="text-[10px] px-2 py-0.5 rounded-full border bg-[rgb(var(--panel-alt))] border-[rgb(var(--border))] text-[rgb(var(--text-muted))]"
                  >
                    {tag}
                  </span>
                ))}
                <span
                  role="button"
                  tabIndex={0}
                  onClick={(e) => e.stopPropagation()}
                  className="h-5 w-5 rounded-full border inline-flex items-center justify-center border-[rgb(var(--border-soft))] bg-[rgb(var(--panel))] hover:bg-[rgb(var(--panel-alt))]"
                >
                  <Plus size={12} strokeWidth={2} className={iconClass} />
                </span>
              </div>
            </button>
          );
        })}
      </div>

      <div className="col-start-2 border-t border-[rgb(var(--border))] px-3 py-2 flex flex-col gap-1 items-center bg-[rgb(var(--panel))]">
        <div className="w-full max-w-2xl">
          <div className="text-sm text-[rgb(var(--text))] font-semibold">
            {currentSong.title} - {currentSong.artist}
          </div>
          <input
            type="range"
            min="0"
            max="100"
            className="w-full mt-1"
            style={{ accentColor: "rgb(var(--accent))" }}
          />
          <div className="flex justify-between text-xs text-[rgb(var(--text-muted))]">
            <span>1:12</span>
            <span>{currentSong.duration}</span>
          </div>
        </div>

        <div className="w-full max-w-2xl grid grid-cols-3 items-center">
          <div />

          <div className="flex justify-center gap-3">
            <button type="button" className={cls("p-2 rounded-full transition", interactiveSurface)}>
              <Shuffle size={20} strokeWidth={1.75} className={iconClass} />
            </button>
            <button type="button" className={cls("p-2 rounded-full transition", interactiveSurface)}>
              <SkipBack size={20} strokeWidth={1.75} className={iconClass} />
            </button>

            <button type="button" className={cls("p-3 rounded-full transition text-white shadow", accentButton)}>
              <Play size={20} strokeWidth={2} />
            </button>

            <button type="button" className={cls("p-2 rounded-full transition", interactiveSurface)}>
              <SkipForward size={20} strokeWidth={1.75} className={iconClass} />
            </button>
            <button type="button" className={cls("p-2 rounded-full transition", interactiveSurface)}>
              <Repeat2 size={20} strokeWidth={1.75} className={iconClass} />
            </button>
          </div>

          <div className="flex justify-end items-center gap-2">
            <Volume2 size={18} strokeWidth={1.75} className={iconClass} />
            <input
              type="range"
              min="0"
              max="100"
              className="w-24"
              style={{ accentColor: "rgb(var(--accent))" }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
