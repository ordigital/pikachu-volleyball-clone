# Original music provenance

`original-bgm.mids` is the untouched MIDS/BGM/1041 resource from the user's
original Pokemon_20.exe. `original-bgm.mid` is its lossless musical-event
conversion to standard MIDI format 0, 48 PPQN. The provenance JSON records hashes
of the executable and both outputs; the executable itself is not bundled.
The complete 4,541 events span 83.25 seconds at 400,000 microseconds per quarter.

The final `src/music_data.h` contains native YM2612/PSG register writes, preserving
notes and pitch bends with console instrument approximations. The public audio
test independently verifies original events and actual linked note pitches.
No pitch transcription or sampled BGM is used. Optional authoring tools and their
intermediate inputs are private under the Git-ignored `tools/` directory.

Original game artwork/music: SACHI SOFT / SAWAYAKAN Programmers and Satoshi
Takenouchi. The ball artwork was taken from gorisanson/pikachu-volleyball at
commit 6bc096ad9e25137ae27eb536a6eac1d056be7506; the final player/sitting/cloud
pixel artwork derives from this remake's HD assets. See the root README credits.
