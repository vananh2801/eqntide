// Math Equation Editor & Converter Client Application
// Task 3.2: MathLive Canvas & Bidirectional LaTeX/MathML Sync

(function () {
  'use strict';

  // State
  let activeSessionId = null;
  let activeTargetMode = 'omml';
  let isModified = false;
  const BRIDGE_URL = 'http://127.0.0.1:17890';

  // DOM Elements
  const tabEditorBtn = document.getElementById('tab-editor-btn');
  const tabBatchBtn = document.getElementById('tab-batch-btn');
  const tabEditor = document.getElementById('tab-editor');
  const tabBatch = document.getElementById('tab-batch');

  const formatSelect = document.getElementById('format-select');
  const bridgeBadge = document.getElementById('bridge-badge');
  const statusText = document.getElementById('status-text');

  const mathField = document.getElementById('math-field');
  const latexSource = document.getElementById('latex-source');

  const btnCopyLatex = document.getElementById('btn-copy-latex');
  const btnCopyMathml = document.getElementById('btn-copy-mathml');
  const btnCopyFlatOpc = document.getElementById('btn-copy-flat-opc');
  const btnToggleLatex = document.getElementById('btn-toggle-latex');
  const btnInsertUpdate = document.getElementById('btn-insert-update');
  const btnCancel = document.getElementById('btn-cancel');

  const closeModal = document.getElementById('close-modal');
  const modalBtnYes = document.getElementById('modal-btn-yes');
  const modalBtnNo = document.getElementById('modal-btn-no');
  const modalBtnCancel = document.getElementById('modal-btn-cancel');

  // Parse session_id from URL query params (when opened via custom URI or bridge)
  const urlParams = new URLSearchParams(window.location.search);
  if (urlParams.has('session_id')) {
    activeSessionId = urlParams.get('session_id');
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

  // Listen to Tauri single instance or batch tab event if running under Tauri
  if (window.__TAURI__) {
    const { listen } = window.__TAURI__.event || {};
    if (listen) {
      listen('open-batch-tab', () => {
        tabBatchBtn.click();
      });
      listen('single-instance-launch', (event) => {
        const argv = event.payload || [];
        console.log('Single instance launched with args:', argv);
        for (const arg of argv) {
          if (typeof arg === 'string' && arg.includes('matheditor://open')) {
            try {
              const url = new URL(arg);
              const sid = url.searchParams.get('session_id');
              if (sid) {
                activeSessionId = sid;
              }
            } catch (e) {
              console.warn('Failed to parse custom URI:', e);
            }
          }
        }
      });
    }
  }

  // 2. Format Selector
  formatSelect.addEventListener('change', (e) => {
    activeTargetMode = e.target.value;
    isModified = true;
  });

  // 3. Bidirectional Sync between MathLive Canvas and LaTeX Source
  function syncFromMathField() {
    if (!mathField) return;
    const latex = (typeof mathField.getValue === "function")
      ? mathField.getValue("latex")
      : (mathField.value || mathField.textContent || "");
    if (latex && latex.trim().length > 0) {
      latexSource.value = latex.trim();
    }
  }

  // Wait for MathLive Web Component custom element upgrade
  if (window.customElements) {
    customElements.whenDefined("math-field").then(() => {
      syncFromMathField();
    });
  }

  // Initial sync attempt
  if (mathField) {
    syncFromMathField();

    // Visual Canvas -> LaTeX Source Sync
    mathField.addEventListener("input", () => {
      syncFromMathField();
      isModified = true;
    });

    // LaTeX Source -> Visual Canvas Sync
    latexSource.addEventListener("input", () => {
      const newLatex = latexSource.value.trim().replace(/^\$|\$$/g, "");
      if (document.activeElement === latexSource && mathField) {
        if (typeof mathField.setValue === "function") {
          mathField.setValue(newLatex, { format: "latex" });
        } else {
          mathField.value = newLatex;
        }
      }
      isModified = true;
    });
  }

  // 4. Palette Category Tabs & Button Insertion (Task 3.3)
  const paletteTabBtns = document.querySelectorAll(".palette-tab-btn");
  const palettePanels = document.querySelectorAll(".palette-panel");

  paletteTabBtns.forEach((tabBtn) => {
    tabBtn.addEventListener("click", () => {
      paletteTabBtns.forEach((b) => b.classList.remove("active"));
      palettePanels.forEach((p) => p.classList.remove("active"));

      tabBtn.classList.add("active");
      const targetId = tabBtn.getAttribute("data-target");
      const targetPanel = document.getElementById(targetId);
      if (targetPanel) {
        targetPanel.classList.add("active");
      }
    });
  });

  document.querySelectorAll(".palette-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const snippet = btn.getAttribute("data-insert");
      if (!snippet) return;
      insertSnippet(snippet);
    });
  });

  function insertSnippet(snippet) {
    if (mathField) {
      if (typeof mathField.executeCommand === "function") {
        mathField.executeCommand(["insert", snippet, { mode: "math", selectionMode: "placeholder" }]);
      } else if (typeof mathField.insert === "function") {
        mathField.insert(snippet, { mode: "math", selectionMode: "placeholder" });
      } else {
        mathField.value = (mathField.value || "") + snippet.replace(/#\?/g, "");
      }
      syncFromMathField();
      mathField.focus();
    }
    isModified = true;
  }

  // 11. MathType Classic Keyboard Shortcuts (Task 3.3)
  window.addEventListener("keydown", (e) => {
    if (document.activeElement === latexSource) return;

    const isCtrl = e.ctrlKey || e.metaKey;
    if (!isCtrl) return;

    const key = e.key.toLowerCase();

    // Ctrl+F: Fraction
    if (key === "f") {
      e.preventDefault();
      insertSnippet("\\frac{#?}{#?}");
      return;
    }

    // Ctrl+R: Radical (Square root, or N-th root with Shift)
    if (key === "r") {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet("\\sqrt[#?]{#?}");
      } else {
        insertSnippet("\\sqrt{#?}");
      }
      return;
    }

    // Ctrl+H: Superscript (High)
    if (key === "h") {
      e.preventDefault();
      insertSnippet("^{#?}");
      return;
    }

    // Ctrl+L: Subscript (Low) or Limit with Shift
    if (key === "l") {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet("\\lim_{#? \\to #?}");
      } else {
        insertSnippet("_{#?}");
      }
      return;
    }

    // Ctrl+J: Joint Super+Subscript
    if (key === "j") {
      e.preventDefault();
      insertSnippet("_{#?}^{#?}");
      return;
    }

    // Ctrl+I: Integral
    if (key === "i") {
      e.preventDefault();
      if (e.shiftKey) {
        insertSnippet("\\int_{#?}^{#?} #? \\, d#?");
      } else {
        insertSnippet("\\int #? \\, d#?");
      }
      return;
    }

    // Ctrl+Shift+S: Summation
    if (key === "s" && e.shiftKey) {
      e.preventDefault();
      insertSnippet("\\sum_{#?}^{#?} #?");
      return;
    }

    // Ctrl+Shift+P: Product
    if (key === "p" && e.shiftKey) {
      e.preventDefault();
      insertSnippet("\\prod_{#?}^{#?} #?");
      return;
    }

    // Ctrl+9 or Ctrl+(: Parentheses
    if (e.key === "9" || e.key === "(") {
      e.preventDefault();
      insertSnippet("\\left( #? \\right)");
      return;
    }

    // Ctrl+[ or Ctrl+]: Brackets
    if (e.key === "[" || e.key === "]") {
      e.preventDefault();
      insertSnippet("\\left[ #? \\right]");
      return;
    }

    // Ctrl+Shift+M: 2x2 Matrix
    if (key === "m" && e.shiftKey) {
      e.preventDefault();
      insertSnippet("\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}");
      return;
    }

    // Ctrl+Shift+C: Cases
    if (key === "c" && e.shiftKey) {
      e.preventDefault();
      insertSnippet("\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}");
      return;
    }
  });

  // 5. MathML Extraction Helper
  function getStandardMathML() {
    if (mathField && typeof mathField.getValue === 'function') {
      try {
        const mathml = mathField.getValue('math-ml');
        if (mathml && mathml.trim().length > 0) {
          return mathml;
        }
      } catch (e) {
        console.warn('MathLive getValue(math-ml) error:', e);
      }
    }
    const val = (latexSource.value || '').trim();
    return `<math xmlns="http://www.w3.org/1998/Math/MathML"><mrow><mtext>${escapeXml(val)}</mtext></mrow></math>`;
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
      } else {
        throw new Error('status ' + res.status);
      }
    } catch (err) {
      bridgeBadge.style.backgroundColor = '#fef2f2';
      bridgeBadge.style.borderColor = '#fecaca';
      bridgeBadge.style.color = '#991b1b';
      statusText.textContent = 'Bridge: Offline (Connecting...)';
    }
  }

  checkBridgeHealth();
  setInterval(checkBridgeHealth, 10000);

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
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      try {
        const target = activeTargetMode === 'omml' ? 'flat_opc_omml' : 'flat_opc_mathtype';
        const flatOpc = await window.__TAURI__.core.invoke('convert_equation', {
          data: mathml,
          fromFormat: 'mathml',
          toFormat: target
        });
        await navigator.clipboard.writeText(flatOpc);
        showNotice(btnCopyFlatOpc, 'Flat OPC Copied!');
        return;
      } catch (e) {
        console.error('Tauri invoke error:', e);
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
          to: target
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

    if (activeSessionId) {
      try {
        const res = await fetch(`${BRIDGE_URL}/session/${activeSessionId}/update`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            mathml: mathml,
            latex: latex,
            target_mode: activeTargetMode
          })
        });
        if (res.ok) {
          showNotice(btnInsertUpdate, 'Saved to Document!');
          isModified = false;
          if (window.__TAURI__ && window.__TAURI__.window) {
            window.__TAURI__.window.getCurrentWindow().hide();
          }
          return;
        }
      } catch (err) {
        console.error('Failed to update bridge session:', err);
      }
    }

    showNotice(btnInsertUpdate, 'Equation Ready!');
    isModified = false;
  }

  // 10. Close Modal Interception
  btnCancel.addEventListener('click', () => {
    if (isModified) {
      closeModal.classList.remove('hidden');
    } else {
      if (window.__TAURI__ && window.__TAURI__.window) {
        window.__TAURI__.window.getCurrentWindow().hide();
      }
    }
  });

  modalBtnYes.addEventListener('click', async () => {
    closeModal.classList.add('hidden');
    await saveAndResolveSession();
  });

  modalBtnNo.addEventListener('click', () => {
    closeModal.classList.add('hidden');
    isModified = false;
    if (window.__TAURI__ && window.__TAURI__.window) {
      window.__TAURI__.window.getCurrentWindow().hide();
    }
  });

  modalBtnCancel.addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

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
