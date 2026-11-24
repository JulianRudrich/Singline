"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { pickMedia, bestArtwork, readMetadata, createSettler, createActions, snapshot } = require("../content/page.js");

const element = (state) => ({ readyState: 4, currentTime: 0, paused: true, ended: false, muted: false, ...state });

test("the playing element wins over paused and empty ones", () => {
  const empty = element({ readyState: 0 });
  const paused = element({ currentTime: 40 });
  const playing = element({ currentTime: 3, paused: false });
  const muted = element({ currentTime: 1, paused: false, muted: true });
  assert.equal(pickMedia([empty, paused, playing, muted]), playing);
  assert.equal(pickMedia([empty, paused]), paused);
  assert.equal(pickMedia([empty]), null);
  assert.equal(pickMedia([]), null);
});

test("metadata comes from the Media Session, with the largest https picture", () => {
  const session = {
    metadata: {
      title: " Paper Lanterns ",
      artist: "Example Artist",
      album: "Example Album",
      artwork: [
        { src: "https://i.ytimg.com/small.jpg", sizes: "96x96" },
        { src: "http://insecure.example/large.jpg", sizes: "1024x1024" },
        { src: "https://i.ytimg.com/large.jpg", sizes: "512x512 256x256" },
      ],
    },
  };
  assert.deepEqual(readMetadata(session), {
    title: "Paper Lanterns",
    artist: "Example Artist",
    album: "Example Album",
    artwork: "https://i.ytimg.com/large.jpg",
  });
  assert.deepEqual(readMetadata({ metadata: null }), { title: "", artist: "", album: "", artwork: "" });
  assert.equal(bestArtwork(undefined), "");
});

test("a new song waits until its media has loaded", () => {
  const settler = createSettler();
  assert.equal(settler.ready("first", true, 0), true, "the first song shows at once");
  assert.equal(settler.ready("second", true, 10_000), false, "new metadata, old media");
  settler.loaded(10_400);
  assert.equal(settler.ready("second", true, 10_500), true);
  assert.equal(settler.ready("second", true, 10_600), true);

  assert.equal(settler.ready("third", false, 20_000), false);
  assert.equal(settler.ready("third", false, 22_000), false, "no duration yet");
  assert.equal(settler.ready("third", true, 21_600 + 2_000), true, "quiet long enough");
  assert.equal(settler.ready("fourth", false, 30_000), false);
  assert.equal(settler.ready("fourth", false, 35_000), true, "never waits forever");
});

test("a snapshot leaves out metadata while it settles or an ad plays", () => {
  const media = element({ currentTime: 12.5, duration: 200, paused: false, playbackRate: 1 });
  const metadata = { title: "Song", artist: "Artist", album: "", artwork: "" };
  const base = { media, metadata, settled: true, ad: false, controls: {}, location: "https://www.youtube.com/watch?v=1", pageTitle: "Song - YouTube", now: 5 };
  const state = snapshot(base);
  assert.equal(state.title, "Song");
  assert.equal(state.currentTime, 12.5);
  assert.equal(state.duration, 200);
  assert.equal(state.paused, false);
  assert.equal(state.sampledAt, 5);
  assert.equal(snapshot({ ...base, ad: true }).title, "");
  assert.equal(snapshot({ ...base, settled: false }).pageTitle, "");
  const live = snapshot({ ...base, media: element({ duration: Number.POSITIVE_INFINITY }) });
  assert.equal(live.duration, null, "a live stream has no duration");
  assert.equal(snapshot({ ...base, media: null }).paused, true);
});

test("the handlers a site gives the media keys skip songs", () => {
  const actions = createActions();
  const calls = [];
  assert.equal(actions.run("nexttrack"), false, "no handler yet");
  actions.set("nexttrack", (details) => calls.push(details.action));
  actions.set("previoustrack", () => {
    throw new Error("the site failed");
  });
  assert.equal(actions.has("nexttrack"), true);
  assert.equal(actions.run("nexttrack"), true);
  assert.deepEqual(calls, ["nexttrack"]);
  assert.equal(actions.run("previoustrack"), false, "a failing handler did not skip");

  actions.set("nexttrack", null);
  assert.equal(actions.has("nexttrack"), false, "the site took its handler back");
  assert.equal(actions.run("nexttrack"), false);
  actions.set("seekto", "not a function");
  assert.equal(actions.has("seekto"), false);
});
