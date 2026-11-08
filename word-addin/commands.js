// Microsoft Word Add-in - Ribbon Action Commands
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

const BRIDGE_URL = "http://127.0.0.1:17890";

Office.onReady(() => {
  // Office is ready
});

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
 * If selected text is LaTeX ($...$ or \[...\]), converts it to Word native equation.
 * If selected object is an equation, converts it to LaTeX text.
 */
async function toggleLatex(event) {
  try {
    await Word.run(async (context) => {
      const selection = context.document.getSelection();
      selection.load(["text"]);
      const ooxml = selection.getOoxml();
      await context.sync();

      const selText = (selection.text || "").trim();

      // Case A: Selected text is LaTeX ($...$ or \[...\])
      if (
        (selText.startsWith("$") && selText.endsWith("$")) ||
        (selText.startsWith("\\[") && selText.endsWith("\\]"))
      ) {
        let cleanLatex = selText;
        if (cleanLatex.startsWith("$$") && cleanLatex.endsWith("$$")) {
          cleanLatex = cleanLatex.slice(2, -2).trim();
        } else if (cleanLatex.startsWith("$") && cleanLatex.endsWith("$")) {
          cleanLatex = cleanLatex.slice(1, -1).trim();
        } else if (cleanLatex.startsWith("\\[") && cleanLatex.endsWith("\\]")) {
          cleanLatex = cleanLatex.slice(2, -2).trim();
        }

        const convRes = await fetch(`${BRIDGE_URL}/convert/snippet`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            data: cleanLatex,
            from: "latex",
            to: "flat_opc_omml"
          })
        });

        if (convRes.ok) {
          const convData = await convRes.json();
          Office.context.document.setSelectedDataAsync(
            convData.result,
            { coercionType: Office.CoercionType.Ooxml },
            () => event.completed()
          );
          return;
        }
      }

      // Case B: Selection contains an equation, toggle to LaTeX string
      const snipRes = await fetch(`${BRIDGE_URL}/convert/snippet`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          data: ooxml.value,
          from: "flat_opc",
          to: "mathml"
        })
      });

      if (snipRes.ok) {
        const snipData = await snipRes.json();
        // Insert as inline LaTeX text placeholder
        const latexText = `$${snipData.result}$`;
        selection.insertText(latexText, Word.InsertLocation.replace);
        await context.sync();
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
 * Helper to launch the desktop application via custom URI scheme or iframe ping
 */
function launchDesktopEditor(param) {
  const uri = `matheditor://open?${typeof param === "string" && param === "batch" ? "tab=batch" : `session_id=${param}`}`;
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
