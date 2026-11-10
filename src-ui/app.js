// Math Equation Editor & Converter Client Application
// Task 3.4: Output Format Selector & Close Confirmation ([X])

(function () {
  'use strict';

  // State
  let activeSessionId = null;
  let activeTargetMode = 'mathtype';
  let isModified = false;
  const BRIDGE_URL = 'http://127.0.0.1:17890';

  // DOM Elements
  const tabEditorBtn = document.getElementById('tab-editor-btn');
  const tabBatchBtn = document.getElementById('tab-batch-btn');
  const tabEditor = document.getElementById('tab-editor');
  const tabBatch = document.getElementById('tab-batch');

  const formatSelect = document.getElementById('format-select');
  const fontSizeGroup = document.getElementById('font-size-group');
  const fontSizeInput = document.getElementById('font-size-input');
  const bridgeBadge = document.getElementById('bridge-badge');
  const statusText = document.getElementById('status-text');

  const mathField = document.getElementById('math-field');
  const latexSource = document.getElementById('latex-source');

  const btnCopyLatex = document.getElementById('btn-copy-latex');
  const btnCopyMathml = document.getElementById('btn-copy-mathml');
  const btnCopyFlatOpc = document.getElementById('btn-copy-flat-opc');
  const btnCopyEmf = document.getElementById('btn-copy-emf');
  const btnCopyPng = document.getElementById('btn-copy-png');
  const btnToggleLatex = document.getElementById('btn-toggle-latex');
  const btnInsertUpdate = document.getElementById('btn-insert-update');
  const btnCancel = document.getElementById('btn-cancel');

  // Batch Converter Elements
  const batchDropzone = document.getElementById('batch-dropzone');
  const batchFileInput = document.getElementById('batch-file-input');
  const batchFileListContainer = document.getElementById('batch-file-list-container');
  const batchFileList = document.getElementById('batch-file-list');
  const batchFileCountLabel = document.getElementById('batch-file-count-label');
  const btnClearBatchFiles = document.getElementById('btn-clear-batch-files');
  const batchDirectionSelect = document.getElementById('batch-direction-select');
  const batchOutputModeSelect = document.getElementById('batch-output-mode-select');
  const btnRunBatch = document.getElementById('btn-run-batch');
  const btnOpenOutputFolder = document.getElementById('btn-open-output-folder');
  const batchProgressSection = document.getElementById('batch-progress-section');
  const batchProgressBar = document.getElementById('batch-progress-bar');
  const batchProgressLabel = document.getElementById('batch-progress-label');
  const batchProgressPercent = document.getElementById('batch-progress-percent');
  const batchStatFiles = document.getElementById('batch-stat-files');
  const batchStatFound = document.getElementById('batch-stat-found');
  const batchStatConverted = document.getElementById('batch-stat-converted');
  const batchStatSkipped = document.getElementById('batch-stat-skipped');
  const batchStatErrors = document.getElementById('batch-stat-errors');
  const batchLogOutput = document.getElementById('batch-log-output');

  const closeModal = document.getElementById('close-modal');
  const modalBtnYes = document.getElementById('modal-btn-yes');
  const modalBtnNo = document.getElementById('modal-btn-no');
  const modalBtnCancel = document.getElementById('modal-btn-cancel');

  // Disable MathLive audio feedback (soundOnError: null) to prevent Linux WebKit GStreamer warnings
  function disableMathLiveSounds() {
    if (window.MathfieldElement) {
      try {
        window.MathfieldElement.soundsDirectory = null;
        window.MathfieldElement.keypressSound = null;
      } catch (_) {}
    }
    if (mathField) {
      try {
        mathField.soundsDirectory = null;
        mathField.soundOnError = null;
        if (typeof mathField.setOptions === 'function') {
          mathField.setOptions({ soundOnError: null });
        }
      } catch (_) {}
    }
  }
  disableMathLiveSounds();

  // Parse session_id from URL query params (when opened via custom URI or bridge)
  const urlParams = new URLSearchParams(window.location.search);
  if (urlParams.has('session_id')) {
    activeSessionId = urlParams.get('session_id');
  }

  window.__loadSession = function (payload) {
    if (typeof payload === "string") {
      try { payload = JSON.parse(payload); } catch (_) {}
    }
    if (payload && payload.session_id) {
      console.log("[MathEditor] Direct loadSession invoked:", payload);
      loadSessionData(payload.session_id, payload.mathml, payload.latex, payload.font_pt);
    }
  };

  async function loadSessionData(sessionId, mathml, latex, fontPt) {
    if (!sessionId) return;
    activeSessionId = sessionId;
    if (typeof fontPt === 'number' && fontPt > 0 && fontSizeInput) {
      fontSizeInput.value = fontPt;
    }
    console.log("Loaded active session:", activeSessionId);

    if (tabEditorBtn) {
      try { tabEditorBtn.click(); } catch (_) {}
    }

    if (latex && typeof latex === "string" && latex.trim().length > 0) {
      latexSource.value = latex.trim();
      if (mathField) {
        if (typeof mathField.setValue === "function") {
          mathField.setValue(latex.trim(), { format: "latex" });
        }
        mathField.value = latex.trim();
      }
      isModified = false;
      return;
    }

    if (mathml && typeof mathml === "string" && mathml.trim().length > 0) {
      if (mathField) {
        try {
          if (typeof mathField.setValue === "function") {
            try {
              mathField.setValue(mathml.trim(), { format: "math-ml" });
            } catch (_) {
              mathField.setValue(mathml.trim(), { format: "mathml" });
            }
          }
          if (typeof mathField.getValue === "function") {
            latexSource.value = mathField.getValue("latex");
          }
          isModified = false;
          return;
        } catch (e) {
          console.warn("mathField.setValue(mathml) failed:", e);
        }
      }
    }

    try {
      const res = await fetch(`${BRIDGE_URL}/session/${sessionId}`);
      if (res.ok) {
        const data = await res.json();
        if (typeof data.font_pt === 'number' && data.font_pt > 0 && fontSizeInput) {
          fontSizeInput.value = data.font_pt;
        }
        if (data.latex && data.latex.trim().length > 0) {
          latexSource.value = data.latex.trim();
          if (mathField) {
            if (typeof mathField.setValue === "function") {
              mathField.setValue(data.latex.trim(), { format: "latex" });
            }
            mathField.value = data.latex.trim();
          }
        } else if (data.mathml && data.mathml.trim().length > 0) {
          if (mathField) {
            try {
              if (typeof mathField.setValue === "function") {
                try {
                  mathField.setValue(data.mathml.trim(), { format: "math-ml" });
                } catch (_) {
                  mathField.setValue(data.mathml.trim(), { format: "mathml" });
                }
              }
              if (typeof mathField.getValue === "function") {
                latexSource.value = mathField.getValue("latex");
              }
            } catch (_) {}
          }
        } else {
          // Fresh/empty session: clear canvas so no stale formula is displayed
          if (mathField) {
            if (typeof mathField.setValue === "function") {
              mathField.setValue("", { format: "latex" });
            }
            mathField.value = "";
          }
          if (latexSource) {
            latexSource.value = "";
          }
        }
      }
    } catch (err) {
      console.warn("Failed to fetch session data:", err);
    }
    isModified = false;
  }

  // 1. Navigation Tabs
  tabEditorBtn.addEventListener('click', () => {
    tabEditorBtn.classList.add('active');
    tabBatchBtn.classList.remove('active');
    tabEditor.classList.remove('hidden');
    tabBatch.classList.add('hidden');
  });

  tabBatchBtn.addEventListener('click', () => {
    tabBatchBtn.classList.add('active');
    tabEditorBtn.classList.remove('active');
    tabBatch.classList.remove('hidden');
    tabEditor.classList.add('hidden');
  });

  // Window hide helper for Tauri
  async function hideAppWindow() {
    if (window.__TAURI__) {
      if (window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
        try {
          await window.__TAURI__.core.invoke('hide_window');
          return;
        } catch (_) {}
      }
      if (window.__TAURI__.window) {
        try {
          await window.__TAURI__.window.getCurrentWindow().hide();
          return;
        } catch (_) {}
      }
    }
  }

  // Window close interception logic
  async function handleCloseRequested() {
    if (activeSessionId) {
      try {
        await fetch(`${BRIDGE_URL}/session/${activeSessionId}/cancel`, {
          method: "POST"
        });
      } catch (_) {}
      activeSessionId = null;
    }
    if (mathField) {
      if (typeof mathField.setValue === "function") {
        mathField.setValue("", { format: "latex" });
      }
      mathField.value = "";
    }
    if (latexSource) {
      latexSource.value = "";
    }
    isModified = false;
    await hideAppWindow();
  }

  // Listen to Tauri single instance, close interception, or batch tab events
  const onOpenSession = async (event) => {
    const payload = event ? (event.payload || event) : {};
    console.log('[MathEditor] Received open-session event:', payload);
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      try {
        await window.__TAURI__.core.invoke('show_window');
      } catch (_) {}
    }
    if (payload.session_id) {
      loadSessionData(payload.session_id, payload.mathml, payload.latex, payload.font_pt);
    }
  };

  if (window.__TAURI__) {
    const { listen } = window.__TAURI__.event || {};
    if (listen) {
      listen('open-batch-tab', () => {
        tabBatchBtn.click();
      });
      listen('window-close-requested', () => {
        handleCloseRequested();
      });
      listen('open-session', onOpenSession);
      listen('single-instance-launch', async (event) => {
        if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
          try {
            await window.__TAURI__.core.invoke('show_window');
          } catch (_) {}
        }
        const argv = event.payload || [];
        console.log('Single instance launched with args:', argv);
        for (const arg of argv) {
          if (typeof arg === 'string' && arg.includes('matheditor://open')) {
            try {
              const url = new URL(arg);
              const sid = url.searchParams.get('session_id');
              if (sid) {
                loadSessionData(sid);
              }
            } catch (e) {
              console.warn('Failed to parse custom URI:', e);
            }
          }
        }
      });
    }

    if (window.__TAURI__.window && typeof window.__TAURI__.window.getCurrentWindow === 'function') {
      try {
        window.__TAURI__.window.getCurrentWindow().listen('open-session', onOpenSession);
      } catch (_) {}
    }
    if (window.__TAURI__.webviewWindow && typeof window.__TAURI__.webviewWindow.getCurrentWebviewWindow === 'function') {
      try {
        window.__TAURI__.webviewWindow.getCurrentWebviewWindow().listen('open-session', onOpenSession);
      } catch (_) {}
    }
  }

  // Active Session Polling & Focus Detection
  async function checkLatestBridgeSession() {
    try {
      const res = await fetch(`${BRIDGE_URL}/session/latest`);
      if (res.ok) {
        const data = await res.json();
        if (data.session_id && data.session_id !== activeSessionId) {
          console.log('[MathEditor] Detected new active bridge session:', data.session_id);
          await loadSessionData(data.session_id, null, null, data.font_pt);
        }
      }
    } catch (_) {}
  }

  window.addEventListener('focus', checkLatestBridgeSession);
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) {
      checkLatestBridgeSession();
    }
  });

  setInterval(() => {
    if (document.hasFocus() || !document.hidden) {
      checkLatestBridgeSession();
    }
  }, 600);

  // 2. Format Selector & Font Size (Task 3.4)
  function updateFormatSelector() {
    activeTargetMode = formatSelect.value;
    if (activeTargetMode === 'mathtype') {
      fontSizeGroup.classList.remove('hidden');
    } else {
      fontSizeGroup.classList.add('hidden');
    }
  }

  formatSelect.addEventListener('change', () => {
    updateFormatSelector();
    isModified = true;
  });

  if (fontSizeInput) {
    fontSizeInput.addEventListener('input', () => {
      isModified = true;
    });
  }
  updateFormatSelector();

  // 3. Bidirectional Sync between MathLive Canvas and LaTeX Source
  function syncFromMathField() {
    if (!mathField) return;
    let latex = '';
    if (typeof mathField.getValue === 'function') {
      latex = mathField.getValue('latex');
    } else if (mathField.value) {
      latex = mathField.value;
    } else if (mathField.textContent) {
      latex = mathField.textContent;
    }
    if (latex && typeof latex === 'string' && latex.trim().length > 0 && latex.trim() !== 'undefined') {
      latexSource.value = latex.trim();
    }
  }

  // Wait for MathLive Web Component custom element upgrade
  if (window.customElements) {
    customElements.whenDefined('math-field').then(() => {
      disableMathLiveSounds();
      syncFromMathField();
    });
  }

  // Initial sync attempt
  if (mathField) {
    syncFromMathField();

    // Visual Canvas -> LaTeX Source Sync
    mathField.addEventListener('input', () => {
      syncFromMathField();
      isModified = true;
    });

    // LaTeX Source -> Visual Canvas Sync
    latexSource.addEventListener('input', () => {
      const newLatex = latexSource.value.trim().replace(/^\$|\$$/g, '');
      if (document.activeElement === latexSource && mathField) {
        if (typeof mathField.setValue === 'function') {
          mathField.setValue(newLatex, { format: 'latex' });
        } else {
          mathField.value = newLatex;
        }
      }
      isModified = true;
    });
  }

  // 4. Palette Category Tabs & Button Insertion (Task 3.3)
  const paletteTabBtns = document.querySelectorAll('.palette-tab-btn');
  const palettePanels = document.querySelectorAll('.palette-panel');

  paletteTabBtns.forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      paletteTabBtns.forEach((b) => b.classList.remove('active'));
      palettePanels.forEach((p) => p.classList.remove('active'));

      tabBtn.classList.add('active');
      const targetId = tabBtn.getAttribute('data-target');
      const targetPanel = document.getElementById(targetId);
      if (targetPanel) {
        targetPanel.classList.add('active');
      }
    });
  });

  document.querySelectorAll('.palette-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const snippet = btn.getAttribute('data-insert');
      if (!snippet) return;
      insertSnippet(snippet);
    });
  });

  function insertSnippet(snippet) {
    if (mathField) {
      if (typeof mathField.executeCommand === 'function') {
        mathField.executeCommand(['insert', snippet, { mode: 'math', selectionMode: 'placeholder' }]);
      } else if (typeof mathField.insert === 'function') {
        mathField.insert(snippet, { mode: 'math', selectionMode: 'placeholder' });
      } else {
        mathField.value = (mathField.value || '') + snippet.replace(/#\?/g, '');
      }
      syncFromMathField();
      mathField.focus();
    }
    isModified = true;
  }

  // MathType Classic Keyboard Shortcuts (Task 3.3 & 3.4)
  window.addEventListener('keydown', (e) => {
    // Handle Escape for close modal or closing window
    if (e.key === 'Escape') {
      if (!closeModal.classList.contains('hidden')) {
        closeModal.classList.add('hidden');
      } else {
        handleCloseRequested();
      }
      return;
    }

    if (document.activeElement === latexSource) return;

    const isCtrl = e.ctrlKey || e.metaKey;
    if (!isCtrl) return;

    const key = e.key.toLowerCase();

    // Ctrl+F: Fraction
    if (key === 'f') {
      e.preventDefault();
      insertSnippet('\\frac{#?}{#?}');
      return;
    }

    // Ctrl+R: Radical (Square root, or N-th root with Shift)
    if (key === 'r') {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet('\\sqrt[#?]{#?}');
      } else {
        insertSnippet('\\sqrt{#?}');
      }
      return;
    }

    // Ctrl+H: Superscript (High)
    if (key === 'h') {
      e.preventDefault();
      insertSnippet('^{#?}');
      return;
    }

    // Ctrl+L: Subscript (Low) or Limit with Shift
    if (key === 'l') {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet('\\lim_{#? \\to #?}');
      } else {
        insertSnippet('_{#?}');
      }
      return;
    }

    // Ctrl+J: Joint Super+Subscript
    if (key === 'j') {
      e.preventDefault();
      insertSnippet('_{#?}^{#?}');
      return;
    }

    // Ctrl+I: Integral
    if (key === 'i') {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet('\\int_{#?}^{#?} #? \\, d#?');
      } else {
        insertSnippet('\\int #? \\, d#?');
      }
      return;
    }

    // Ctrl+Shift+S: Summation
    if (key === 's' && e.shiftKey) {
      e.preventDefault();
      insertSnippet('\\sum_{#?}^{#?} #?');
      return;
    }

    // Ctrl+Shift+P: Product
    if (key === 'p' && e.shiftKey) {
      e.preventDefault();
      insertSnippet('\\prod_{#?}^{#?} #?');
      return;
    }

    // Ctrl+9 or Ctrl+(: Parentheses
    if (e.key === '9' || e.key === '(') {
      e.preventDefault();
      insertSnippet('\\left( #? \\right)');
      return;
    }

    // Ctrl+[ or Ctrl+]: Brackets
    if (e.key === '[' || e.key === ']') {
      e.preventDefault();
      insertSnippet('\\left[ #? \\right]');
      return;
    }

    // Ctrl+Shift+M: 2x2 Matrix
    if (key === 'm' && e.shiftKey) {
      e.preventDefault();
      insertSnippet('\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}');
      return;
    }

    // Ctrl+Shift+C: Cases
    if (key === 'c' && e.shiftKey) {
      e.preventDefault();
      insertSnippet('\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}');
      return;
    }
  });

  // 5. MathML Extraction Helper
  function getStandardMathML() {
    let mathml = '';
    if (mathField && typeof mathField.getValue === 'function') {
      try {
        mathml = (mathField.getValue('math-ml') || '').trim();
      } catch (e) {
        console.warn('MathLive getValue(math-ml) error:', e);
      }
    }
    if (!mathml) {
      const val = (latexSource.value || '').trim();
      return `<math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><mrow><mtext>${escapeXml(val)}</mtext></mrow></math>`;
    }
    if (!mathml.startsWith('<math')) {
      mathml = `<math xmlns="http://www.w3.org/1998/Math/MathML" display="block"><mrow>${mathml}</mrow></math>`;
    }
    return mathml;
  }

  // 6. Check Local Bridge Server Health
  async function checkBridgeHealth() {
    try {
      const res = await fetch(`${BRIDGE_URL}/health`);
      if (res.ok) {
        const data = await res.json();
        bridgeBadge.style.backgroundColor = '#ecfdf5';
        bridgeBadge.style.borderColor = '#a7f3d0';
        bridgeBadge.style.color = '#065f46';
        statusText.textContent = `Bridge: Active (${data.version})`;
        return true;
      }
    } catch (_) {}
    bridgeBadge.style.backgroundColor = '#fef2f2';
    bridgeBadge.style.borderColor = '#fecaca';
    bridgeBadge.style.color = '#991b1b';
    statusText.textContent = 'Bridge: Offline (Connecting...)';
    return false;
  }

  // Quick initial retry sequence (0ms, 300ms, 800ms, 1500ms) then 5s steady interval
  (async function initBridgeMonitoring() {
    for (const delay of [0, 300, 800, 1500]) {
      if (delay > 0) await new Promise(r => setTimeout(r, delay));
      const ok = await checkBridgeHealth();
      if (ok) break;
    }
    setInterval(checkBridgeHealth, 5000);
  })();

  // 7. Copy Actions
  btnCopyLatex.addEventListener('click', () => {
    const text = mathField && mathField.getValue ? mathField.getValue('latex') : latexSource.value;
    navigator.clipboard.writeText(text);
    showNotice(btnCopyLatex, 'Copied!');
  });

  btnCopyMathml.addEventListener('click', async () => {
    const mathml = getStandardMathML();
    await navigator.clipboard.writeText(mathml);
    showNotice(btnCopyMathml, 'MathML Copied!');
  });

  btnCopyFlatOpc.addEventListener('click', async () => {
    const mathml = getStandardMathML();
    const fontSize = parseFloat(fontSizeInput.value) || 12.0;

    // Tauri invoke if available
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      try {
        const target = activeTargetMode === 'omml' ? 'flat_opc_omml' : 'flat_opc_mathtype';
        const flatOpc = await window.__TAURI__.core.invoke('convert_equation', {
          data: mathml,
          fromFormat: 'mathml',
          toFormat: target,
          fontSizePt: fontSize
        });
        await navigator.clipboard.writeText(flatOpc);
        showNotice(btnCopyFlatOpc, 'Flat OPC Copied!');
        return;
      } catch (e) {
        console.warn('Tauri invoke convert_equation error, trying bridge:', e);
      }
    }
    // Fallback: request via bridge server
    try {
      const target = activeTargetMode === 'omml' ? 'flat_opc_omml' : 'flat_opc_mathtype';
      const res = await fetch(`${BRIDGE_URL}/convert/snippet`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          data: mathml,
          from: 'mathml',
          to: target,
          font_pt: fontSize
        })
      });
      if (res.ok) {
        const json = await res.json();
        await navigator.clipboard.writeText(json.result);
        showNotice(btnCopyFlatOpc, 'Flat OPC Copied!');
        return;
      }
    } catch (e) {
      console.warn('Bridge snippet fallback failed:', e);
    }

    await navigator.clipboard.writeText(`<!-- Flat OPC snippet placeholder for: ${mathml} -->`);
    showNotice(btnCopyFlatOpc, 'Flat OPC Copied!');
  });

  // Copy PNG picture (Task 6.2)
  if (btnCopyPng) {
    btnCopyPng.addEventListener('click', async () => {
      const mathml = getStandardMathML();
      const fontSize = parseFloat(fontSizeInput.value) || 12.0;
      let base64Data = null;

      if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
        try {
          base64Data = await window.__TAURI__.core.invoke('convert_equation', {
            data: mathml,
            fromFormat: 'mathml',
            toFormat: 'png',
            fontSizePt: fontSize
          });
        } catch (e) {
          console.warn('Tauri invoke png failed, trying bridge:', e);
        }
      }

      if (!base64Data) {
        try {
          const res = await fetch(`${BRIDGE_URL}/convert/snippet`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              data: mathml,
              from: 'mathml',
              to: 'png',
              font_pt: fontSize
            })
          });
          if (res.ok) {
            const json = await res.json();
            base64Data = json.result;
          }
        } catch (e) {
          console.warn('Bridge snippet png failed:', e);
        }
      }

      if (!base64Data) {
        showNotice(btnCopyPng, 'PNG Export Failed');
        return;
      }

      try {
        const byteCharacters = atob(base64Data);
        const byteNumbers = new Array(byteCharacters.length);
        for (let i = 0; i < byteCharacters.length; i++) {
          byteNumbers[i] = byteCharacters.charCodeAt(i);
        }
        const byteArray = new Uint8Array(byteNumbers);
        const blob = new Blob([byteArray], { type: 'image/png' });
        if (navigator.clipboard && navigator.clipboard.write) {
          await navigator.clipboard.write([
            new ClipboardItem({ 'image/png': blob })
          ]);
          showNotice(btnCopyPng, 'PNG Copied!');
        } else {
          await navigator.clipboard.writeText('data:image/png;base64,' + base64Data);
          showNotice(btnCopyPng, 'PNG Copied!');
        }
      } catch (err) {
        await navigator.clipboard.writeText('data:image/png;base64,' + base64Data);
        showNotice(btnCopyPng, 'PNG Copied!');
      }
    });
  }

  // Copy EMF Vector (Task 6.2)
  if (btnCopyEmf) {
    btnCopyEmf.addEventListener('click', async () => {
      const mathml = getStandardMathML();
      const fontSize = parseFloat(fontSizeInput.value) || 12.0;
      let base64Data = null;

      if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
        try {
          base64Data = await window.__TAURI__.core.invoke('convert_equation', {
            data: mathml,
            fromFormat: 'mathml',
            toFormat: 'emf',
            fontSizePt: fontSize
          });
        } catch (e) {
          console.warn('Tauri invoke emf failed, trying bridge:', e);
        }
      }

      if (!base64Data) {
        try {
          const res = await fetch(`${BRIDGE_URL}/convert/snippet`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              data: mathml,
              from: 'mathml',
              to: 'emf',
              font_pt: fontSize
            })
          });
          if (res.ok) {
            const json = await res.json();
            base64Data = json.result;
          }
        } catch (e) {
          console.warn('Bridge snippet emf failed:', e);
        }
      }

      if (!base64Data) {
        showNotice(btnCopyEmf, 'EMF Export Failed');
        return;
      }

      try {
        await navigator.clipboard.writeText(base64Data);
        showNotice(btnCopyEmf, 'EMF Copied!');
      } catch (err) {
        showNotice(btnCopyEmf, 'Clipboard Error');
      }
    });
  }

  // 8. Toggle LaTeX
  btnToggleLatex.addEventListener('click', () => {
    let val = latexSource.value.trim();
    if (val.startsWith('$') && val.endsWith('$')) {
      val = val.slice(1, -1).trim();
    } else {
      val = `$${val}$`;
    }
    latexSource.value = val;
    const cleanLatex = val.replace(/^\$|\$$/g, '').trim();
    if (mathField && mathField.setValue) {
      mathField.setValue(cleanLatex, { format: 'latex' });
    }
  });

  // 9. Insert / Update in Document (Resolve Active Session)
  btnInsertUpdate.addEventListener('click', async () => {
    await saveAndResolveSession();
  });

  async function saveAndResolveSession() {
    const mathml = getStandardMathML();
    const latex = mathField && mathField.getValue ? mathField.getValue('latex') : latexSource.value;
    const fontSize = parseFloat(fontSizeInput.value) || 12.0;

    if (!activeSessionId) {
      try {
        const latestRes = await fetch(`${BRIDGE_URL}/session/latest`);
        if (latestRes.ok) {
          const latestJson = await latestRes.json();
          if (latestJson.session_id) {
            activeSessionId = latestJson.session_id;
            console.log("Auto-attached to latest active bridge session:", activeSessionId);
          }
        }
      } catch (e) {
        console.warn("Could not check latest bridge session:", e);
      }
    }

    if (activeSessionId) {
      try {
        const res = await fetch(`${BRIDGE_URL}/session/${activeSessionId}/update`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            mathml: mathml,
            latex: latex,
            target_mode: activeTargetMode,
            font_pt: fontSize
          })
        });
        if (res.ok) {
          showNotice(btnInsertUpdate, 'Saved!');
          isModified = false;
          activeSessionId = null;
          if (mathField) {
            if (typeof mathField.setValue === 'function') {
              mathField.setValue('', { format: 'latex' });
            }
            mathField.value = '';
          }
          if (latexSource) {
            latexSource.value = '';
          }
          await hideAppWindow();
          return;
        }
      } catch (err) {
        console.error('Failed to update bridge session:', err);
      }
    }

    showNotice(btnInsertUpdate, 'Saved!');
    isModified = false;
  }

  // 10. Close Modal Interception (Task 3.4)
  btnCancel.addEventListener('click', () => {
    handleCloseRequested();
  });

  modalBtnYes.addEventListener('click', async () => {
    closeModal.classList.add('hidden');
    await saveAndResolveSession();
    await hideAppWindow();
  });

  modalBtnNo.addEventListener('click', async () => {
    closeModal.classList.add('hidden');
    if (activeSessionId) {
      try {
        await fetch(`${BRIDGE_URL}/session/${activeSessionId}/cancel`, {
          method: 'POST'
        });
      } catch (_) {}
      activeSessionId = null;
    }
    isModified = false;
    await hideAppWindow();
  });

  modalBtnCancel.addEventListener('click', () => {
    closeModal.classList.add('hidden');
    // Stay in editor
  });

  // Close modal when clicking outside box
  closeModal.addEventListener('click', (e) => {
    if (e.target === closeModal) {
      closeModal.classList.add('hidden');
    }
  });

  // ==========================================
  // Batch Document Converter Logic (Task 6.1)
  // ==========================================
  let batchFiles = [];
  let lastOutputFolder = '';

  function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function appendBatchLog(msg) {
    if (!batchLogOutput) return;
    const time = new Date().toLocaleTimeString();
    batchLogOutput.textContent += `\n[${time}] ${msg}`;
    batchLogOutput.scrollTop = batchLogOutput.scrollHeight;
  }

  function renderBatchFileList() {
    if (!batchFileList || !batchFileListContainer) return;
    if (batchFiles.length === 0) {
      batchFileListContainer.classList.add('hidden');
      if (btnRunBatch) btnRunBatch.disabled = true;
      return;
    }

    batchFileListContainer.classList.remove('hidden');
    if (btnRunBatch) btnRunBatch.disabled = false;
    if (batchFileCountLabel) {
      batchFileCountLabel.textContent = `Selected Files (${batchFiles.length})`;
    }

    batchFileList.innerHTML = '';
    batchFiles.forEach((fileItem) => {
      const row = document.createElement('div');
      row.className = 'batch-file-item';

      const info = document.createElement('div');
      info.className = 'file-info';

      const nameEl = document.createElement('span');
      nameEl.className = 'file-name';
      nameEl.textContent = fileItem.name;

      const pathEl = document.createElement('span');
      pathEl.className = 'file-path';
      pathEl.textContent = fileItem.path ? `${fileItem.path} (${formatBytes(fileItem.size)})` : formatBytes(fileItem.size);

      info.appendChild(nameEl);
      info.appendChild(pathEl);

      const badge = document.createElement('span');
      badge.className = `file-status-badge status-${fileItem.status}`;
      badge.textContent = fileItem.status.toUpperCase();

      row.appendChild(info);
      row.appendChild(badge);
      batchFileList.appendChild(row);
    });
  }

  function addFilesToBatch(fileList) {
    for (const f of fileList) {
      const name = f.name || '';
      const path = f.path || f.name || '';
      if (!name.toLowerCase().endsWith('.docx') && !path.toLowerCase().endsWith('.docx')) {
        continue;
      }
      if (!batchFiles.some(item => item.path === path)) {
        batchFiles.push({
          name: name || path.split(/[\/\\]/).pop(),
          path: path,
          size: f.size || 0,
          status: 'pending'
        });
      }
    }
    renderBatchFileList();
    appendBatchLog(`Loaded ${batchFiles.length} file(s) for conversion.`);
  }

  if (batchDropzone) {
    batchDropzone.addEventListener('click', () => {
      if (batchFileInput) batchFileInput.click();
    });

    batchDropzone.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.stopPropagation();
      batchDropzone.style.borderColor = 'var(--primary-color)';
      batchDropzone.style.backgroundColor = '#e0f2fe';
    });

    batchDropzone.addEventListener('dragleave', (e) => {
      e.preventDefault();
      e.stopPropagation();
      batchDropzone.style.borderColor = '#93c5fd';
      batchDropzone.style.backgroundColor = '#f0f7ff';
    });

    batchDropzone.addEventListener('drop', (e) => {
      e.preventDefault();
      e.stopPropagation();
      batchDropzone.style.borderColor = '#93c5fd';
      batchDropzone.style.backgroundColor = '#f0f7ff';
      if (e.dataTransfer && e.dataTransfer.files) {
        addFilesToBatch(e.dataTransfer.files);
      }
    });
  }

  if (batchFileInput) {
    batchFileInput.addEventListener('change', () => {
      if (batchFileInput.files) {
        addFilesToBatch(batchFileInput.files);
      }
    });
  }

  if (btnClearBatchFiles) {
    btnClearBatchFiles.addEventListener('click', () => {
      batchFiles = [];
      renderBatchFileList();
      if (batchProgressSection) batchProgressSection.classList.add('hidden');
      if (btnOpenOutputFolder) btnOpenOutputFolder.classList.add('hidden');
      if (batchLogOutput) batchLogOutput.textContent = 'Cleared file list. Ready for new files.';
    });
  }

  if (btnRunBatch) {
    btnRunBatch.addEventListener('click', async () => {
      if (batchFiles.length === 0) return;

      btnRunBatch.disabled = true;
      if (btnClearBatchFiles) btnClearBatchFiles.disabled = true;
      if (batchProgressSection) batchProgressSection.classList.remove('hidden');
      if (btnOpenOutputFolder) btnOpenOutputFolder.classList.add('hidden');

      const direction = batchDirectionSelect ? batchDirectionSelect.value : 'upgrade_modern_omml';
      const outputMode = batchOutputModeSelect ? batchOutputModeSelect.value : 'suffix';
      const fontSize = parseFloat(fontSizeInput.value) || 12.0;

      let totalFound = 0;
      let totalConverted = 0;
      let totalSkipped = 0;
      let totalErrors = 0;

      batchLogOutput.textContent = `Starting batch conversion of ${batchFiles.length} file(s)...\nDirection: ${direction}\nOutput Mode: ${outputMode}\n---`;

      for (let i = 0; i < batchFiles.length; i++) {
        const item = batchFiles[i];
        item.status = 'converting';
        renderBatchFileList();

        const pct = Math.round(((i) / batchFiles.length) * 100);
        if (batchProgressBar) batchProgressBar.style.width = `${pct}%`;
        if (batchProgressPercent) batchProgressPercent.textContent = `${pct}%`;
        if (batchProgressLabel) batchProgressLabel.textContent = `Converting ${i + 1}/${batchFiles.length}: ${item.name}...`;

        let outPath = null;
        if (outputMode === 'suffix') {
          outPath = item.path.replace(/\.docx$/i, '_converted.docx');
        } else {
          outPath = item.path;
        }

        let result = null;
        let errorMsg = null;

        // Try Tauri command
        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
          try {
            result = await window.__TAURI__.core.invoke('convert_document', {
              inputPath: item.path,
              outputPath: outPath,
              direction: direction,
              fontSizePt: fontSize
            });
          } catch (err) {
            errorMsg = err.toString();
          }
        }

        // Bridge HTTP fallback
        if (!result && !errorMsg) {
          try {
            const res = await fetch(`${BRIDGE_URL}/convert/document`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                input_path: item.path,
                output_path: outPath,
                target: direction.includes('mathtype') ? 'upgrade_mathtype' : 'upgrade_omml',
                preview: 'emf',
                font_pt: fontSize
              })
            });
            if (res.ok) {
              result = await res.json();
            } else {
              errorMsg = `Bridge error ${res.status}: ${await res.text()}`;
            }
          } catch (err) {
            errorMsg = err.toString();
          }
        }

        if (result && result.status === 'ok') {
          item.status = 'done';
          const found = result.found || 0;
          const converted = result.converted || 0;
          const skipped = result.skipped || 0;
          const errors = result.errors || 0;

          totalFound += found;
          totalConverted += converted;
          totalSkipped += skipped;
          totalErrors += errors;

          lastOutputFolder = outPath;
          appendBatchLog(`✓ [${i + 1}/${batchFiles.length}] ${item.name}: found ${found}, converted ${converted}, skipped ${skipped}, errors ${errors}`);
        } else {
          item.status = 'error';
          totalErrors += 1;
          appendBatchLog(`✗ [${i + 1}/${batchFiles.length}] ${item.name}: FAILED - ${errorMsg || 'unknown error'}`);
        }

        // Update live stats
        if (batchStatFiles) batchStatFiles.textContent = `${i + 1} / ${batchFiles.length}`;
        if (batchStatFound) batchStatFound.textContent = totalFound;
        if (batchStatConverted) batchStatConverted.textContent = totalConverted;
        if (batchStatSkipped) batchStatSkipped.textContent = totalSkipped;
        if (batchStatErrors) batchStatErrors.textContent = totalErrors;

        renderBatchFileList();
      }

      // Finish
      if (batchProgressBar) batchProgressBar.style.width = '100%';
      if (batchProgressPercent) batchProgressPercent.textContent = '100%';
      if (batchProgressLabel) batchProgressLabel.textContent = 'Batch Conversion Complete!';
      appendBatchLog(`---\nAll ${batchFiles.length} file(s) finished. Total equations converted: ${totalConverted}, errors: ${totalErrors}.`);

      btnRunBatch.disabled = false;
      if (btnClearBatchFiles) btnClearBatchFiles.disabled = false;
      if (btnOpenOutputFolder && lastOutputFolder) {
        btnOpenOutputFolder.classList.remove('hidden');
      }
    });
  }

  if (btnOpenOutputFolder) {
    btnOpenOutputFolder.addEventListener('click', async () => {
      if (!lastOutputFolder) return;
      if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
        try {
          await window.__TAURI__.core.invoke('open_folder', { path: lastOutputFolder });
          return;
        } catch (_) {}
      }
      try {
        await fetch(`${BRIDGE_URL}/open/folder`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path: lastOutputFolder })
        });
      } catch (e) {
        console.warn('Failed to open folder:', e);
      }
    });
  }

  // Helpers
  function showNotice(buttonEl, text) {
    const orig = buttonEl.textContent;
    buttonEl.textContent = text;
    setTimeout(() => {
      buttonEl.textContent = orig;
    }, 1800);
  }

  function escapeXml(unsafe) {
    return unsafe.replace(/[<>&'"]/g, function (c) {
      switch (c) {
        case '<': return '&lt;';
        case '>': return '&gt;';
        case '&': return '&amp;';
        case '\'': return '&apos;';
        case '"': return '&quot;';
      }
    });
  }
})();
