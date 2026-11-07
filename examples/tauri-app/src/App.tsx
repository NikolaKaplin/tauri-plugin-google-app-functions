import { memo, useEffect, useRef, useState } from "react";
import { Badge, Card, Flex, Heading, IconButton, Slider, Text, TextField, Theme } from "@radix-ui/themes";
import { DiscIcon, MagnifyingGlassIcon, PauseIcon, PlayIcon, TrackNextIcon, TrackPreviousIcon, SpeakerLoudIcon, SpeakerQuietIcon, LoopIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { connectPlayer, send, tracks, usePlayer, type Playback } from "./music";

type Track = (typeof tracks)[number];
const AlbumArt = memo(function AlbumArt({ trackId, className = "" }: { trackId: number; className?: string }) {
  const track = tracks[trackId] ?? tracks[0];
  return <img className={`album-art ${className}`} src={track.cover} alt={`${track.title} album cover`}
    width="600" height="600" decoding="async" draggable={false} />;
});

const AlbumCard = memo(function AlbumCard({ track }: { track: Track }) {
  return <article className="album-card">
    <button className="album-cover-button" aria-label={`Play ${track.title}`} onClick={() => send(`track:${track.id}`)}>
      <AlbumArt trackId={track.id} />
      <span className="cover-play"><PlayIcon width="20" height="20" /></span>
    </button>
    <Text as="div" size="2" weight="medium" mt="3">{track.title}</Text>
    <Text as="div" size="1" color="gray" mt="1">{track.artist} — {track.album}</Text>
  </article>;
});

const TrackRow = memo(function TrackRow({ track, active = false, playing = false, compact = false }: {
  track: Track; active?: boolean; playing?: boolean; compact?: boolean;
}) {
  return <button className={`track-row${active ? " active" : ""}${compact ? " compact" : ""}`}
    aria-label={`Play ${track.title}`} aria-current={active ? "true" : undefined} onClick={() => send(`track:${track.id}`)}>
    <AlbumArt trackId={track.id} className="track-art" />
    <span className="track-text"><Text as="span" size="2" weight={active ? "medium" : "regular"}>{track.title}</Text>
      <Text as="span" size="1" color="gray">{track.artist} — {track.album}</Text></span>
    {active && playing ? <SpeakerLoudIcon className="playing-mark" /> : <Text size="1" color="gray" className="track-duration">0:16</Text>}
  </button>;
});

const Library = memo(function Library() {
  const [query, setQuery] = useState("");
  const visible = tracks.filter((track) => `${track.title} ${track.artist} ${track.album}`.toLowerCase().includes(query.toLowerCase()));
  return <section className="library" aria-label="Music library">
    <header className="library-toolbar">
      <div className="window-dots" aria-hidden="true"><i /><i /><i /></div>
      <TextField.Root placeholder="Search your library" value={query} onChange={(event) => setQuery(event.target.value)} size="2" radius="full" aria-label="Search your library">
        <TextField.Slot><MagnifyingGlassIcon /></TextField.Slot>
      </TextField.Root>
    </header>
    <div className="library-content">
      <Flex align="center" justify="between" mb="4"><Heading size="5">Your favorites</Heading><Badge variant="soft" color="gray">2 albums</Badge></Flex>
      <div className="album-grid">{visible.map((track) => <AlbumCard key={track.id} track={track} />)}</div>
      {visible.length === 0 && <Text as="p" color="gray" size="2">No tracks match “{query}”.</Text>}
      <Flex align="center" justify="between" mt="6" mb="3"><Heading size="5">Made for you</Heading><Text size="1" color="gray">Demo sessions</Text></Flex>
      <div className="session-grid">{visible.map((track) => <button className={`session-card session-${track.id}`} key={track.id} onClick={() => send(`track:${track.id}`)}>
        <AlbumArt trackId={track.id} className="session-art" />
        <span><Text as="span" weight="medium" size="2">{track.album}</Text><Text as="span" size="1">{track.mood}</Text></span>
        <ArrowRightIcon />
      </button>)}</div>
      <Heading size="5" mt="6" mb="3">All tracks</Heading>
      <div className="library-tracks">{visible.map((track) => <TrackRow key={track.id} track={track} />)}</div>
      <Flex gap="2" align="center" className="library-note"><LoopIcon /><Text color="gray" size="1">Two original loops. Yours to play, even offline.</Text></Flex>
    </div>
  </section>;
});

function Queue() {
  const { playback } = usePlayer();
  return <Card className="queue-panel" size="3">
    <Flex align="center" justify="between" mb="4"><Heading size="3">Queue</Heading><Text size="1" color="red">Repeat all <LoopIcon /></Text></Flex>
    {tracks.map((track) => <TrackRow key={track.id} track={track} compact active={playback.trackId === track.id} playing={playback.trackId === track.id && playback.playing} />)}
  </Card>;
}

function Sound() {
  const { playback } = usePlayer();
  const [draft, setDraft] = useState(playback.volume);
  useEffect(() => { setDraft(playback.volume); }, [playback.volume]);
  return <Card className="sound-panel" size="3">
    <Flex justify="between" align="center" mb="5"><Heading size="3">Sound</Heading><Text size="1" color="gray">This device</Text></Flex>
    <Flex align="center" gap="3"><SpeakerQuietIcon /><Slider aria-label="Playback volume" value={[draft]} onValueChange={([value]) => setDraft(value)} onValueCommit={([value]) => send(`volume:${value}`)} /><SpeakerLoudIcon /></Flex>
    <Flex justify="between" mt="4"><Text size="1" color="gray">Player volume</Text><Text size="1" color="gray">{draft}%</Text></Flex>
  </Card>;
}

const ListeningNote = memo(function ListeningNote() {
  return <div className="listening-note"><DiscIcon width="24" height="24" /><Text as="p" size="2" weight="medium">A little less noise.</Text><Text as="p" size="1" color="gray">Morning light or late-night roads.<br />Pick a track and settle in.</Text></div>;
});

function time(milliseconds: number) {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

// Only this small component ticks. It never updates library cards or artwork.
function ProgressClock({ playback, receivedAt }: { playback: Playback; receivedAt: number }) {
  const [position, setPosition] = useState(playback.positionMs);
  const bar = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const tick = () => {
      const next = playback.playing ? (playback.positionMs + Date.now() - receivedAt) % playback.durationMs : playback.positionMs;
      setPosition(next);
      if (bar.current) bar.current.style.transform = `scaleX(${next / playback.durationMs})`;
    };
    tick();
    if (!playback.playing) return;
    const timer = window.setInterval(tick, 250);
    return () => clearInterval(timer);
  }, [playback, receivedAt]);
  return <div className="playback-progress"><span className="elapsed">{time(position)}</span><div className="progress-track" role="progressbar" aria-label="Track progress" aria-valuemin={0} aria-valuemax={playback.durationMs} aria-valuenow={Math.round(position)}><div ref={bar} /></div><span>{time(playback.durationMs)}</span></div>;
}

function PlayerBar() {
  const { playback, busy, error, receivedAt } = usePlayer();
  return <>
    {error && <div className="player-error" role="alert">{error}</div>}
    <section className="player-bar" aria-label="Now playing" aria-busy={busy}>
      <div className="player-controls">
        <IconButton variant="ghost" color="gray" aria-label="Previous track" onClick={() => send("previous")}><TrackPreviousIcon width="18" height="18" /></IconButton>
        <IconButton className="main-play" radius="full" size="3" aria-label={playback.playing ? "Pause" : "Play"} onClick={() => send(playback.playing ? "pause" : "play")}>
          {playback.playing ? <PauseIcon width="20" height="20" /> : <PlayIcon width="20" height="20" />}
        </IconButton>
        <IconButton variant="ghost" color="gray" aria-label="Next track" onClick={() => send("next")}><TrackNextIcon width="18" height="18" /></IconButton>
      </div>
      <div className="current-track"><AlbumArt trackId={playback.trackId} className="player-art" /><div><Text as="div" size="2" weight="medium">{playback.title}</Text><Text as="div" size="1" color="gray">Small Hours — Demo sessions</Text></div></div>
      <ProgressClock playback={playback} receivedAt={receivedAt} />
      <div className="player-repeat" title="Both tracks repeat"><LoopIcon /><Text size="1" color="gray">Repeat</Text></div>
    </section>
  </>;
}

export default function App() {
  useEffect(connectPlayer, []);
  return <Theme appearance="light" accentColor="red" grayColor="mauve" radius="medium" scaling="95%">
    <div className="app-shell">
      <header className="app-header"><Flex gap="2" align="center"><DiscIcon width="23" height="23" /><Text size="4" weight="bold">Small Hours</Text></Flex><Badge variant="soft" color="gray">Your music, quietly.</Badge></header>
      <main className="workspace"><aside className="sidebar"><Queue /><Sound /><ListeningNote /></aside><Library /></main>
      <PlayerBar />
    </div>
  </Theme>;
}
