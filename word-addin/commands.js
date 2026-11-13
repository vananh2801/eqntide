// Microsoft Word Add-in - Ribbon Action Commands
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

const BRIDGE_URL = "http://127.0.0.1:17890";

Office.onReady(() => {
  // Office is ready
});

/**
 * Helper: Find all LaTeX math formulas ($...$, $$...$$, \[...\], \(...\)) within text.
 */
function findAllLatexMatches(text) {
  if (!text || typeof text !== "string") return [];
  const regex = /(\$\$[\s\S]*?\$\$)|(\\\[[\s\S]*?\\\])|(\\\([\s\S]*?\\\))|(\$(?:\\\$|[^\$\r\n])+?\$)/g;
  const matches = [];
  let m;
  while ((m = regex.exec(text)) !== null) {
    const raw = m[0];
    let isDisplay = false;
    let inner = raw;
    if (raw.startsWith("$$") && raw.endsWith("$$") && raw.length >= 4) {
      isDisplay = true;
      inner = raw.slice(2, -2).trim();
    } else if (raw.startsWith("\\[") && raw.endsWith("\\]") && raw.length >= 4) {
      isDisplay = true;
      inner = raw.slice(2, -2).trim();
    } else if (raw.startsWith("\\(") && raw.endsWith("\\)") && raw.length >= 4) {
      isDisplay = false;
      inner = raw.slice(2, -2).trim();
    } else if (raw.startsWith("$") && raw.endsWith("$") && raw.length >= 2) {
      isDisplay = false;
      inner = raw.slice(1, -1).trim();
    }
    matches.push({
      start: m.index,
      end: m.index + raw.length,
      raw: raw,
      latex: inner,
      isDisplay: isDisplay
    });
  }
  return matches;
}

/**
 * 1. Insert Equation at cursor:
 * Opens a new session in desktop editor, waits for user confirmation,
 * then inserts the resulting Flat OPC OOXML into Word.
 */
async function insertEquation(event) {
  try {
    const openRes = await fetch(`${BRIDGE_URL}/session/open`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        data: "",
        format: "auto",
        target_mode: "omml"
      })
    });

    if (!openRes.ok) {
      throw new Error(`Failed to open session: ${openRes.status}`);
    }

    const { session_id } = await openRes.json();

    // Trigger local application launch via custom URI scheme
    launchDesktopEditor(session_id);

    // Long-poll wait until user clicks Insert/Update or closes editor
    const waitRes = await fetch(`${BRIDGE_URL}/session/${session_id}/wait?timeout=300`);
    if (waitRes.ok) {
      const waitData = await waitRes.json();
      if (waitData.status === "ready" && waitData.flat_opc) {
        Office.context.document.setSelectedDataAsync(
          waitData.flat_opc,
          { coercionType: Office.CoercionType.Ooxml },
          () => event.completed()
        );
        return;
      }
    }
  } catch (err) {
    console.error("Error in insertEquation:", err);
  } finally {
    event.completed();
  }
}

/**
 * 2. Edit selected equation:
 * Reads selection OOXML, opens it in desktop editor for editing,
 * and updates document in-place.
 */
async function editEquation(event) {
  try {
    await Word.run(async (context) => {
      const selection = context.document.getSelection();
      const ooxml = selection.getOoxml();
      await context.sync();

      const openRes = await fetch(`${BRIDGE_URL}/session/open`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          data: ooxml.value,
          format: "flat_opc",
          target_mode: "omml"
        })
      });

      if (!openRes.ok) {
        throw new Error(`Failed to open session: ${openRes.status}`);
      }

      const { session_id } = await openRes.json();
      launchDesktopEditor(session_id);

      const waitRes = await fetch(`${BRIDGE_URL}/session/${session_id}/wait?timeout=300`);
      if (waitRes.ok) {
        const waitData = await waitRes.json();
        if (waitData.status === "ready" && waitData.flat_opc) {
          Office.context.document.setSelectedDataAsync(
            waitData.flat_opc,
            { coercionType: Office.CoercionType.Ooxml },
            () => event.completed()
          );
          return;
        }
      }
    });
  } catch (err) {
    console.error("Error in editEquation:", err);
  } finally {
    event.completed();
  }
}

/**
 * 3. Toggle LaTeX:
 * Two-way toggle between LaTeX text and Word native equations (OMML).
 * Supports:
 * - Highlighting an entire paragraph / range with multiple LaTeX formulas -> batch converts to equations in-place
 * - Single selection containing LaTeX ($...$ or \[...\]) -> converts to equation
 * - Selected equation -> converts to LaTeX $...$ text string
 */
async function toggleLatex(event) {
  try {
    await Word.run(async (context) => {
      const selection = context.document.getSelection();
      selection.load(["text"]);
      const ooxml = selection.getOoxml();
      await context.sync();

      const selText = (selection.text || "").trim();
      const latexMatches = findAllLatexMatches(selText);

      // Case A: Selection contains one or more LaTeX formulas
      if (latexMatches.length > 0) {
        for (let i = 0; i < latexMatches.length; i++) {
          const match = latexMatches[i];
          try {
            const convRes = await fetch(`${BRIDGE_URL}/convert/snippet`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                data: match.latex,
                from: "latex",
                to: "flat_opc_omml"
              })
            });

            if (convRes.ok) {
              const convData = await convRes.json();
              if (convData.result) {
                // Search match.raw within the selection range
                const searchResults = selection.search(match.raw, { matchCase: true });
                searchResults.load("items");
                await context.sync();

                if (searchResults.items.length > 0) {
                  searchResults.items[0].insertOoxml(convData.result, Word.InsertLocation.replace);
                  await context.sync();
                }
              }
            }
          } catch (e) {
            console.warn(`Failed to convert LaTeX formula "${match.raw}":`, e);
          }
        }
        return;
      }

      // Case B: Selection contains an equation, toggle to LaTeX string
      const snipRes = await fetch(`${BRIDGE_URL}/convert/snippet`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          data: ooxml.value,
          from: "flat_opc",
          to: "latex"
        })
      });

      if (snipRes.ok) {
        const snipData = await snipRes.json();
        if (snipData && snipData.result) {
          const clean = snipData.result.trim();
          const latexText = `$${clean}$`;
          selection.insertText(latexText, Word.InsertLocation.replace);
          await context.sync();
        }
      }
    });
  } catch (err) {
    console.error("Error in toggleLatex:", err);
  } finally {
    event.completed();
  }
}

/**
 * 4. Open Standalone Batch Document Converter
 */
function openBatchConverter(event) {
  try {
    launchDesktopEditor("batch");
  } finally {
    event.completed();
  }
}

/**
 * Helper to launch the EqnTide desktop application through its registered URI scheme.
 */
function launchDesktopEditor(param) {
  const uri = `eqntide://open?${typeof param === "string" && param === "batch" ? "tab=batch" : `session_id=${param}`}`;
  const iframe = document.createElement("iframe");
  iframe.style.display = "none";
  iframe.src = uri;
  document.body.appendChild(iframe);
  setTimeout(() => {
    try {
      document.body.removeChild(iframe);
    } catch (_) {}
  }, 2000);
}

// Register command actions with Office.js runtime
Office.actions.associate("insertEquation", insertEquation);
Office.actions.associate("editEquation", editEquation);
Office.actions.associate("toggleLatex", toggleLatex);
Office.actions.associate("openBatchConverter", openBatchConverter);
