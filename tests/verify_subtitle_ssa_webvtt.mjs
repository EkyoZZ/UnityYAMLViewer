// Reference codec/container checks. Does not execute the Rust application.
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import assert from 'node:assert/strict';

const root = resolve(process.argv[2] || 'target/subtitle-ssa-webvtt-verification');
mkdirSync(root, { recursive: true });
const ffmpeg = process.env.FFMPEG || 'ffmpeg';
const ffprobe = process.env.FFPROBE || 'ffprobe';
function run(exe, args) {
  const p = spawnSync(exe, args, { encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(p.error, undefined, p.error?.message);
  assert.equal(p.status, 0, p.stderr);
  return p.stdout;
}
const ssa = `[Script Info]
ScriptType: v4.00
PlayResX: 640
PlayResY: 360
[V4 Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, TertiaryColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, AlphaLevel, Encoding
Style: Legacy,Arial,28,16777215,255,65280,0,-1,0,1,2,1,6,10,20,30,0,1
[Events]
Format: Marked, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: Marked=1,0:00:00.12,0:00:01.98,Legacy,actor,11,22,33,,{\i1}SSA original, kept
`;
const vtt = `WEBVTT original

STYLE
::cue(.red) { color: red; }

REGION
id:one
width:40%

NOTE original metadata
retained by native document editing

cue-one
00:00.123 --> 00:01.987 line:20%,start position:10%,line-left align:start
<v Alice><i>WebVTT original</i></v>
second line
`;
const srt = '1\n00:00:00,123 --> 00:00:01,987\nSRT original\n';
writeFileSync(join(root, 'source.ssa'), ssa, 'utf8');
writeFileSync(join(root, 'source.vtt'), vtt, 'utf8');
writeFileSync(join(root, 'source.srt'), srt, 'utf8');
const media = join(root, 'three-tracks.mkv');
run(ffmpeg, ['-nostdin', '-v', 'error', '-y', '-f', 'lavfi', '-i', 'color=c=black:s=640x360:r=25:d=2', '-i', join(root, 'source.ssa'), '-i', join(root, 'source.srt'), '-i', join(root, 'source.vtt'),
  '-map', '0:v:0', '-map', '1:s:0', '-map', '2:s:0', '-map', '3:s:0', '-c:v', 'ffv1', '-c:s', 'copy', media]);
const formats = [['ass', 'ass'], ['srt', 'srt'], ['vtt', 'webvtt']];
const extracted = [];
for (let n = 0; n < formats.length; n++) {
  const [extension, muxer] = formats[n];
  const text = run(ffmpeg, ['-nostdin', '-v', 'error', '-i', media, '-map', `0:s:${n}`, '-c:s', 'copy', '-f', muxer, 'pipe:1']);
  extracted.push(text);
  writeFileSync(join(root, `native-${n}.${extension}`), text, 'utf8');
}
assert.ok(extracted[0].includes('[V4 Styles]'));
assert.ok(extracted[0].includes('Style: Legacy,Arial,28'));
assert.ok(extracted[0].includes('{\i1}SSA original, kept'));
assert.ok(extracted[1].includes('00:00:00,123 --> 00:00:01,987'));
assert.ok(extracted[1].includes('SRT original'));
assert.ok(extracted[2].startsWith('WEBVTT'));
assert.ok(extracted[2].includes('cue-one'));
assert.ok(extracted[2].includes('line:20%,start position:10%,line-left align:start'));
assert.ok(extracted[2].includes('<v Alice><i>WebVTT original</i></v>'));
const streams = JSON.parse(run(ffprobe, ['-v', 'error', '-select_streams', 's', '-show_entries', 'stream=codec_name', '-of', 'json', media])).streams.map(s => s.codec_name);
assert.deepEqual(streams, ['ass', 'subrip', 'webvtt']);
const filter = `subtitles=filename='${join(root, 'source.ssa').replaceAll('\\', '/').replaceAll(':', '\\:')}',select='gte(t,0.4)'`;
const frame = spawnSync(ffmpeg, ['-nostdin', '-v', 'error', '-i', media, '-vf', filter, '-map', '0:v:0', '-frames:v', '1', '-pix_fmt', 'rgb24', '-f', 'rawvideo', 'pipe:1'], { windowsHide: true, maxBuffer: 4 * 1024 * 1024 });
assert.equal(frame.status, 0, frame.stderr?.toString());
let minY = 360, maxY = -1;
for (let n = 0; n < frame.stdout.length; n += 3) {
  if (Math.max(...frame.stdout.subarray(n, n + 3)) > 32) {
    const y = Math.floor(n / 3 / 640); minY = Math.min(minY, y); maxY = Math.max(maxY, y);
  }
}
assert.ok(maxY >= 0 && maxY < 150, 'SSA alignment 6 must render near the top, not ASS alignment 6 at mid-right');
const result = { passed: true, streams, ssaTextBounds: { minY, maxY }, verified: ['native SSA header/style survives copy extraction', 'native SRT milliseconds preserved', 'native WebVTT cue id/settings/voice tags preserved in MKV roundtrip', 'SSA legacy alignment 6 renders at top'],
  limitations: ['Rust model/editor unit cases compiled but not executed', 'full application GUI not run', 'WebVTT global STYLE/REGION/NOTE may be absent from the embedded stream; extraction cannot reconstruct upstream-discarded data'] };
writeFileSync(join(root, 'result.json'), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
