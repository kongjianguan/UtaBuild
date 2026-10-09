import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { XMLParser } from 'fast-xml-parser';
import { parseQrc } from '@applemusic-like-lyrics/lyric';
import OpenCC from 'opencc-js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const { values } = parseArgs({ options: {
  title: { type: 'string' }, artist: { type: 'string' },
  output: { type: 'string' }, 'song-id': { type: 'string' },
} });
if (!values.title || !values.output) throw new Error('必须提供 --title 和 --output');
const output = resolve(values.output);
if (existsSync(output) && readdirSync(output).length) {
  throw new Error('输出目录已有文件，请使用空目录保存新的歌曲来源');
}
mkdirSync(output, { recursive: true });

function run(command, args, input) {
  const result = spawnSync(command, args, { cwd: root, input, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(`${command} 执行失败：${result.stderr || result.error}`);
  return result.stdout;
}
function save(name, value) {
  writeFileSync(resolve(output, name), JSON.stringify(value, null, 2) + '\n');
}
const comm = { ct: '11', cv: '1003006', v: '1003006', os_ver: '15',
  phonetype: '24122RKC7C', tmeAppID: 'qqmusiclight', nettype: 'NETWORK_WIFI' };
function qqRequest(request, key) {
  const requestPath = resolve(output, `${request[key].method}-request.json`);
  const responsePath = resolve(output, `${request[key].method}-response.json`);
  save(`${request[key].method}-request.json`, request);
  run('curl', ['-fsSL', '--compressed', '--max-time', '30',
    'https://u.y.qq.com/cgi-bin/musicu.fcg', '-H', 'Referer: https://y.qq.com/',
    '--user-agent', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    '-H', 'Content-Type: application/json', '--data-binary', `@${requestPath}`, '-o', responsePath]);
  const response = JSON.parse(readFileSync(responsePath, 'utf8'));
  if (response.code !== 0 || response[key]?.code !== 0) throw new Error(`QQ Music 请求失败：${JSON.stringify(response)}`);
  return response;
}
const searchKey = 'music.search.SearchCgiService';
const search = qqRequest({ [searchKey]: {
  method: 'DoSearchForQQMusicDesktop', module: searchKey,
  param: { query: values.title, search_type: 0, num_per_page: 20, page_num: 1 },
} }, searchKey);
save('search.json', search);
const songs = search[searchKey].data.body.song.list;
const matches = songs.filter(song => song.name.toLowerCase() === values.title.toLowerCase()
  && (!values.artist || song.singer.some(s => s.name.toLowerCase() === values.artist.toLowerCase()))
  && (!values['song-id'] || String(song.id) === values['song-id']));
if (!matches.length) throw new Error('没有找到匹配的歌曲，请检查歌曲名称、歌手或 song-id');
const song = matches[0];
if (!values.artist && matches.some(item => item.singer.map(s => s.name).join('/') !== song.singer.map(s => s.name).join('/'))) {
  throw new Error('存在同名歌曲，请指定 --artist');
}
const artist = song.singer.map(s => s.name).join('/');
save('song.json', song);
const encode = text => Buffer.from(text).toString('base64');
const response = qqRequest({ comm, req_0: {
  method: 'GetPlayLyricInfo', module: 'music.musichallSong.PlayLyricInfo', param: {
    songID: song.id, songName: encode(song.name), singerName: encode(artist), albumName: encode(song.album.name),
    crypt: 1, qrc: 1, roma: 1, trans: 1, cv: 2111, ct: 19,
    lrc_t: 0, qrc_t: 0, roma_t: 0, trans_t: 0, type: 0, interval: song.interval,
  },
} }, 'req_0');
save('lyric-response.json', response);
run('cargo', ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', 'video-source',
  '--title', song.name, '--artist', artist, '--lyric-response', resolve(output, 'lyric-response.json'),
  '--output', resolve(output, 'source.json')]);
const source = JSON.parse(readFileSync(resolve(output, 'source.json'), 'utf8'));
if (source.title.toLowerCase() !== song.name.toLowerCase() || source.artist.toLowerCase() !== artist.toLowerCase()) {
  throw new Error('歌词来源的歌名和歌手与歌曲资料不一致');
}
if (new URL(source.ruby_url).origin !== 'https://utaten.com') throw new Error('注音来源必须为 UtaTen');
save('source.json', source);
function parse(xml) {
  const document = new XMLParser({ ignoreAttributes: false, trimValues: false }).parse(xml, true);
  const content = document.QrcInfos?.LyricInfo?.Lyric_1?.['@_LyricContent'];
  if (typeof content !== 'string') throw new Error('QRC 必须包含一份 Lyric_1 和 LyricContent');
  const lines = parseQrc(content);
  if (!lines.length) throw new Error('QRC 没有逐字时间信息');
  return lines;
}
const original = parse(source.original_qrc);
const romaji = parse(source.romaji_qrc);
save('timed-source.json', { original, romaji });
const normalize = text => text.normalize('NFC').replace(/[\s\p{Punctuation}]/gu, '').toLowerCase();
const rubySpans = [];
let rubyText = '';
for (const element of source.ruby_annotations) {
  if (element.type === 'ruby' && (!element.base?.trim() || !element.ruby?.trim())) {
    throw new Error('UtaTen Ruby 原文或读音为空');
  }
  if (!element.base) continue;
  const base = normalize(element.base);
  if (element.type === 'ruby') {
    if (!base || !element.ruby?.trim()) throw new Error('UtaTen Ruby 原文或读音为空');
    rubySpans.push({ start: [...rubyText].length, end: [...rubyText + base].length,
      base, reading: element.ruby });
  }
  rubyText += base;
}
let cursor = 0;
const lines = [];
const mappedRuby = new Set();
const sourceTitles = new Set([song.name, song.title].map(title => normalize(`${title} - ${artist}`)));
for (const item of original) {
  const text = item.words.map(word => word.word.normalize('NFC')).join('');
  if (!text.trim()) continue;
  const normalized = normalize(text);
  const position = rubyText.indexOf(normalized, cursor);
  if (position < 0) {
    // 来源中独立的制作人员信息不属于演唱内容。
    if (!lines.length && (sourceTitles.has(normalize(text))
      || /^(作词|作曲|编曲|制作|词：|曲：|歌手|翻译|翻譯|中文|日语|日語)/u.test(text.trim()))) continue;
    throw new Error(`逐字歌词在 UtaTen 原文中找不到：${text}`);
  }
  if (position !== cursor) throw new Error(`逐字歌词缺少 UtaTen 中的内容：${rubyText.slice(cursor, position)}`);
  const start = [...rubyText.slice(0, position)].length;
  const end = start + [...normalized].length;
  cursor = position + normalized.length;
  const characters = [...text];
  const characterPositions = characters.flatMap((character, index) => normalize(character) ? [index] : []);
  const overlapping = rubySpans.filter(span => span.start < end && span.end > start);
  if (overlapping.some(span => span.start < start || span.end > end)) {
    throw new Error(`UtaTen Ruby 跨越逐字歌词分句：${text}`);
  }
  const annotations = overlapping
    .map(span => {
      mappedRuby.add(span);
      const localStart = characterPositions[span.start - start];
      const localEnd = characterPositions[span.end - start - 1] + 1;
      return { start: localStart, end: localEnd, base: characters.slice(localStart, localEnd).join(''), reading: span.reading };
    });
  if (characters.some((character, index) => /\p{Script=Han}/u.test(character)
    && !annotations.some(annotation => annotation.start <= index && index < annotation.end))) {
    throw new Error(`原文汉字缺少 UtaTen 注音：${text}`);
  }
  const words = item.words.filter(word => word.word.length)
    .map(word => ({ text: word.word.normalize('NFC'), start_ms: word.startTime, end_ms: word.endTime }));
  const paired = romaji.filter(line => Math.abs(line.startTime - item.startTime) <= 2 && Math.abs(line.endTime - item.endTime) <= 2);
  if (paired.length !== 1) throw new Error(`罗马音歌词时间与原文不一致：${text}`);
  const japanese = /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u.test(text);
  const roman = japanese ? paired[0].words.filter(word => word.word.length)
    .map(word => ({ text: word.word, start_ms: word.startTime, end_ms: word.endTime })) : words;
  const sungWords = words.filter(word => /[\p{Letter}\p{Number}]/u.test(word.text));
  const sungRoman = roman.filter(word => /[\p{Script=Latin}\p{Number}]/u.test(word.text));
  if (!sungWords.length) throw new Error(`缺少原文演唱时间信息：${text}`);
  if (!sungRoman.length) throw new Error(`缺少罗马音时间信息：${text}`);
  if (sungRoman[0].start_ms > sungWords[0].start_ms + 2
    || sungRoman.at(-1).end_ms < sungWords.at(-1).end_ms - 2) {
    throw new Error(`罗马音缺少原文首末演唱时间：${text}`);
  }
  lines.push({ id: lines.length, text, start_ms: item.startTime, end_ms: item.endTime,
    words, romaji_words: roman, ruby: annotations });
}
if (lines.length < 2) throw new Error('有效演唱歌词不足');
if (cursor !== rubyText.length) throw new Error(`逐字歌词缺少 UtaTen 末尾内容：${rubyText.slice(cursor)}`);
if (mappedRuby.size !== rubySpans.length) throw new Error('UtaTen Ruby 未全部对应到逐字歌词');
const audio = resolve(output, 'audio.m4a');
const simplify = OpenCC.Converter({ from: 't', to: 'cn' });
const searchArtist = artist.replace(/\p{Punctuation}/gu, '');
const candidates = JSON.parse(run('yt-dlp', ['--flat-playlist', '--dump-single-json', '--skip-download',
  `ytsearch20:${searchArtist} ${song.name} auto-generated`]));
save('audio-candidates.json', candidates);
const exact = candidates.entries.filter(item => item.title.toLowerCase() === song.title.toLowerCase()
  && Number.isFinite(item.duration) && Math.abs(item.duration - song.interval) <= 5);
let audioUrl;
for (const item of exact) {
  const recording = JSON.parse(run('yt-dlp', ['--no-playlist', '--dump-single-json', '--skip-download', item.url]));
  if (recording.track?.toLowerCase() !== song.title.toLowerCase()
    || typeof recording.album !== 'string' || simplify(recording.album).toLowerCase() !== simplify(song.album.name).toLowerCase()
    || recording.artists?.length !== song.singer.length
    || !song.singer.every(singer => recording.artists?.some(name => name.toLowerCase() === singer.name.toLowerCase()))
    || Math.abs(recording.duration - song.interval) > 5) continue;
  audioUrl = recording.webpage_url;
  save('audio-selection.json', { title: recording.title, track: recording.track, artists: recording.artists,
    album: recording.album, duration: recording.duration, channel: recording.channel, channel_id: recording.channel_id,
    webpage_url: recording.webpage_url, description: recording.description });
  break;
}
if (!audioUrl) throw new Error('没有找到歌名、歌手、专辑和时长一致的正式音源');
run('yt-dlp', ['--no-playlist', '--force-overwrites', '--write-info-json', '-f', 'bestaudio', '-x', '--audio-format', 'm4a',
  '--no-progress', '-o', resolve(output, 'audio.%(ext)s'), audioUrl]);
const probe = JSON.parse(run('ffprobe', ['-v', 'error', '-show_format', '-show_streams', '-of', 'json', audio]));
const duration = Number(probe.format.duration) * 1000;
if (!probe.streams.some(stream => stream.codec_type === 'audio')) throw new Error('音频文件没有音轨');
if (!Number.isFinite(duration) || duration <= 0) throw new Error('音频时长无效');
if (Math.abs(duration - song.interval * 1000) > 5000) throw new Error('音频与歌词版本时长不一致，请选择录音室版音源');
if (Math.max(...lines.flatMap(line => line.words.map(word => word.end_ms))) > duration + 1000) {
  throw new Error('逐字歌词时间超出音频时长');
}
const cover = `https://y.gtimg.cn/music/photo_new/T002R800x800M000${song.album.mid}.jpg`;
run('curl', ['-fsSL', '--max-time', '30', '-o', resolve(output, 'cover.jpg'), cover]);
const project = { version: 1, title: song.name, artist, duration_ms: duration, audio_path: audio,
  audio_url: audioUrl,
  background_path: resolve(output, 'cover.jpg'), lyrics_url: `https://y.qq.com/n/ryqq/songDetail/${song.mid}`,
  ruby_url: source.ruby_url, lines };
save('project-source.json', project);
run(process.execPath, [resolve(root, 'tools/teaching-video/python.mjs'), 'calibrate',
  '--project', resolve(output, 'project-source.json'), '--output', resolve(output, 'project.json')]);
save('teaching-input.json', { title: song.name, artist, entries: [...new Set(lines.map(line => line.text))]
  .map(text => ({ text, ruby: lines.find(line => line.text === text).ruby })) });
console.log(JSON.stringify({ project: resolve(output, 'project.json'), lines: lines.length,
  teaching_input: resolve(output, 'teaching-input.json'), duration_ms: duration }));
