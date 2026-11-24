// Runs in the page's own world, because only there are the media elements
// visible that a player never adds to the document. Reads the element that
// plays and the Media Session metadata, reports both to relay.js, and carries
// out the commands that come back. Nothing here talks to Singline directly.
(function () {
  "use strict";

  const CHANNEL = "singline-v1";
  /** How often to report while playing; paused tabs report less often. */
  const PLAYING_INTERVAL = 1000;
  const PAUSED_INTERVAL = 2500;

  /**
   * The site's own skip buttons; the media element cannot skip by itself.
   * Other sites skip through the handlers they give the media keys.
   */
  const BUTTONS = {
    "music.youtube.com": {
      next: "ytmusic-player-bar .next-button",
      previous: "ytmusic-player-bar .previous-button",
    },
    "www.youtube.com": {
      next: ".ytp-next-button",
      previous: ".ytp-prev-button",
    },
    "open.spotify.com": {
      next: '[data-testid="control-button-skip-forward"]',
      previous: '[data-testid="control-button-skip-back"]',
    },
  };

  function finite(value) {
    return typeof value === "number" && Number.isFinite(value) ? value : null;
  }

  function text(value) {
    return typeof value === "string" ? value.trim() : "";
  }

  /** How likely an element is the one the listener hears: playing beats paused. */
  function mediaScore(element) {
    if (!element) return 0;
    const started = element.readyState >= 1 || finite(element.currentTime) > 0;
    if (!started) return 0;
    if (!element.paused && !element.ended) return element.muted ? 3 : 4;
    if (finite(element.currentTime) > 0 && !element.ended) return 2;
    return 1;
  }

  /** The element that plays, or else the one that played last. */
  function pickMedia(elements) {
    let best = null;
    let bestScore = 0;
    for (const element of elements) {
      const score = mediaScore(element);
      if (score >= bestScore && score > 0) {
        best = element;
        bestScore = score;
      }
    }
    return best;
  }

  /** The largest https picture of a Media Session artwork list. */
  function bestArtwork(artwork) {
    let best = "";
    let bestSize = -1;
    for (const image of Array.isArray(artwork) ? artwork : []) {
      const src = text(image && image.src);
      if (!src.startsWith("https://")) continue;
      const size = Math.max(
        0,
        ...text(image.sizes)
          .split(/\s+/)
          .map((entry) => Number.parseInt(entry.split("x")[0], 10) || 0),
      );
      if (size > bestSize) {
        best = src;
        bestSize = size;
      }
    }
    return best;
  }

  function readMetadata(session) {
    const metadata = session && session.metadata;
    if (!metadata) return { title: "", artist: "", album: "", artwork: "" };
    return {
      title: text(metadata.title),
      artist: text(metadata.artist),
      album: text(metadata.album),
      artwork: bestArtwork(metadata.artwork),
    };
  }

  /**
   * Holds back new metadata until the new media has loaded, so a song is
   * never reported with the duration or position of the one before.
   */
  function createSettler({ quiet = 1500, maxWait = 5000 } = {}) {
    let shownKey = null;
    let pendingKey = null;
    let changedAt = 0;
    let loadedAt = -Infinity;
    return {
      loaded(now) {
        loadedAt = now;
      },
      ready(key, durationKnown, now) {
        if (key === shownKey) return true;
        if (shownKey === null && durationKnown) {
          shownKey = key;
          return true;
        }
        if (key !== pendingKey) {
          pendingKey = key;
          changedAt = now;
        }
        const waited = now - changedAt;
        const loadedForIt = loadedAt >= changedAt - 3000;
        const ok = (durationKnown && (loadedForIt || waited >= quiet)) || waited >= maxWait;
        if (ok) {
          shownKey = key;
          pendingKey = null;
        }
        return ok;
      },
    };
  }

  /** The Media Session actions of the media keys that skip songs. */
  const SKIP_ACTIONS = { next: "nexttrack", previous: "previoustrack" };

  /**
   * Keeps the handlers a site gives the Media Session, which the browser calls
   * for the media keys: with them a site skips songs without Singline knowing
   * its buttons.
   */
  function createActions() {
    const handlers = new Map();
    return {
      set(action, handler) {
        if (typeof handler === "function") handlers.set(action, handler);
        else handlers.delete(action);
      },
      has(action) {
        return handlers.has(action);
      },
      /** Calls the site's handler, and says whether there was one that ran. */
      run(action) {
        const handler = handlers.get(action);
        if (!handler) return false;
        try {
          handler({ action });
          return true;
        } catch {
          return false;
        }
      },
    };
  }

  /** What the tab plays right now, in the shape the background expects. */
  function snapshot({ media, metadata, settled, ad, controls, location, pageTitle, now }) {
    const duration = media ? finite(media.duration) : null;
    const known = settled && !ad;
    return {
      url: location,
      pageTitle: known ? pageTitle : "",
      title: known ? metadata.title : "",
      artist: known ? metadata.artist : "",
      album: known ? metadata.album : "",
      artwork: known ? metadata.artwork : "",
      currentTime: media ? finite(media.currentTime) : null,
      duration: duration !== null && duration > 0 ? duration : null,
      playbackRate: media ? finite(media.playbackRate) : null,
      paused: media ? Boolean(media.paused) : true,
      ended: media ? Boolean(media.ended) : false,
      sampledAt: now,
      controls,
    };
  }

  const exported = { pickMedia, mediaScore, bestArtwork, readMetadata, createSettler, createActions, snapshot };
  // In Node, for the tests, only the functions above are wanted.
  if (typeof module === "object" && typeof process === "object" && process.versions?.node) {
    module.exports = exported;
    return;
  }

  // Everything below runs in the browser only.
  const known = [];
  const settler = createSettler();
  const actions = createActions();
  let lastSent = 0;
  let lastReport = "";

  const EVENTS = ["play", "pause", "seeked", "ratechange", "durationchange", "loadedmetadata", "emptied", "ended"];

  function remember(element) {
    if (!(element instanceof HTMLMediaElement) || known.includes(element)) return;
    known.push(element);
    if (known.length > 8) known.shift();
    for (const name of EVENTS) {
      element.addEventListener(name, (event) => {
        if (event.type === "loadedmetadata" || event.type === "durationchange" || event.type === "emptied") {
          settler.loaded(Date.now());
        }
        report(true);
      });
    }
    element.addEventListener("timeupdate", () => report(false));
  }

  // Elements in the document announce themselves when they play...
  document.addEventListener("play", (event) => remember(event.target), true);
  // ...and some players play an element they never add to the document.
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function (...args) {
    remember(this);
    return play.apply(this, args);
  };

  // The handlers for the media keys, as the site sets them. A handler the
  // browser refuses is not kept.
  if (typeof MediaSession === "function") {
    const setActionHandler = MediaSession.prototype.setActionHandler;
    MediaSession.prototype.setActionHandler = function (action, handler) {
      const result = setActionHandler.call(this, action, handler);
      actions.set(action, handler);
      return result;
    };
  }

  function currentMedia() {
    for (const element of document.querySelectorAll("video, audio")) remember(element);
    return pickMedia(known);
  }

  function button(kind) {
    const selector = BUTTONS[location.hostname] && BUTTONS[location.hostname][kind];
    const element = selector ? document.querySelector(selector) : null;
    if (!element || element.disabled || element.getAttribute("aria-disabled") === "true") return null;
    const box = element.getBoundingClientRect();
    return box.width > 0 && box.height > 0 ? element : null;
  }

  /** Whether the site can skip that way, with its button or its media keys. */
  function canSkip(kind) {
    return button(kind) !== null || actions.has(SKIP_ACTIONS[kind]);
  }

  /** Skips as the site's button does, or else as its media keys do. */
  function skip(kind) {
    const site = button(kind);
    if (site) site.click();
    else actions.run(SKIP_ACTIONS[kind]);
  }

  function adShowing() {
    return document.querySelector("#movie_player.ad-showing, .html5-video-player.ad-showing") !== null;
  }

  function report(urgent) {
    const now = Date.now();
    const media = currentMedia();
    if (!media) return;
    const playing = !media.paused && !media.ended;
    if (!urgent && now - lastSent < (playing ? PLAYING_INTERVAL : PAUSED_INTERVAL)) return;
    const metadata = readMetadata(navigator.mediaSession);
    const key = [metadata.title, metadata.artist, metadata.album].join("\u001f");
    const duration = finite(media.duration);
    const settled = settler.ready(key, duration !== null && duration > 0, now);
    const state = snapshot({
      media,
      metadata,
      settled,
      ad: adShowing(),
      controls: {
        playPause: true,
        seek: duration !== null && duration > 0,
        next: canSkip("next"),
        previous: canSkip("previous"),
      },
      location: location.href,
      pageTitle: document.title,
      now,
    });
    // An urgent report that changes nothing is not worth a message.
    const signature = JSON.stringify({ ...state, currentTime: null, sampledAt: null });
    if (urgent && signature === lastReport && now - lastSent < 250) return;
    lastReport = signature;
    lastSent = now;
    window.postMessage({ channel: CHANNEL, from: "page", type: "state", state }, location.origin);
  }

  function run(command) {
    const media = currentMedia();
    if (!media || !command || typeof command.kind !== "string") return;
    switch (command.kind) {
      case "playPause":
        if (media.paused) media.play().catch(() => {});
        else media.pause();
        break;
      case "pause":
        media.pause();
        break;
      case "next":
      case "previous":
        skip(command.kind);
        break;
      case "seekTo": {
        const position = finite(command.position);
        const duration = finite(media.duration);
        if (position !== null && duration !== null) media.currentTime = Math.min(Math.max(position, 0), duration);
        break;
      }
      default:
        return;
    }
    setTimeout(() => report(true), 150);
  }

  window.addEventListener("message", (event) => {
    const data = event.data;
    if (event.source !== window || !data || data.channel !== CHANNEL || data.from !== "relay") return;
    if (data.type === "command") run(data.command);
  });

  setInterval(() => report(false), PLAYING_INTERVAL);
})();
