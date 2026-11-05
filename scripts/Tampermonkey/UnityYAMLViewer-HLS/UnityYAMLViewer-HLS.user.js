// ==UserScript==
// @name         UnityYAMLViewer · HLS 嗅探助手
// @namespace    https://github.com/EkyoZZ/UnityYAMLViewer
// @version      1.0.5
// @description  发现 HLS 地址，复制链接、在原在线工具或 UnityYamlViewer 中打开；脚本不直接下载视频。
// @author       UnityYAMLViewer contributors
// @license      MIT
// @match        http://*/*
// @match        https://*/*
// @grant        none
// @run-at       document-start
// ==/UserScript==

// Inspired by 威软FFmpeg · HLS流嗅探器 (Weiruan, MIT), kept separately.
(() => {
  'use strict';
  const TOOL_URL = 'https://weiruankeji2025.github.io/ffmpeg/';
  const seen = new Set();
  const pending = [];
  const panelId = 'uyv-hls-helper';
  let list, badge, panel, message;
  function record(value, force = false) {
    try {
      const url = new URL(String(value), location.href);
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) return;
      if (!force && !url.pathname.toLowerCase().endsWith('.m3u8')) return;
      url.hash = '';
      if (seen.has(url.href) || seen.size >= 300) return;
      seen.add(url.href);
      if (list) append(url.href); else pending.push(url.href);
    } catch { /* An invalid candidate must never interrupt page requests. */ }
  }
  function scanText(text) {
    const decoded = text.slice(0, 3000000)
      .replace(/\\u([0-9a-fA-F]{4})/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)))
      .replace(/\\\//g, '/');
    const matches = decoded.match(/https?:\/\/[^\s"'<>\\]+?\.m3u8(?:\?[^\s"'<>\\]*)?/gi) || [];
    for (const value of matches) record(value.replace(/&amp;/g, '&').replace(/&#38;/g, '&'));
  }
  const originalFetch = window.fetch;
  window.fetch = function (...args) {
    try { record(args[0] instanceof Request ? args[0].url : args[0]); } catch { /* ignore */ }
    return originalFetch.apply(this, args).then(response => {
      record(response.url, /application\/(?:vnd\.apple\.mpegurl|x-mpegurl)/i.test(response.headers.get('content-type') || ''));
      return response;
    });
  };
  const originalOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (...args) {
    const candidate = args[1];
    this.addEventListener('load', () => {
      record(this.responseURL || candidate, /application\/(?:vnd\.apple\.mpegurl|x-mpegurl)/i.test(this.getResponseHeader('content-type') || ''));
    }, { once: true });
    return originalOpen.apply(this, args);
  };
  function button(text, action, className = '') {
    const element = document.createElement('button');
    element.textContent = text;
    element.type = 'button';
    element.className = className;
    element.addEventListener('click', action);
    return element;
  }
  async function copy(value) {
    try {
      if (navigator.clipboard && window.isSecureContext) await navigator.clipboard.writeText(value);
      else {
        const field = document.createElement('textarea');
        field.value = value;
        field.style.cssText = 'position:fixed;left:-10000px;';
        document.body.appendChild(field);
        try { field.select(); if (!document.execCommand('copy')) throw new Error('copy failed'); }
        finally { field.remove(); }
      }
      message.textContent = '已复制地址';
    } catch { message.textContent = '复制失败，请选中地址手动复制。'; }
  }
  function append(url) {
    const item = document.createElement('div');
    item.className = 'uyv-item';
    const address = document.createElement('div');
    address.className = 'uyv-address';
    address.textContent = url;
    address.title = url;
    const actions = document.createElement('div');
    actions.className = 'uyv-actions';
    actions.append(button('复制链接', () => copy(url)), button('在工具中打开', () => {
      window.open(`${TOOL_URL}?url=${encodeURIComponent(url)}`, '_blank', 'noopener,noreferrer');
    }, 'uyv-online'), button('在 UnityYamlViewer 中打开', () => {
      const referer = new URL(location.href);
      referer.hash = '';
      const uri = `unityyamlviewer://open?url=${encodeURIComponent(url)}&referer=${encodeURIComponent(referer.href)}`;
      if (uri.length > 32000) { message.textContent = '地址过长，请复制后在软件中粘贴打开。'; return; }
      message.textContent = '已请求打开软件。首次使用请先运行新版软件，并允许浏览器打开外部应用。';
      location.assign(uri);
    }, 'uyv-app'));
    item.append(address, actions);
    list.appendChild(item);
    badge.textContent = `HLS ${seen.size}`;
    fitExpandedWidth();
  }
  function fitExpandedWidth() {
    if (!panel?.open) return;
    const actions = list.querySelector('.uyv-actions');
    if (!actions) return;
    const buttons = Array.from(actions.children);
    const gap = parseFloat(getComputedStyle(actions).columnGap) || 0;
    const itemStyle = getComputedStyle(actions.parentElement);
    const padding = parseFloat(itemStyle.paddingLeft) + parseFloat(itemStyle.paddingRight);
    const scrollbarsAndBorder = list.offsetWidth - list.clientWidth + panel.offsetWidth - panel.clientWidth;
    const width = buttons.reduce((sum, entry) => sum + entry.getBoundingClientRect().width, 0)
      + gap * Math.max(0, buttons.length - 1) + padding + scrollbarsAndBorder;
    panel.style.setProperty('--uyv-expanded-width', `${Math.ceil(width)}px`);
  }
  function scanDOM() {
    document.querySelectorAll('video[src],source[src]').forEach(element => record(element.src));
    const html = document.documentElement.innerHTML.replace(panel.outerHTML, '');
    scanText(html);
  }
  function installDrag(element, handle) {
    let drag = null;
    let suppressClick = false;
    function clampPosition() {
      if (!element.style.left) return;
      const rect = element.getBoundingClientRect();
      element.style.left = `${Math.max(0, Math.min(rect.left, window.innerWidth - rect.width))}px`;
      element.style.top = `${Math.max(0, Math.min(rect.top, window.innerHeight - rect.height))}px`;
    }
    handle.addEventListener('pointerdown', event => {
      if (event.button !== 0 || !event.isPrimary) return;
      const rect = element.getBoundingClientRect();
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY, left: rect.left, top: rect.top, moved: false };
      suppressClick = false;
      handle.setPointerCapture(event.pointerId);
    });
    handle.addEventListener('pointermove', event => {
      if (!drag || event.pointerId !== drag.id) return;
      const dx = event.clientX - drag.x, dy = event.clientY - drag.y;
      if (!drag.moved && Math.hypot(dx, dy) < 4) return;
      drag.moved = true;
      event.preventDefault();
      element.style.right = 'auto';
      element.style.bottom = 'auto';
      element.style.left = `${drag.left + dx}px`;
      element.style.top = `${drag.top + dy}px`;
      clampPosition();
    });
    const finish = event => {
      if (!drag || event.pointerId !== drag.id) return;
      suppressClick = drag.moved;
      drag = null;
      if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
    };
    handle.addEventListener('pointerup', finish);
    handle.addEventListener('pointercancel', finish);
    handle.addEventListener('lostpointercapture', finish);
    handle.addEventListener('click', event => {
      if (suppressClick) {
        event.preventDefault();
        event.stopImmediatePropagation();
        suppressClick = false;
      }
    }, true);
    element.addEventListener('toggle', clampPosition);
    window.addEventListener('resize', clampPosition);
  }
  function init() {
    if (!document.body || document.getElementById(panelId)) return;
    const style = document.createElement('style');
    style.textContent = `#${panelId}{position:fixed;right:16px;bottom:16px;z-index:2147483647;width:420px;max-width:calc(100vw - 32px);background:rgba(32,36,44,.3);color:#eee;border:1px solid #667;border-radius:7px;font:13px sans-serif;text-align:left}#${panelId} summary{cursor:pointer;padding:8px}#${panelId} .uyv-list{max-height:280px;overflow:auto}#${panelId} .uyv-item{padding:8px;border-top:1px solid #556}#${panelId} .uyv-address{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;margin-bottom:6px;user-select:text}#${panelId} button{cursor:pointer;margin-right:6px;padding:4px 7px;background:#303b50;color:#fff;border:1px solid #778;border-radius:4px}#${panelId} p{font-size:12px;padding:0 8px;line-height:1.4}`;
    style.textContent += `#${panelId} button.uyv-online{background:#1f6feb;border-color:#388bfd}#${panelId} button.uyv-online:hover{background:#388bfd}#${panelId} button.uyv-app{background:#238636;border-color:#2ea043}#${panelId} button.uyv-app:hover{background:#2ea043}`;
    style.textContent += `#${panelId}{box-sizing:border-box;max-height:calc(100vh - 16px);overflow:auto}#${panelId}:not([open]){width:max-content;background:rgba(32,36,44,.3)}#${panelId}:not([open]) .uyv-brand{display:none}#${panelId} summary{white-space:nowrap;user-select:none;touch-action:none;cursor:grab}#${panelId} summary:active{cursor:grabbing}`;
    style.textContent += `#${panelId}[open]{width:var(--uyv-expanded-width,360px)}#${panelId} .uyv-actions{display:flex;gap:6px;flex-wrap:wrap}#${panelId} .uyv-actions button{margin-right:0;flex-shrink:0;white-space:nowrap}`;
    document.head.appendChild(style);
    panel = document.createElement('details');
    panel.id = panelId;
    const header = document.createElement('summary');
    badge = document.createElement('span');
    badge.textContent = 'HLS 0';
    const brand = document.createElement('span');
    brand.className = 'uyv-brand';
    brand.textContent = 'UnityYAMLViewer · ';
    header.append(brand, badge);
    list = document.createElement('div');
    list.className = 'uyv-list';
    message = document.createElement('p');
    message.textContent = '播放网页视频后可发现更多地址；受登录、跨域或播放器环境限制的请求可能无法捕获。';
    panel.append(header, list, message);
    document.body.appendChild(panel);
    panel.addEventListener('toggle', fitExpandedWidth);
    window.addEventListener('resize', fitExpandedWidth);
    installDrag(panel, header);
    for (const url of pending.splice(0)) append(url);
    scanDOM();
    let timer;
    new MutationObserver(records => {
      if (records.every(change => panel.contains(change.target))) return;
      clearTimeout(timer);
      timer = setTimeout(scanDOM, 800);
    }).observe(document.body, { childList: true, subtree: true });
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init, { once: true });
  else init();
})();
