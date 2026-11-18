// Web Audio plugin for the browser build (miniquad plugin API, see src/audio_web.rs).
// Ids 0-6: sound effects (WAV), 7: menu music, 8: game music (Ogg Vorbis). Decoding is done by the
// browser (decodeAudioData); music is one looping source plus a gain node.
"use strict";
(function () {
    var ctx = null;
    var buffers = {};
    var musicGain = null;
    var music = null;        // { id, source, startedAt, offset, paused }
    var wanted = null;       // music requested before its buffer was decoded: { id, gain }

    function ensure() {
        if (ctx) return ctx;
        var Ctx = window.AudioContext || window.webkitAudioContext;
        if (!Ctx) return null;
        ctx = new Ctx();
        musicGain = ctx.createGain();
        musicGain.connect(ctx.destination);
        return ctx;
    }

    function startMusic(id, offset) {
        var buffer = buffers[id];
        if (!buffer || !ctx) return false;
        var source = ctx.createBufferSource();
        source.buffer = buffer;
        source.loop = true;
        source.connect(musicGain);
        source.start(0, offset % buffer.duration);
        music = { id: id, source: source, startedAt: ctx.currentTime - offset, offset: offset, paused: false };
        return true;
    }

    function stopSource() {
        if (music && music.source) {
            try { music.source.stop(); } catch (e) { /* already stopped */ }
            music.source.disconnect();
            music.source = null;
        }
    }

    window.pika_audio_unlock = function () {
        if (ensure() && ctx.state !== "running") ctx.resume();
    };
    window.pika_audio_state = function () {
        return ctx ? ctx.state + (music ? ", music " + music.id + (music.paused ? " paused" : " playing") : ", no music") : "no context";
    };

    miniquad_add_plugin({
        name: "pika_audio",
        version: 1,
        register_plugin: function (importObject) {
            var env = importObject.env;
            env.pika_audio_load = function (id, ptr, len) {
                if (!ensure()) return;
                var bytes = new Uint8Array(wasm_memory.buffer, ptr, len).slice().buffer;
                ctx.decodeAudioData(bytes).then(function (buffer) {
                    buffers[id] = buffer;
                    if (location.search.indexOf("shot") >= 0) {
                        console.log("pika_audio: sound " + id + " decoded: " + buffer.duration.toFixed(2) + " s, " + buffer.numberOfChannels + " ch, " + buffer.sampleRate + " Hz");
                    }
                    if (wanted && wanted.id === id) {
                        musicGain.gain.value = wanted.gain;
                        startMusic(id, 0);
                        wanted = null;
                    }
                }, function (err) { console.error("pika_audio: cannot decode sound " + id, err); });
            };
            env.pika_audio_sfx = function (id, pan, gain) {
                if (!ctx || !buffers[id]) return;
                var source = ctx.createBufferSource();
                source.buffer = buffers[id];
                var node = source;
                if (ctx.createStereoPanner) {
                    var panner = ctx.createStereoPanner();
                    panner.pan.value = pan;
                    node.connect(panner);
                    node = panner;
                }
                var g = ctx.createGain();
                g.gain.value = gain;
                node.connect(g);
                g.connect(ctx.destination);
                source.start();
            };
            env.pika_audio_music = function (id, gain) {
                if (!ensure()) return;
                stopSource();
                music = null;
                wanted = null;
                musicGain.gain.value = gain;
                if (!startMusic(id, 0)) wanted = { id: id, gain: gain };
            };
            env.pika_audio_music_stop = function () {
                stopSource();
                music = null;
                wanted = null;
            };
            env.pika_audio_music_pause = function (paused) {
                if (!music) return;
                if (paused && !music.paused) {
                    music.offset = (ctx.currentTime - music.startedAt) % buffers[music.id].duration;
                    stopSource();
                    music.paused = true;
                } else if (!paused && music.paused) {
                    startMusic(music.id, music.offset);
                }
            };
            env.pika_audio_music_gain = function (gain) {
                if (musicGain) musicGain.gain.value = gain;
                if (wanted) wanted.gain = gain;
            };
            env.pika_open_url = function (ptr, len) {
                var url = new TextDecoder("utf-8").decode(new Uint8Array(wasm_memory.buffer, ptr, len));
                window.open(url, "_blank", "noopener");
            };
        },
        on_init: function () { ensure(); }
    });
})();
