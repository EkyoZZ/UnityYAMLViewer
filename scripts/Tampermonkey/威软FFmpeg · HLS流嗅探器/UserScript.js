// ==UserScript==
// @name         威软FFmpeg · HLS流嗅探器
// @namespace    https://weiruankeji2025.github.io/ffmpeg/
// @version      1.1.0
// @description  自动嗅探页面中的 HLS/m3u8 流地址，支持一键复制、在威软FFmpeg在线工具中处理、或直接下载 TS 文件（自动解密 AES-128）
// @author       威软 (Weiruan)
// @license      MIT
// @homepage     https://weiruankeji2025.github.io/ffmpeg/
// @match        *://*/*
// @exclude      https://weiruankeji2025.github.io/ffmpeg/*
// @grant        GM_xmlhttpRequest
// @grant        GM_setClipboard
// @grant        GM_addStyle
// @connect      *
// @run-at       document-start
// @downloadURL https://update.sleazyfork.org/scripts/573117/%E5%A8%81%E8%BD%AFFFmpeg%20%C2%B7%20HLS%E6%B5%81%E5%97%85%E6%8E%A2%E5%99%A8.user.js
// @updateURL https://update.sleazyfork.org/scripts/573117/%E5%A8%81%E8%BD%AFFFmpeg%20%C2%B7%20HLS%E6%B5%81%E5%97%85%E6%8E%A2%E5%99%A8.meta.js
// ==/UserScript==

/* jshint esversion: 11 */
'use strict';

// ================================================================
// 常量
// ================================================================

const TOOL_URL = 'https://weiruankeji2025.github.io/ffmpeg/';
const M3U8_RE = /\.m3u8(\?[^"'\s]*)?($|["'\s])/i;
const M3U8_URL_RE = /https?:\/\/[^\s"'<>]+\.m3u8[^\s"'<>]*/gi;
const CONTENT_TYPE_RE = /application\/(vnd\.apple\.mpegurl|x-mpegurl)/i;
const REQUEST_TIMEOUT = 30000; // 单请求超时（毫秒）
const MAX_HTML_SCAN = 3000000; // 全页文本扫描上限，避免超大文档拖垮主线程
const MAX_DETECTED = 300;      // 嗅探记录上限，防止长会话内存无限增长

// ================================================================
// 状态
// ================================================================

const detected = new Map();   // url → { url, title, time }
let panelEl = null;
let listEl = null;
let badgeEl = null;
let abortController = null;   // 用于取消下载

// ================================================================
// URL 过滤与记录
// ================================================================

function decodeUnicodeEscapes(text) {
  // 先解码再匹配地址，兼容被转义的协议、路径和查询参数。
  return text.replace(/\\u(?:\{([0-9a-fA-F]{1,6})\}|([0-9a-fA-F]{4}))/g, (match, point, unit) => {
    const code = parseInt(point || unit, 16);
    return code <= 0x10ffff ? String.fromCodePoint(code) : match;
  });
}

function isM3u8Url(url) {
  if (typeof url !== 'string') return false;
  url = decodeUnicodeEscapes(url);
  try { new URL(url); } catch { return false; }
  return M3U8_RE.test(url) || url.includes('m3u8');
}

function recordUrl(url) {
  if (!url || typeof url !== 'string') return;
  url = decodeUnicodeEscapes(url);
  url = url.split('#')[0];  // 去除 fragment
  if (detected.has(url)) return;
  // 超出上限时移除最早的记录，保持面板与缓存同步
  if (detected.size >= MAX_DETECTED) {
    const oldest = detected.keys().next().value;
    detected.delete(oldest);
    const staleItem = listEl?.querySelector(`[data-url="${CSS.escape(oldest)}"]`);
    if (staleItem) staleItem.remove();
  }
  detected.set(url, {
    url,
    host: (() => { try { return new URL(url).hostname; } catch { return ''; } })(),
    time: new Date().toLocaleTimeString(),
  });
  appendUrlToPanel(url, detected.get(url));
  updateBadge();
}

// ================================================================
// 劫持 XMLHttpRequest
// ================================================================

(function hookXHR() {
  const OrigOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    if (isM3u8Url(String(url))) {
      this._weiruanUrl = String(url);
      this.addEventListener('load', function () {
        recordUrl(this._weiruanUrl);
        // 也尝试从响应内容中提取子 m3u8
        // （二进制 responseType 下访问 responseText 会直接抛异常，需保护）
        try {
          const text = (this.responseType === '' || this.responseType === 'text') ? this.responseText : null;
          if (text) scanText(text, this._weiruanUrl);
        } catch { /* ignore */ }
      });
    }
    return OrigOpen.call(this, method, url, ...rest);
  };
})();

// ================================================================
// 劫持 fetch
// ================================================================

(function hookFetch() {
  const origFetch = window.fetch;
  window.fetch = function (input, init) {
    const url = String(typeof input === 'string' ? input : input?.url || '');
    if (isM3u8Url(url)) recordUrl(url);
    return origFetch.apply(this, arguments).then(resp => {
      if (isM3u8Url(resp.url)) recordUrl(resp.url);
      const ct = resp.headers.get('Content-Type') || '';
      if (CONTENT_TYPE_RE.test(ct)) recordUrl(resp.url || url);
      return resp;
    });
  };
})();

// ================================================================
// 扫描 DOM 中的 video/source 标签
// ================================================================

function scanDOM() {
  document.querySelectorAll('video[src],source[src]').forEach(el => {
    if (isM3u8Url(el.src)) recordUrl(el.src);
  });
  // 扫描页面 HTML 中的裸 URL
  // 截断超大文档，并排除自身面板（面板中展示的链接会被再次匹配产生垃圾记录）
  let html = document.documentElement.innerHTML;
  const panelIdx = html.indexOf('<div id="wr-panel"');
  if (panelIdx !== -1) html = html.slice(0, panelIdx);
  if (html.length > MAX_HTML_SCAN) html = html.slice(0, MAX_HTML_SCAN);
  scanText(html, location.href);
}

function scanText(text, baseUrl) {
  if (!text) return;
  text = decodeUnicodeEscapes(text);
  const matches = text.match(M3U8_URL_RE);
  if (!matches) return;
  matches.forEach(u => {
    try { recordUrl(new URL(u, baseUrl).href); } catch { recordUrl(u); }
  });
}

// ================================================================
// UI 构建
// ================================================================

GM_addStyle(`
  #wr-panel {
    position: fixed; z-index: 2147483647;
    bottom: 20px; right: 20px;
    width: 380px;
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0,0,0,.6);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 13px;
    color: #c9d1d9;
    overflow: hidden;
    transition: height .2s;
    user-select: none;
  }
  #wr-panel.collapsed { width: auto; border-radius: 20px; }
  #wr-header {
    display: flex; align-items: center; gap: 6px;
    padding: 10px 14px;
    background: #21262d;
    cursor: move;
    border-bottom: 1px solid #30363d;
  }
  #wr-panel.collapsed #wr-header { border-bottom: none; }
  #wr-title { flex: 1; font-weight: 600; font-size: 13px; }
  #wr-badge {
    background: #388bfd; color: #fff;
    border-radius: 10px; padding: 1px 7px;
    font-size: 11px; font-weight: 700;
    min-width: 18px; text-align: center;
  }
  #wr-badge.zero { background: #30363d; }
  #wr-toggle {
    background: none; border: none; color: #8b949e;
    cursor: pointer; font-size: 16px; line-height: 1;
    padding: 0 2px;
  }
  #wr-toggle:hover { color: #c9d1d9; }
  #wr-body { max-height: 420px; overflow-y: auto; padding: 6px 0; }
  .wr-empty { padding: 18px; text-align: center; color: #484f58; font-size: 12px; }
  .wr-item {
    padding: 8px 14px;
    border-bottom: 1px solid #21262d;
  }
  .wr-item:last-child { border-bottom: none; }
  .wr-item-url {
    font-size: 11px; color: #8b949e;
    word-break: break-all;
    margin-bottom: 6px;
    max-height: 36px; overflow: hidden;
  }
  .wr-item-url.expanded { max-height: none; }
  .wr-item-meta { color: #484f58; font-size: 10px; margin-bottom: 5px; }
  .wr-btns { display: flex; gap: 5px; flex-wrap: wrap; }
  .wr-btn {
    background: #21262d; border: 1px solid #30363d;
    color: #c9d1d9; border-radius: 5px;
    padding: 3px 9px; font-size: 11px;
    cursor: pointer; transition: background .15s;
  }
  .wr-btn:hover { background: #2d333b; }
  .wr-btn.primary { background: #238636; border-color: #2ea043; color: #fff; }
  .wr-btn.primary:hover { background: #2ea043; }
  .wr-btn.blue { background: #1f6feb; border-color: #388bfd; color: #fff; }
  .wr-btn.blue:hover { background: #388bfd; }
  .wr-btn:disabled { opacity: .45; cursor: not-allowed; }
  #wr-prog-wrap {
    padding: 8px 14px;
    border-top: 1px solid #30363d;
    display: none;
  }
  #wr-prog-wrap.visible { display: block; }
  #wr-prog-label { font-size: 11px; color: #8b949e; margin-bottom: 4px; }
  #wr-prog-bar-bg {
    background: #21262d; border-radius: 4px; height: 6px; overflow: hidden;
  }
  #wr-prog-bar { height: 6px; background: #388bfd; width: 0%; transition: width .2s; }
  #wr-prog-cancel {
    margin-top: 5px;
    background: none; border: none; color: #f85149;
    font-size: 11px; cursor: pointer; padding: 0;
  }
`);

function createPanel() {
  panelEl = document.createElement('div');
  panelEl.id = 'wr-panel';
  panelEl.className = 'collapsed';
  panelEl.innerHTML = `
    <div id="wr-header">
      <span style="font-size:15px">📡</span>
      <span id="wr-title">威软 HLS 嗅探器</span>
      <span id="wr-badge" class="zero">0</span>
      <button id="wr-toggle" title="展开/收起">▲</button>
    </div>
    <div id="wr-body" style="display:none">
      <div class="wr-empty" id="wr-empty">暂未嗅探到 m3u8 流，请播放视频…</div>
      <div id="wr-list"></div>
    </div>
    <div id="wr-prog-wrap">
      <div id="wr-prog-label">准备中…</div>
      <div id="wr-prog-bar-bg"><div id="wr-prog-bar"></div></div>
      <button id="wr-prog-cancel">✕ 取消下载</button>
    </div>
  `;
  document.body.appendChild(panelEl);

  listEl = panelEl.querySelector('#wr-list');
  badgeEl = panelEl.querySelector('#wr-badge');

  // 展开/收起
  const toggle = panelEl.querySelector('#wr-toggle');
  const body = panelEl.querySelector('#wr-body');
  toggle.addEventListener('click', () => {
    const open = body.style.display !== 'none';
    body.style.display = open ? 'none' : 'block';
    toggle.textContent = open ? '▲' : '▼';
    panelEl.classList.toggle('collapsed', open);
  });

  // 可拖动
  makeDraggable(panelEl, panelEl.querySelector('#wr-header'));

  // 取消下载
  panelEl.querySelector('#wr-prog-cancel').addEventListener('click', () => {
    if (abortController) abortController.abort();
    hideProgress();
  });

  // 把已有的 URL 填进去
  detected.forEach((info) => appendUrlToPanel(info.url, info));
  updateBadge();
}

function appendUrlToPanel(url, info) {
  if (!listEl) return;
  const empty = document.getElementById('wr-empty');
  if (empty) empty.style.display = 'none';

  const item = document.createElement('div');
  item.className = 'wr-item';
  item.dataset.url = url;

  const short = url.length > 80 ? url.slice(0, 80) + '…' : url;
  item.innerHTML = `
    <div class="wr-item-meta">${escHtml(info.host || '')} · ${escHtml(info.time)}</div>
    <div class="wr-item-url" title="${escHtml(url)}">${escHtml(short)}</div>
    <div class="wr-btns">
      <button class="wr-btn" data-act="copy">复制链接</button>
      <button class="wr-btn blue" data-act="open">在工具中打开</button>
      <button class="wr-btn primary" data-act="dl">直接下载 TS</button>
    </div>
  `;

  item.querySelector('[data-act="copy"]').addEventListener('click', () => {
    GM_setClipboard(url);
    showToast('已复制到剪贴板');
  });

  item.querySelector('[data-act="open"]').addEventListener('click', () => {
    window.open(`${TOOL_URL}?url=${encodeURIComponent(url)}`, '_blank');
  });

  item.querySelector('[data-act="dl"]').addEventListener('click', () => {
    startDownload(url);
  });

  listEl.prepend(item);
}

function updateBadge() {
  if (!badgeEl) return;
  const n = detected.size;
  badgeEl.textContent = n;
  badgeEl.className = n > 0 ? '' : 'zero';
}

// ================================================================
// 进度条
// ================================================================

function showProgress(label, pct) {
  const wrap = document.getElementById('wr-prog-wrap');
  const lbl = document.getElementById('wr-prog-label');
  const bar = document.getElementById('wr-prog-bar');
  if (!wrap) return;
  wrap.classList.add('visible');
  lbl.textContent = label;
  bar.style.width = pct + '%';
}

function hideProgress() {
  const wrap = document.getElementById('wr-prog-wrap');
  if (wrap) wrap.classList.remove('visible');
}

// ================================================================
// 下载实现（GM_xmlhttpRequest，支持 AES-128 解密）
// ================================================================

function gmFetch(url, responseType = 'text') {
  return new Promise((resolve, reject) => {
    GM_xmlhttpRequest({
      method: 'GET',
      url,
      responseType,
      timeout: REQUEST_TIMEOUT,
      onload: (r) => {
        if (r.status >= 400) reject(new Error(`HTTP ${r.status}`));
        else resolve(responseType === 'arraybuffer' ? r.response : r.responseText);
      },
      onerror: (r) => reject(new Error(`网络错误：${r.statusText || url}`)),
      ontimeout: () => reject(new Error('请求超时')),
    });
  });
}

function resolveUrl(base, rel) {
  try { return new URL(rel, base).href; } catch { return rel; }
}

function parsePlaylist(text, baseUrl) {
  const lines = text.split('\n').map(l => l.trim());
  const segs = [];
  let currentKey = null;
  let seqNum = 0;

  for (const line of lines) {
    if (line.startsWith('#EXT-X-MEDIA-SEQUENCE:')) {
      seqNum = parseInt(line.slice(22)) || 0;
    }
  }

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith('#EXT-X-KEY:')) {
      const method = (line.match(/METHOD=([^,\s\r\n]+)/) || [])[1] || 'NONE';
      if (method === 'NONE') {
        currentKey = null;
      } else {
        const uriMatch = line.match(/URI="([^"]+)"/);
        const ivHex = (line.match(/IV=0x([0-9a-fA-F]+)/i) || [])[1] || null;
        currentKey = {
          method,
          uri: uriMatch ? resolveUrl(baseUrl, uriMatch[1]) : null,
          ivHex,
        };
      }
    } else if (line.startsWith('#EXT-X-MAP:')) {
      // fMP4 初始化段：不拼接则输出文件无法解码
      const mapUriMatch = line.match(/URI="([^"]+)"/);
      if (mapUriMatch) {
        segs.push({ isInit: true, url: resolveUrl(baseUrl, mapUriMatch[1]) });
      }
    } else if (line.startsWith('#EXT-X-STREAM-INF')) {
      // Master playlist — 记录带宽，取下一个 URI 行并递归
      const varUri = lines[i + 1]?.trim();
      const bandwidth = parseInt((line.match(/BANDWIDTH=(\d+)/) || [])[1]) || 0;
      if (varUri && !varUri.startsWith('#')) {
        segs.push({ isMaster: true, url: resolveUrl(baseUrl, varUri), bandwidth });
        i++;
      }
    } else if (line.startsWith('#EXTINF:')) {
      const uri = lines[i + 1]?.trim();
      if (uri && !uri.startsWith('#')) {
        segs.push({ url: resolveUrl(baseUrl, uri), key: currentKey ? { ...currentKey, seqNum } : null });
        seqNum++;
        i++;
      }
    } else if (line && !line.startsWith('#')) {
      segs.push({ url: resolveUrl(baseUrl, line), key: currentKey ? { ...currentKey, seqNum } : null });
      seqNum++;
    }
  }
  return segs;
}

async function decryptSegment(data, keyInfo, keyCache) {
  if (!keyInfo || keyInfo.method === 'NONE' || !keyInfo.uri) return data;

  if (!keyCache.has(keyInfo.uri)) {
    const keyBuf = await gmFetch(keyInfo.uri, 'arraybuffer');
    keyCache.set(keyInfo.uri, keyBuf);
  }
  const keyData = keyCache.get(keyInfo.uri);

  let iv;
  if (keyInfo.ivHex) {
    const hex = keyInfo.ivHex.padStart(32, '0');
    iv = new Uint8Array(hex.match(/.{2}/g).map(b => parseInt(b, 16)));
  } else {
    iv = new Uint8Array(16);
    const n = keyInfo.seqNum;
    for (let j = 0; j < 4; j++) iv[15 - j] = (n >> (j * 8)) & 0xff;
  }

  const cryptoKey = await crypto.subtle.importKey(
    'raw', keyData, { name: 'AES-CBC' }, false, ['decrypt']
  );
  const decrypted = await crypto.subtle.decrypt({ name: 'AES-CBC', iv }, cryptoKey, data);
  return new Uint8Array(decrypted);
}

async function startDownload(m3u8Url) {
  if (abortController) abortController.abort();
  abortController = { aborted: false, abort() { this.aborted = true; } };
  const ac = abortController;

  // Disable all DL buttons
  document.querySelectorAll('.wr-btn[data-act="dl"]').forEach(b => b.disabled = true);
  showProgress('获取 m3u8 清单…', 0);

  try {
    // 1. 获取清单
    let manifestUrl = m3u8Url;
    let manifestText = await gmFetch(manifestUrl);

    // 2. Master playlist 处理 → 选择带宽最高的变体（原实现固定选第一个，多码率源会拿到最低画质）
    const parsed = parsePlaylist(manifestText, manifestUrl);
    const variants = parsed.filter(s => s.isMaster);
    if (variants.length > 0) {
      const best = variants.reduce((a, b) => (b.bandwidth > a.bandwidth ? b : a));
      manifestUrl = best.url;
      showProgress(`获取子清单（${best.bandwidth ? Math.round(best.bandwidth / 1000) + ' kbps' : '默认'}）…`, 2);
      manifestText = await gmFetch(manifestUrl);
    }

    if (ac.aborted) return;

    // 3. 解析分片
    const parsedMedia = parsePlaylist(manifestText, manifestUrl);
    const initSeg = parsedMedia.find(s => s.isInit);
    const segments = parsedMedia.filter(s => !s.isMaster && !s.isInit);

    if (segments.length === 0) throw new Error('清单中未找到媒体分片');

    const hasEnc = segments.some(s => s.key && s.key.method === 'AES-128');
    // SAMPLE-AES 等加密方式无法解密，提前告知用户输出可能损坏
    if (segments.some(s => s.key && s.key.method && s.key.method !== 'NONE' && s.key.method !== 'AES-128')) {
      showToast('检测到 SAMPLE-AES 加密，暂不支持解密，输出可能无法播放', true);
    }
    showProgress(`下载分片 0 / ${segments.length}${hasEnc ? '（自动解密）' : ''}…`, 3);

    // 4. 逐片下载 + 解密
    const keyCache = new Map();
    const chunks = [];

    // fMP4：初始化段必须拼接在最前
    if (initSeg) {
      try {
        chunks.push(new Uint8Array(await gmFetch(initSeg.url, 'arraybuffer')));
      } catch (e) {
        console.warn('[威软HLS] 初始化段获取失败:', e.message);
      }
    }
    let failed = 0;

    for (let i = 0; i < segments.length; i++) {
      if (ac.aborted) return;
      const pct = Math.round(3 + (i / segments.length) * 80);
      showProgress(`下载分片 ${i + 1} / ${segments.length}${hasEnc ? '（解密中）' : ''}…`, pct);

      try {
        const buf = await gmFetch(segments[i].url, 'arraybuffer');
        let data = new Uint8Array(buf);
        if (segments[i].key && segments[i].key.method === 'AES-128') {
          data = await decryptSegment(data, segments[i].key, keyCache);
        }
        chunks.push(data);
      } catch (e) {
        failed++;
        console.warn(`[威软HLS] 分片 ${i + 1} 失败：${e.message}`);
      }
    }

    if (chunks.length === 0) throw new Error('所有分片均下载失败');

    // 5. 触发下载（分片数组直接交给 Blob，避免额外拼接一份完整字节导致峰值内存翻倍）
    showProgress(`拼接 ${chunks.length} 个分片…`, 85);
    const blob = new Blob(chunks, { type: 'video/mp2t' });
    const dlUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = dlUrl;
    const baseTitle = (document.title || '').replace(/[<>:"/\\|?*\n\r]+/g, '_').trim().slice(0, 80);
    a.download = `${baseTitle || 'hls'}_${Date.now()}.ts`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(dlUrl), 30000);

    showProgress(`下载完成！共 ${chunks.length} 片${failed ? `，${failed} 片失败` : ''}`, 100);
    setTimeout(hideProgress, 3000);
    showToast('TS 文件下载完成');

  } catch (err) {
    if (!ac.aborted) {
      showProgress(`下载失败：${err.message}`, 0);
      setTimeout(hideProgress, 4000);
      showToast(`失败：${err.message}`, true);
    } else {
      hideProgress();
    }
  } finally {
    document.querySelectorAll('.wr-btn[data-act="dl"]').forEach(b => b.disabled = false);
  }
}

// ================================================================
// 工具函数
// ================================================================

function escHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

let toastEl = null;
let toastTimer = null;
function showToast(msg, isErr = false) {
  // 复用单个 Toast 元素，避免连续提示时堆叠
  if (!toastEl) {
    toastEl = document.createElement('div');
    Object.assign(toastEl.style, {
      position: 'fixed', zIndex: '2147483646',
      bottom: '70px', right: '20px',
      color: '#fff', padding: '8px 14px',
      borderRadius: '6px', fontSize: '13px',
      boxShadow: '0 4px 12px rgba(0,0,0,.4)',
      transition: 'opacity .4s',
    });
    document.body.appendChild(toastEl);
  }
  toastEl.textContent = msg;
  toastEl.style.background = isErr ? '#da3633' : '#238636';
  toastEl.style.opacity = '1';
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { toastEl.style.opacity = '0'; }, 2500);
}

function makeDraggable(el, handle) {
  let ox = 0, oy = 0, sx = 0, sy = 0;
  handle.addEventListener('mousedown', (e) => {
    e.preventDefault();
    sx = e.clientX; sy = e.clientY;
    const rect = el.getBoundingClientRect();
    ox = rect.left; oy = rect.top;
    el.style.right = 'auto';
    el.style.bottom = 'auto';
    el.style.left = ox + 'px';
    el.style.top = oy + 'px';

    function move(e) {
      el.style.left = (ox + e.clientX - sx) + 'px';
      el.style.top = (oy + e.clientY - sy) + 'px';
    }
    function up() {
      document.removeEventListener('mousemove', move);
      document.removeEventListener('mouseup', up);
    }
    document.addEventListener('mousemove', move);
    document.addEventListener('mouseup', up);
  });
}

// ================================================================
// 启动
// ================================================================

function init() {
  if (document.getElementById('wr-panel')) return;
  createPanel();
  scanDOM();

  // 监听 DOM 变化（单页应用动态加载）
  // 防抖：高频 DOM 变动下全页扫描代价高昂，合并为一次
  let scanTimer = null;
  new MutationObserver(() => {
    clearTimeout(scanTimer);
    scanTimer = setTimeout(scanDOM, 800);
  }).observe(document.body, { childList: true, subtree: true });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
