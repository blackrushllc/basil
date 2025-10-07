/* Basil Language Reference - Offline Manual Script */
(function() {
  const $ = sel => document.querySelector(sel);
  const $$ = sel => Array.from(document.querySelectorAll(sel));

  // Slugify to id string
  function slugify(s) {
    return s
      .toLowerCase()
      .replace(/^#+/, '') // drop leading # for directives like #CGI
      .replace(/[.$%`'"()\[\]{}:;+*,!?]/g, '')
      .replace(/\s+/g, '-')
      .replace(/[^a-z0-9_-]/g, '-')
      .replace(/-+/g, '-')
      .replace(/^-|-$|_/g, m => m === '_' ? '_' : '') // keep underscores
      .replace(/^-+|-+$/g, '');
  }

  // Very small Markdown parser (headings, paragraphs, lists, code fences, inline code)
  function parseMarkdown(md, { sectionLevel = 2, isCategory = false } = {}) {
    const lines = md.replace(/\r\n?/g, '\n').split('\n');
    let i = 0, html = '', inCode = false, codeLang = '', codeBuffer = [];
    let inList = false, listType = null;
    const headings = []; // {level, text, id}
    const sections = []; // {id, title, level, startHtmlIndex, shortDesc}
    let currentSection = null;

    function closeList() {
      if (inList) {
        html += listType === 'ol' ? '</ol>' : '</ul>';
        inList = false; listType = null;
      }
    }

    function flushCode() {
      if (inCode) {
        const raw = codeBuffer.join('\n');
        const lang = (codeLang || '').toLowerCase();
        const pre = `<pre class="language-${lang}"><button class="copy-btn" title="Copy">Copy</button><code>${escapeHtml(raw)}</code></pre>`;
        html += pre;
        inCode = false; codeLang = ''; codeBuffer = [];
      }
    }

    function startSectionIfNeeded(level, text, id) {
      // Close previous section's wrapper (no special tag needed; we rely on headings separation)
      const sec = { id, title: text, level, startHtmlIndex: html.length, shortDesc: '' };
      sections.push(sec);
      currentSection = sec;
    }

    while (i < lines.length) {
      const line = lines[i];

      // Code fence
      const codeStart = line.match(/^```\s*([a-zA-Z0-9_-]+)?\s*$/);
      if (codeStart && !inCode) {
        closeList();
        inCode = true; codeLang = codeStart[1] || '';
        i++; continue;
      }
      if (inCode) {
        const codeEnd = /^```\s*$/.test(line);
        if (codeEnd) { flushCode(); i++; continue; }
        codeBuffer.push(line);
        i++; continue;
      }

      // Headings
      const h = line.match(/^(#{1,6})\s+(.*)$/);
      if (h) {
        closeList();
        const level = h[1].length;
        const text = h[2].trim();
        const idBase = isCategory && level === 2 ? ('cat-' + slugify(text))
                     : (level >= sectionLevel ? ('kw-' + slugify(text)) : slugify(text));
        const id = idBase || ('h-' + headings.length);
        headings.push({ level, text, id });
        if (level >= 2) startSectionIfNeeded(level, text, id);
        html += `<h${level} id="${id}">${escapeHtml(text)}<a class="permalink" href="#${id}" aria-label="Permalink">#</a></h${level}>`;
        i++; continue;
      }

      // Lists
      const ol = line.match(/^\s*\d+\.\s+(.*)$/);
      const ul = line.match(/^\s*[-*+]\s+(.*)$/);
      if (ol || ul) {
        const type = ol ? 'ol' : 'ul';
        if (!inList) { closeList(); html += `<${type}>`; inList = true; listType = type; }
        const itemText = (ol ? ol[1] : ul[1]).trim();
        html += `<li>${inline(itemText)}</li>`;
        i++; continue;
      } else {
        closeList();
      }

      // Blockquote (rare here)
      const bq = line.match(/^>\s?(.*)$/);
      if (bq) { html += `<blockquote>${inline(bq[1])}</blockquote>`; i++; continue; }

      // The line is blank
      if (/^\s*$/.test(line)) { html += '\n'; i++; continue; }

      // Paragraph
      // Also track short description: the first non-Type paragraph after a heading
      const para = collectParagraph(lines, i);
      i = para.nextIndex;
      if (para.text.trim().length) {
        const raw = para.text.trim();
        // Skip the '*Type:* ...' line (we will still render it)
        const isTypeLine = /^\*?\s*Type\s*:\s*/i.test(raw);
        html += `<p>${inline(raw)}</p>`;
        if (currentSection && !currentSection.shortDesc && !isTypeLine) {
          currentSection.shortDesc = stripTags(inline(raw));
        }
        continue;
      }
    }

    closeList(); flushCode();
    return { html, headings, sections };
  }

  function collectParagraph(lines, start) {
    let i = start; let buf = [];
    while (i < lines.length && !/^\s*$/.test(lines[i]) && !/^(#{1,6})\s+/.test(lines[i]) && !/^```/.test(lines[i]) && !/^\s*\d+\./.test(lines[i]) && !/^\s*[-*+]\s+/.test(lines[i])) {
      buf.push(lines[i]); i++;
    }
    return { text: buf.join('\n'), nextIndex: i };
  }

  function inline(s) {
    // Escape first
    s = escapeHtml(s);
    // Emphasis **bold** and *italic*
    s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    s = s.replace(/\*([^*]+)\*/g, '<em>$1</em>');
    // Inline code
    s = s.replace(/`([^`]+)`/g, '<code>$1</code>');
    // Links [text](url)
    s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
    // Hard line breaks preserved if original had two spaces (already not tracked), we keep paragraphs simple
    return s;
  }

  function escapeHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
  function stripTags(s) { return s.replace(/<[^>]+>/g, ''); }

  // Simple search highlight
  function highlightText(text, query) {
    if (!query) return escapeHtml(text);
    const q = query.trim().toLowerCase();
    let res = '', i = 0;
    const src = text;
    const lower = src.toLowerCase();
    let idx = 0; let last = 0;
    while ((idx = lower.indexOf(q, last)) !== -1) {
      res += escapeHtml(src.slice(last, idx)) + '<mark>' + escapeHtml(src.slice(idx, idx + q.length)) + '</mark>';
      last = idx + q.length;
    }
    res += escapeHtml(src.slice(last));
    return res;
  }

  // Build sidebar lists
  function buildSidebar({ refSections, catSections, categoriesMap }) {
    const sidebar = $('.sidebar nav');
    const frag = document.createDocumentFragment();

    // Overview
    const groupOverview = div('div', { class: 'group' }, [
      el('div', { class: 'group-title' }, 'Overview'),
      el('ul', {}, [
        liLink('#overview', 'Introduction'),
        liLink('#alphabetical', 'Alphabetical Reference'),
        liLink('#by-category', 'By Category'),
      ])
    ]);
    frag.appendChild(groupOverview);

    // Alphabetical Reference
    const alphaDetails = el('details', { open: true }, [
      el('summary', { class: 'group-title' }, [span('span', { class: 'caret' }, '▶'), text('Alphabetical Reference')]),
      el('ul', {}, refSections.map(sec => liLink('#' + sec.id, sec.title)))
    ]);
    const groupAlpha = div('div', { class: 'group' }, [alphaDetails]);
    frag.appendChild(groupAlpha);

    // By Category
    const catRoot = el('div', { class: 'group' }, [el('div', { class: 'group-title' }, 'By Category')]);
    Object.keys(categoriesMap).forEach(catId => {
      const cat = categoriesMap[catId];
      const details = el('details', { open: false }, [
        el('summary', {}, [span('span', { class: 'caret' }, '▶'), text(cat.title)])
      ]);
      const ul = el('ul');
      cat.keywords.forEach(kw => ul.appendChild(liLink('#' + kw.id, kw.title)));
      details.appendChild(ul);
      catRoot.appendChild(details);
    });
    frag.appendChild(catRoot);

    sidebar.appendChild(frag);
  }

  // Create helper elements
  function el(tag, attrs = {}, children = []) {
    const node = document.createElement(tag);
    for (const k in attrs) { node.setAttribute(k, attrs[k]); }
    (Array.isArray(children) ? children : [children]).forEach(c => {
      if (c == null) return;
      if (typeof c === 'string') node.appendChild(document.createTextNode(c));
      else node.appendChild(c);
    });
    return node;
  }
  const div = el; const span = el; const text = s => document.createTextNode(s);
  function liLink(href, label) {
    const li = el('li');
    const a = el('a', { href }, label);
    li.appendChild(a); return li;
  }

  // Basil highlighter - extremely small and imperfect; keywords derived from reference headings
  function makeBasilHighlighter(allKeywordNames) {
    const kwList = Array.from(allKeywordNames).sort((a,b) => b.length - a.length);
    const kwRegex = new RegExp('\\b(' + kwList.map(escapeReg).join('|') + ')\\b', 'g');
    function escapeReg(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

    return function highlight(code) {
      // Handle strings first
      code = code.replace(/("[^"]*"|'[^']*')/g, '<span class="tok-str">$1</span>');
      // Numbers
      code = code.replace(/(^|[^\w])([+-]?(?:\d+\.?\d*|\d*\.?\d+))(?![\w@])/g, ($0, p1, num) => p1 + '<span class="tok-num">' + num + '</span>');
      // Comments starting with REM to end of line
      code = code.replace(/(^|\s)(REM\s.*)$/gm, ($0, p1, c2) => p1 + '<span class="tok-cmt">' + c2 + '</span>');
      // Keywords
      code = code.replace(kwRegex, '<span class="tok-kw">$1</span>');
      return code;
    };
  }

  // Attach copy buttons
  function enableCopyButtons() {
    $$('.copy-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const pre = btn.closest('pre');
        const code = pre ? pre.querySelector('code') : null;
        if (!code) return;
        const txt = code.innerText.replace(/\n$/, '');
        navigator.clipboard.writeText(txt).then(() => {
          btn.textContent = 'Copied!';
          setTimeout(() => btn.textContent = 'Copy', 1200);
        }).catch(() => {
          // Fallback
          const ta = el('textarea'); ta.value = txt; document.body.appendChild(ta); ta.select();
          try { document.execCommand('copy'); btn.textContent = 'Copied!'; } catch {}
          document.body.removeChild(ta);
          setTimeout(() => btn.textContent = 'Copy', 1200);
        });
      });
    });
  }

  // Search functionality
  function setupSearch(allItems) {
    const input = $('#search');
    function normalize(s) { return (s || '').toLowerCase(); }
    function matches(item, q) {
      const text = normalize(item.title + ' ' + (item.shortDesc || ''));
      const parts = q.split(/\s+/).filter(Boolean);
      return parts.every(p => text.includes(p));
    }
    function applyFilter(q) {
      const hasQ = q.trim().length > 0;
      // Main sections (keyword blocks)
      allItems.forEach(it => {
        const el = document.getElementById(it.id);
        if (!el) return;
        const sectionEl = el.tagName.match(/^H[23]$/) ? el.parentElement : el;
        const hit = matches(it, q);
        sectionEl.classList.toggle('hidden', hasQ && !hit);
        // Highlight heading text
        const titleNode = el.childNodes[0];
        if (titleNode && titleNode.nodeType === Node.TEXT_NODE) {
          el.innerHTML = `${highlightText(it.title, q)}<a class="permalink" href="#${it.id}">#</a>`;
        }
        // Highlight first paragraph (short desc)
        if (sectionEl) {
          const p = sectionEl.querySelector('p');
          if (p) p.innerHTML = highlightText(p.textContent, q);
        }
      });
      // Sidebar links
      $$('.sidebar nav a').forEach(a => {
        const id = (a.getAttribute('href') || '').replace(/^#/, '');
        const it = allItems.find(x => x.id === id);
        if (!it) return;
        const hit = matches(it, q);
        a.parentElement.classList.toggle('hidden', hasQ && !hit);
        a.innerHTML = highlightText(it.title, q);
      });

      // Hide empty category groups
      $$('.sidebar details').forEach(d => {
        const visible = Array.from(d.querySelectorAll('li')).some(li => !li.classList.contains('hidden'));
        d.classList.toggle('hidden', hasQ && !visible);
        if (hasQ) d.setAttribute('open', '');
      });
    }

    input.addEventListener('input', () => applyFilter(input.value));
  }

  // Scroll spy
  function setupScrollSpy() {
    const links = $$('.sidebar nav a');
    const map = new Map(links.map(a => [a.getAttribute('href'), a]));
    const targets = $$('h2[id], h3[id]');
    const obs = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        const id = '#' + entry.target.id;
        const a = map.get(id);
        if (!a) return;
        if (entry.isIntersecting) {
          links.forEach(l => l.classList.remove('active'));
          a.classList.add('active');
          // Expand parent details if collapsed
          const details = a.closest('details'); if (details) details.open = true;
        }
      });
    }, { rootMargin: '-50% 0px -40% 0px', threshold: [0, 1.0] });
    targets.forEach(t => obs.observe(t));
  }

  // Smooth scroll and deep link handling
  function setupLinkBehavior() {
    $$('.sidebar nav a').forEach(a => {
      a.addEventListener('click', (e) => {
        const href = a.getAttribute('href');
        if (!href || !href.startsWith('#')) return;
        e.preventDefault();
        const el = document.querySelector(href);
        if (el) { el.scrollIntoView({ behavior: 'smooth', block: 'start' }); history.pushState(null, '', href); }
      });
    });

    const hash = location.hash;
    if (hash) {
      const el = document.querySelector(hash);
      if (el) setTimeout(() => el.scrollIntoView({ behavior: 'smooth', block: 'start' }), 100);
    }
  }

  // Back to top links
  function addBackToTop() {
    $$('.content section').forEach(sec => {
      const back = el('div', { class: 'back-to-top' }, [el('a', { href: '#top' }, 'Back to top ↑')]);
      sec.appendChild(back);
    });
  }

  function groupCategories(catSections) {
    const categories = {};
    let currentCat = null;
    catSections.forEach(s => {
      if (s.level === 2) {
        currentCat = { id: s.id, title: s.title, keywords: [] };
        categories[s.id] = currentCat;
      } else if (s.level === 3 && currentCat) {
        currentCat.keywords.push(s);
      }
    });
    return categories;
  }

  function render() {
    const refMd = document.getElementById('md-ref').textContent;
    const catMd = document.getElementById('md-cat').textContent;

    const ref = parseMarkdown(refMd, { sectionLevel: 2, isCategory: false });
    const cat = parseMarkdown(catMd, { sectionLevel: 3, isCategory: true });

    // Build content HTML
    const content = $('.content');
    content.innerHTML = '';

    // Top anchor
    const topAnchor = el('a', { id: 'top' });
    content.appendChild(topAnchor);

    const intro = el('section', { id: 'overview' }, [
      el('h2', {}, ['Basil Language Reference']),
      el('p', {}, [
        'Basil is a compact, BASIC-flavored language for quick scripting, templating, and learning. ',
        'This manual consolidates the complete set of keywords, functions, flow-control constructs, directives, and core types. ',
        'Use the sidebar to browse alphabetically or by category, search to filter, and click the # button near each heading for a permalink.'
      ]),
      el('p', {}, [
        'Tip: Code samples include a copy button; the sidebar highlights where you are as you scroll.'
      ])
    ]);

    const alphaSec = el('section', { id: 'alphabetical' });
    alphaSec.innerHTML = `<h2>Alphabetical Reference<a class="permalink" href="#alphabetical">#</a></h2>` + ref.html;

    const catSec = el('section', { id: 'by-category' });
    catSec.innerHTML = `<h2>By Category<a class="permalink" href="#by-category">#</a></h2>` + cat.html;

    content.appendChild(intro);
    content.appendChild(alphaSec);
    content.appendChild(catSec);

    // Build sidebar
    const categoriesMap = groupCategories(cat.sections);
    const refKeywordSections = ref.sections.filter(s => s.level === 2);
    buildSidebar({ refSections: refKeywordSections, catSections: cat.sections, categoriesMap });

    // Syntax highlight code blocks tagged basil
    const allKeywordNames = new Set(refKeywordSections.map(s => s.title));
    const highlight = makeBasilHighlighter(allKeywordNames);
    $$('pre.language-basil code').forEach(code => { code.innerHTML = highlight(code.textContent); });

    // Enable copy, search, scroll behaviors
    enableCopyButtons();
    const allItems = ref.sections.concat(cat.sections).filter(s => s.level === 2 || s.level === 3);
    setupSearch(allItems);
    setupScrollSpy();
    setupLinkBehavior();

    // Append back-to-top to each keyword section
    // Wrap each H2/H3 + following content into a section element to place the back link
    wrapSections();
    addBackToTop();

    // Footer timestamp
    const ts = new Date();
    $('#last-generated').textContent = ts.toLocaleString();
  }

  function wrapSections() {
    // Transform structure so that each keyword heading and its subsequent block becomes <section>
    const container = $('.content');
    const nodes = Array.from(container.childNodes);
    const result = [];
    let currentSection = null;

    function startNewSection(afterNode) {
      currentSection = el('section');
      result.push(currentSection);
    }

    // Rebuild content: iterate and wrap H2/H3 originating from parsed content blocks
    const mainBlocks = $$('h2, h3');
    // Keep the top-level h2 (Alphabetical, By Category, Overview) out of wrapping
    const topIds = new Set(['overview', 'alphabetical', 'by-category']);

    const walker = document.createTreeWalker(container, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, null);
    const toMove = [];
    let node;
    while ((node = walker.nextNode())) {
      if (node.nodeType === Node.ELEMENT_NODE && /H[23]/.test(node.tagName)) {
        const id = node.getAttribute('id');
        if (id && topIds.has(id)) { currentSection = null; continue; }
        // Start new section for keyword/category headings from markdown
        currentSection = el('section');
        node.parentNode.insertBefore(currentSection, node);
        currentSection.appendChild(node);
        // Append following siblings until the next heading of same or higher level or until section changes context
        let sib = currentSection.nextSibling;
        while (sib && !(sib.nodeType === 1 && /H[23]/.test(sib.tagName))) {
          const next = sib.nextSibling;
          currentSection.appendChild(sib);
          sib = next;
        }
      }
    }
  }

  document.addEventListener('DOMContentLoaded', render);
})();
