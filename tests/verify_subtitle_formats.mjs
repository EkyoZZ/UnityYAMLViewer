// FFmpeg integration evidence; does not execute the Rust application or its unit tests.
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import assert from 'node:assert/strict';

const root = resolve(process.argv[2] || 'target/subtitle-editor-verification');
mkdirSync(root, { recursive: true });
const ffmpeg = process.env.FFMPEG || 'ffmpeg';
const ffprobe = process.env.FFPROBE || 'ffprobe';
function run(executable, args, input) {
  const result = spawnSync(executable, args, { input, encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 });
  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
}
const source = `[Script Info]
ScriptType: v4.00+
PlayResX: 640
PlayResY: 360
WrapStyle: 2
ScaledBorderAndShadow: yes
[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Sign,Arial,28,&H00123456,&H000000FF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,2,1,7,10,20,30,1
[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 3,0:00:00.12,0:00:01.85,Sign,actor,11,22,33,,{\\move(10,20,100,50)\\fad(50,80)\\k20}ASS, original
Dialogue: 4,0:00:00.50,0:00:01.00,Sign,,0,0,0,,{\\an7\\pos(10,20)\\p1}m 0 0 l 20 0 20 20 0 20{\\p0}
`.replaceAll('\\\\', '\\');
const ass = join(root, 'original.ass');
const srt = join(root, 'original.srt');
const media = join(root, 'two-tracks.mkv');
writeFileSync(ass, source, 'utf8');
writeFileSync(srt, '1\n00:00:00,123 --> 00:00:01,987\n<i>SRT original</i>\nsecond line\n', 'utf8');
run(ffmpeg, ['-nostdin', '-v', 'error', '-y', '-f', 'lavfi', '-i', 'color=c=black:s=640x360:r=25:d=2',
  '-i', ass, '-i', srt, '-map', '0:v:0', '-map', '1:s:0', '-map', '2:s:0', '-c:v', 'ffv1', '-c:s:0', 'copy', '-c:s:1', 'srt', media]);
function extract(ordinal) {
  const codec = run(ffprobe, ['-v', 'error', '-select_streams', `s:${ordinal}`, '-show_entries', 'stream=codec_name', '-of', 'default=nw=1:nk=1', media]).trim();
  const extracted = run(ffmpeg, ['-nostdin', '-v', 'error', '-i', media, '-map', `0:s:${ordinal}`, '-c:s', ['ass', 'ssa'].includes(codec) ? 'copy' : 'ass', '-f', 'ass', 'pipe:1']);
  writeFileSync(join(root, `track-${ordinal}.ass`), extracted, 'utf8');
  return { codec, extracted };
}
const first = extract(0);
assert.equal(first.codec, 'ass');
for (const value of ['PlayResX: 640', 'PlayResY: 360', 'Style: Sign,Arial,28,&H00123456', '3,0:00:00.12,0:00:01.85,Sign,actor,11,22,33,',
  '{\\move(10,20,100,50)\\fad(50,80)\\k20}ASS, original', '{\\an7\\pos(10,20)\\p1}m 0 0 l 20 0 20 20 0 20{\\p0}']) {
  assert.ok(first.extracted.includes(value), `Original ASS field/tag missing: ${value}`);
}
const second = extract(1);
assert.equal(second.codec, 'subrip');
assert.ok(second.extracted.includes('SRT original'));
assert.ok(second.extracted.includes('second line'));
assert.ok(!second.extracted.includes('ASS, original'));
const converted = run(ffmpeg, ['-nostdin', '-v', 'error', '-f', 'ass', '-i', 'pipe:0', '-map', '0:s:0', '-c:s', 'srt', '-f', 'srt', 'pipe:1'], first.extracted);
assert.ok(converted.includes('ASS, original'));
assert.ok(!converted.includes('\\move'));
writeFileSync(join(root, 'converted.srt'), converted, 'utf8');
// The baseline codec exposes vector commands as SRT text. The editor filters
// drawing spans/events first; mixed spans are covered by its Rust unit cases.
assert.ok(converted.includes('m 0 0'));
const textOnlyAss = first.extracted.split(/\r?\n/).filter(line => !line.startsWith('Dialogue: 4,')).join('\n');
const cleanSrt = run(ffmpeg, ['-nostdin', '-v', 'error', '-f', 'ass', '-i', 'pipe:0', '-map', '0:s:0', '-c:s', 'srt', '-f', 'srt', 'pipe:1'], textOnlyAss).replace(/\{\\an[1-9]\}/g, '');
assert.ok(!cleanSrt.includes('m 0 0'));
assert.ok(!cleanSrt.includes('\\an'));
writeFileSync(join(root, 'text-only.srt'), cleanSrt, 'utf8');
run(ffmpeg, ['-nostdin', '-v', 'error', '-y', '-i', media, '-vf', `subtitles=filename='${join(root, 'track-0.ass').replaceAll('\\', '/').replaceAll(':', '\\:')}',select='gte(t,0.4)'`, '-map', '0:v:0', '-frames:v', '1', '-update', '1', join(root, 'preview.png')]);
const pixels = spawnSync(ffmpeg, ['-nostdin', '-v', 'error', '-i', join(root, 'preview.png'), '-f', 'rawvideo', '-pix_fmt', 'rgb24', 'pipe:1'], { windowsHide: true, maxBuffer: 4 * 1024 * 1024 });
assert.equal(pixels.status, 0, pixels.stderr?.toString());
assert.equal(pixels.stdout.length, 640 * 360 * 3);
assert.ok(pixels.stdout.some(value => value > 32), 'Burned subtitle must produce visible pixels on the black video');
const result = {
  passed: true,
  verified: ['two subtitle ordinals selected independently', 'embedded ASS copied without SRT conversion', 'script resolution and named style preserved',
    'layer/actor/margins/commas preserved', 'move/fade/karaoke/drawing tags preserved', 'embedded SRT converted to ASS', 'ASS to SRT removes ASS effects', 'drawing-event filtering avoids vector text in SRT', 'extracted ASS renders through libass'],
  limitations: ['Rust unit tests were not executed', 'full application GUI was not run', 'font attachments are not extracted'],
};
writeFileSync(join(root, 'result.json'), `${JSON.stringify(result, null, 2)}\n`, 'utf8');
console.log(JSON.stringify(result, null, 2));
