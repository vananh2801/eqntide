/**
 * Ensure desktop app & bridge server is running before executing bridge requests.
 * Automatically wakes up the app via cold-start if bridge is offline.
 */
function ensureBridgeRunning(callback) {
  ajaxGet(BRIDGE_URL + "/health", function (status) {
    if (status === 200) {
      callback();
    } else {
      ColdStartDesktopEditor();
      var attempts = 0;
      var poll = setInterval(function () {
        attempts++;
        ajaxGet(BRIDGE_URL + "/health", function (s) {
          if (s === 200) {
            clearInterval(poll);
            callback();
          } else if (attempts >= 10) {
            clearInterval(poll);
            setTimeout(function () {
              alert("Math Editor is currently not running.\n\nPlease launch Math Editor from your application menu, then try again.");
            }, 50);
          }
        });
      }, 500);
    }
  });
}

// WPS Office JS Add-in: Main Controller & Ribbon Handlers
// Compatible with WPS Office Writer on Windows, Linux, and macOS
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

var BRIDGE_URL = "http://127.0.0.1:17890";
var g_ribbonUI = null;
var g_editTargetRange = null;
var g_editRangeStart = null;
var g_editRangeEnd = null;

function wpsLog(msg) {
  try {
    var text = typeof msg === "string" ? msg : JSON.stringify(msg);
    console.log("[WPS Addin] " + text);
    ajaxPost(BRIDGE_URL + "/log", text, function () {});
  } catch (_) {}
}
var g_currentSessionId = null;
var g_currentWaitXhr = null;

/**
 * Check if a given XML snippet actually contains an OMML or MathType equation.
 */
function hasEquationInXml(xmlStr) {
  if (!xmlStr || typeof xmlStr !== "string") return false;
  var lower = xmlStr.toLowerCase();
  return (
    lower.indexOf("omath") !== -1 ||
    lower.indexOf("oleobject") !== -1 ||
    lower.indexOf("equation.") !== -1 ||
    lower.indexOf("mathtype") !== -1 ||
    lower.indexOf("w:object") !== -1 ||
    lower.indexOf("w:pict") !== -1 ||
    lower.indexOf("w:drawing") !== -1 ||
    lower.indexOf("imagedata") !== -1 ||
    lower.indexOf("bindata") !== -1 ||
    lower.indexOf("appsmfcc") !== -1
  );
}

/**
 * Check if raw text looks like a LaTeX math formula.
 */
function isLatexText(text) {
  if (!text || typeof text !== "string") return false;
  var t = text.trim();
  return (
    (t.startsWith("$") && t.endsWith("$")) ||
    (t.startsWith("\\[") && t.endsWith("\\]")) ||
    t.indexOf("\\frac") !== -1 ||
    t.indexOf("\\sqrt") !== -1 ||
    t.startsWith("\\begin")
  );
}

/**
 * Cancel any ongoing session wait requests to prevent hangs or race conditions.
 */
function cancelPendingSession() {
  if (g_currentWaitXhr) {
    try {
      g_currentWaitXhr.abort();
    } catch (_) {}
    g_currentWaitXhr = null;
  }
  if (g_currentSessionId) {
    var sid = g_currentSessionId;
    g_currentSessionId = null;
    try {
      ajaxPost(BRIDGE_URL + "/session/" + sid + "/cancel", "{}", function () {});
    } catch (_) {}
  }
}

/**
 * Called when WPS loads the add-in
 */
function OnAddinLoad(ribbonUI) {
  g_ribbonUI = ribbonUI;
  console.log("Math Editor WPS JS Add-in loaded successfully.");
}

/**
 * Provide icon images for Ribbon buttons
 */
function GetImage(control) {
  return "assets/icon-32.png";
}

/**
 * Main Ribbon Action Dispatcher.
 * Uses setTimeout(..., 0) so the ribbon callback returns immediately to WPS,
 * preventing ribbon dispatcher deadlocks or freezes on Linux.
 */
function OnAction(control) {
  var controlId = control.Id;
  setTimeout(function () {
    switch (controlId) {
      case "btnInsertEquation":
        InsertEquation();
        break;
      case "btnEditEquation":
        EditEquation();
        break;
      case "btnToggleLatex":
        ToggleLatex();
        break;
      case "btnUpgradeLegacy":
        UpgradeLegacyEquations();
        break;
      case "btnBatchConverter":
        OpenBatchConverter();
        break;
      default:
        console.warn("Unknown control action:", controlId);
    }
  }, 0);
}

/**
 * 1. Insert Equation:
 * Opens a new session in desktop editor and inserts the resulting Flat OPC XML.
 */
function InsertEquation() {
  wpsLog("InsertEquation clicked");
  cancelPendingSession();
  ensureBridgeRunning(function () {
    ajaxPost(
      BRIDGE_URL + "/session/open",
      JSON.stringify({ data: "", format: "auto", target_mode: "mathtype" }),
      function (status, responseText) {
        if (status !== 200) {
          console.error("Bridge error (" + status + "): " + responseText);
          return;
        }

        var res = JSON.parse(responseText);
        var sessionId = res.session_id;
        g_currentSessionId = sessionId;

        // Bring desktop window to front
        LaunchDesktopEditor(sessionId);

        // Long-poll wait for user confirmation
        WaitSession(sessionId, function (waitData) {
          if (waitData && waitData.status === "ready" && waitData.flat_opc) {
            InsertFlatOpcToSelection(waitData.flat_opc);
          }
        });
      }
    );
  });
}

/**
 * 2. Edit Equation:
 * Extracts selected equation from WPS Selection and opens it in editor.
 * If no equation is selected, alerts the user directly without redirecting or freezing WPS.
 */
function EditEquation() {
  try {
    wpsLog("EditEquation clicked");
    var app = wps.Application;
    var selection = app ? app.Selection : null;
    if (!selection) {
      wpsLog("EditEquation: selection is null");
      alert("Không tìm thấy công thức nào được chọn!");
      return;
    }

    var selData = "";
    var targetRange = null;
    var isEquationFound = false;

    // 1. Check for OMML equation under cursor or in selection
    try {
      if (selection.OMaths && selection.OMaths.Count > 0) {
        targetRange = selection.OMaths.Item(1).Range;
        isEquationFound = true;
      }
    } catch (_) {}

    if (!targetRange) {
      try {
        if (selection.Range && selection.Range.OMaths && selection.Range.OMaths.Count > 0) {
          targetRange = selection.Range.OMaths.Item(1).Range;
          isEquationFound = true;
        }
      } catch (_) {}
    }

    // 2. Check for MathType OLE / EMF inline shape
    if (!targetRange) {
      try {
        if (selection.InlineShapes && selection.InlineShapes.Count > 0) {
          targetRange = selection.InlineShapes.Item(1).Range;
          isEquationFound = true;
        }
      } catch (_) {}
    }

    if (!targetRange) {
      try {
        if (selection.Range && selection.Range.InlineShapes && selection.Range.InlineShapes.Count > 0) {
          targetRange = selection.Range.InlineShapes.Item(1).Range;
          isEquationFound = true;
        }
      } catch (_) {}
    }

    // 3. Check for floating ShapeRange
    if (!targetRange) {
      try {
        if (selection.ShapeRange && selection.ShapeRange.Count > 0) {
          targetRange = selection.ShapeRange.Item(1).Anchor;
          isEquationFound = true;
        }
      } catch (_) {}
    }

    // 4. Check adjacent word
    if (!targetRange) {
      try {
        if (selection.Words && selection.Words.Count > 0) {
          var word = selection.Words.Item(1);
          if (word.InlineShapes && word.InlineShapes.Count > 0) {
            targetRange = word.InlineShapes.Item(1).Range;
            isEquationFound = true;
          } else if (word.OMaths && word.OMaths.Count > 0) {
            targetRange = word.OMaths.Item(1).Range;
            isEquationFound = true;
          }
        }
      } catch (_) {}
    }

    // 5. Check adjacent characters (left and right of cursor)
    if (!targetRange) {
      try {
        var doc = app.ActiveDocument;
        var start = selection.Start;
        var end = selection.End;
        if (doc && start > 0) {
          var prevChar = doc.Range(start - 1, start);
          if (prevChar.InlineShapes && prevChar.InlineShapes.Count > 0) {
            targetRange = prevChar.InlineShapes.Item(1).Range;
            isEquationFound = true;
          } else if (prevChar.OMaths && prevChar.OMaths.Count > 0) {
            targetRange = prevChar.OMaths.Item(1).Range;
            isEquationFound = true;
          }
        }
        if (!targetRange && doc && end < (doc.Content ? doc.Content.End : 999999)) {
          var nextChar = doc.Range(end, end + 1);
          if (nextChar.InlineShapes && nextChar.InlineShapes.Count > 0) {
            targetRange = nextChar.InlineShapes.Item(1).Range;
            isEquationFound = true;
          } else if (nextChar.OMaths && nextChar.OMaths.Count > 0) {
            targetRange = nextChar.OMaths.Item(1).Range;
            isEquationFound = true;
          }
        }
      } catch (_) {}
    }

    // Focus selection on the target equation range if found
    if (targetRange && typeof targetRange.Select === "function") {
      try {
        targetRange.Select();
      } catch (_) {}
    }

    // Extract WordOpenXML representation
    if (targetRange) {
      try {
        if (targetRange.WordOpenXML) {
          var xml1 = targetRange.WordOpenXML;
          if (hasEquationInXml(xml1) || isEquationFound) {
            selData = xml1;
          }
        }
      } catch (_) {}
    }

    if (!selData) {
      try {
        if (selection.WordOpenXML) {
          var xml2 = selection.WordOpenXML;
          if (hasEquationInXml(xml2) || isEquationFound) {
            selData = xml2;
          }
        }
      } catch (_) {}
    }

    if (!selData) {
      try {
        if (selection.Range && selection.Range.WordOpenXML) {
          var xml3 = selection.Range.WordOpenXML;
          if (hasEquationInXml(xml3) || isEquationFound) {
            selData = xml3;
          }
        }
      } catch (_) {}
    }

    // Fallback: check raw text if it looks like LaTeX
    if (!selData) {
      try {
        var rawText = (selection.Text || "").trim();
        if (isLatexText(rawText)) {
          selData = rawText;
        }
      } catch (_) {}
    }

    // If NO equation was selected, do NOT redirect! Show alert directly!
    if (!selData || selData.trim().length === 0) {
      wpsLog("EditEquation: No equation found in selection or adjacent text");
      alert("Không tìm thấy công thức nào được chọn!");
      return;
    }

    g_editTargetRange = targetRange;
    if (targetRange && typeof targetRange.Start === "number" && typeof targetRange.End === "number") {
      g_editRangeStart = targetRange.Start;
      g_editRangeEnd = targetRange.End;
    } else {
      g_editRangeStart = null;
      g_editRangeEnd = null;
    }
    wpsLog("EditEquation: Found equation, payload len=" + selData.length + ", sending /session/open...");
    cancelPendingSession();

    ensureBridgeRunning(function () {
      ajaxPost(
        BRIDGE_URL + "/session/open",
        JSON.stringify({
          data: selData,
          format: "auto",
          target_mode: "mathtype"
        }),
        function (status, responseText) {
          wpsLog("EditEquation: /session/open response (" + status + "): " + responseText);
          if (status !== 200) {
            console.error("Bridge error (" + status + "): " + responseText);
            alert("Lỗi kết nối tới Math Editor (" + status + ")");
            return;
          }

          var res = JSON.parse(responseText);
          var sessionId = res.session_id;
          g_currentSessionId = sessionId;

          LaunchDesktopEditor(sessionId);

          WaitSession(sessionId, function (waitData) {
            if (waitData && waitData.status === "ready" && waitData.flat_opc) {
              InsertFlatOpcToSelection(waitData.flat_opc);
            }
          });
        }
      );
    });
  } catch (err) {
    console.error("Error reading equation from selection:", err);
    alert("Không tìm thấy công thức nào được chọn!");
  }
}

/**
 * 3. Toggle LaTeX:
 * Two-way conversion between LaTeX text and native equation in WPS.
 */
function ToggleLatex() {
  try {
    var app = wps.Application;
    var selection = app.Selection;
    if (!selection) return;

    var text = (selection.Text || "").trim();

    // Case A: Selected text is LaTeX ($...$ or \[...\])
    if (
      (text.startsWith("$") && text.endsWith("$")) ||
      (text.startsWith("\\[") && text.endsWith("\\]"))
    ) {
      var cleanLatex = text;
      if (cleanLatex.startsWith("$$") && cleanLatex.endsWith("$$")) {
        cleanLatex = cleanLatex.slice(2, -2).trim();
      } else if (cleanLatex.startsWith("$") && cleanLatex.endsWith("$")) {
        cleanLatex = cleanLatex.slice(1, -1).trim();
      } else if (cleanLatex.startsWith("\\[") && cleanLatex.endsWith("\\]")) {
        cleanLatex = cleanLatex.slice(2, -2).trim();
      }

      ajaxPost(
        BRIDGE_URL + "/convert/snippet",
        JSON.stringify({
          data: cleanLatex,
          from: "latex",
          to: "flat_opc_omml"
        }),
        function (status, responseText) {
          if (status === 200) {
            var conv = JSON.parse(responseText);
            InsertFlatOpcToSelection(conv.result);
          } else {
            console.error("LaTeX conversion failed.");
          }
        }
      );
      return;
    }

    // Case B: Selection contains an equation, convert to LaTeX string
    var ooxml = "";
    try {
      if (selection.Range && selection.Range.WordOpenXML && hasEquationInXml(selection.Range.WordOpenXML)) {
        ooxml = selection.Range.WordOpenXML;
      } else if (selection.WordOpenXML && hasEquationInXml(selection.WordOpenXML)) {
        ooxml = selection.WordOpenXML;
      }
    } catch (_) {}

    if (!ooxml) {
      return;
    }

    ajaxPost(
      BRIDGE_URL + "/convert/snippet",
      JSON.stringify({
        data: ooxml,
        from: "flat_opc",
        to: "latex"
      }),
      function (status, responseText) {
        if (status === 200) {
          var snip = JSON.parse(responseText);
          if (snip && snip.result) {
            selection.Text = "$" + snip.result + "$";
          }
        }
      }
    );
  } catch (err) {
    console.error("ToggleLatex error:", err);
  }
}

/**
 * 4. Upgrade Legacy Equations in Document:
 * Utilizes WPS FullName direct disk conversion or in-memory scanning.
 */
function UpgradeLegacyEquations() {
  try {
    var app = wps.Application;
    var doc = app.ActiveDocument;

    // Feature Bonus for WPS: If document is saved on disk, offer instant high-speed conversion
    if (doc && doc.FullName && doc.FullName.toLowerCase().endsWith(".docx")) {
      var filePath = doc.FullName;
      var confirmMsg = "Document is saved at:\n" + filePath + "\n\nDo you want to run instant Rust batch conversion directly on this file?";
      if (confirm(confirmMsg)) {
        doc.Save();
        var upgradeTarget = confirm("Click [OK] to upgrade to Modern MathType 7 (OLE DSMT4 + EMF-MTEF).\n\nClick [Cancel] to upgrade to Modern Native OMML (<m:oMath>).") ? "upgrade_mathtype" : "upgrade_omml";
        ajaxPost(
          BRIDGE_URL + "/convert/document",
          JSON.stringify({
            input_path: filePath,
            target: upgradeTarget
          }),
          function (status, responseText) {
            if (status === 200) {
              var res = JSON.parse(responseText);
              alert("Instant Upgrade Succeeded!\n\nFound: " + res.found + "\nUpgraded: " + res.converted + "\nErrors: " + res.errors);
              // Reopen to display fresh upgraded equations
              try {
                doc.Close(false);
                app.Documents.Open(filePath);
              } catch (_) {}
            } else {
              alert("Document conversion failed: " + responseText);
            }
          }
        );
        return;
      }
    }

    // Fallback: Open Taskpane for in-memory / selection upgrade
    OpenTaskpane();
  } catch (err) {
    OpenTaskpane();
  }
}

/**
 * 5. Open Desktop Batch Converter
 */
function OpenBatchConverter() {
  LaunchDesktopEditor("batch");
}

/**
 * Helper: Insert Flat OPC XML into active WPS selection
 */
function InsertFlatOpcToSelection(flatOpcXml) {
  try {
    var app = wps.Application;
    var doc = app ? app.ActiveDocument : null;
    var sel = app ? app.Selection : null;
    var range = null;

    wpsLog('InsertFlatOpcToSelection: start. editRangeStart=' + g_editRangeStart + ', editRangeEnd=' + g_editRangeEnd);

    if (doc && g_editRangeStart !== null && g_editRangeEnd !== null) {
      try {
        var oldRange = doc.Range(g_editRangeStart, g_editRangeEnd);
        var insertPos = oldRange.Start;

        // 1. Delete old InlineShapes cleanly from WPS shape table to avoid orphaned handles
        if (oldRange.InlineShapes && oldRange.InlineShapes.Count > 0) {
          for (var i = oldRange.InlineShapes.Count; i >= 1; i--) {
            try {
              oldRange.InlineShapes.Item(i).Delete();
            } catch (_) {}
          }
        }

        // 2. Delete old range text
        oldRange.Delete();

        // 3. Create a clean insertion point at insertPos
        range = doc.Range(insertPos, insertPos);
        wpsLog('InsertFlatOpcToSelection: deleted old equation at pos ' + insertPos);
      } catch (e) {
        wpsLog('InsertFlatOpcToSelection: error cleaning old equation: ' + e);
      }
    }
    g_editRangeStart = null;
    g_editRangeEnd = null;
    g_editTargetRange = null;

    if (!range) {
      range = sel ? sel.Range : null;
    }
    if (!range) return;

    if (typeof range.InsertXML === 'function') {
      range.InsertXML(flatOpcXml);
    } else {
      // Fallback: clipboard paste
      var textarea = document.createElement('textarea');
      textarea.value = flatOpcXml;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      range.Paste();
    }

    // 4. Clean up trailing paragraph break (\r or \n) created by <w:p> wrapper in Flat OPC
    // This maintains 100% inline equation placement and eliminates the newline jump!
    try {
      if (doc && range && typeof range.End === 'number' && range.End > 0) {
        var endCharRange = doc.Range(range.End - 1, range.End);
        if (endCharRange && (endCharRange.Text === "\r" || endCharRange.Text === "\n")) {
          endCharRange.Delete();
          wpsLog('InsertFlatOpcToSelection: removed trailing paragraph break from <w:p>');
        }
      }
    } catch (e) {
      wpsLog('InsertFlatOpcToSelection: error checking trailing break: ' + e);
    }

    // 5. Force WPS canvas layout refresh for all inline shapes in the document
    // This marks every shape dirty and forces WPS to recalculate and repaint subsequent equations!
    try {
      if (doc && doc.InlineShapes) {
        var shpCount = doc.InlineShapes.Count;
        for (var sIdx = 1; sIdx <= shpCount; sIdx++) {
          try {
            var shp = doc.InlineShapes.Item(sIdx);
            var curW = shp.Width;
            shp.Width = curW;
          } catch (_) {}
        }
      }
    } catch (_) {}

    try {
      if (doc && typeof doc.Repaginate === 'function') {
        doc.Repaginate();
      }
      var win = app.ActiveWindow;
      if (win && win.View) {
        var curViewType = win.View.Type;
        win.View.Type = (curViewType === 3 ? 1 : 3);
        win.View.Type = curViewType;
      }
      if (win && win.ActivePane && win.ActivePane.View && win.ActivePane.View.Zoom) {
        var curZoom = win.ActivePane.View.Zoom.Percentage;
        win.ActivePane.View.Zoom.Percentage = curZoom + 1;
        win.ActivePane.View.Zoom.Percentage = curZoom;
      }
      if (app && typeof app.ScreenRefresh === 'function') {
        app.ScreenRefresh();
      }
    } catch (_) {}

    // 6. Finally, collapse cursor to end of equation and reset font baseline formatting
    // so subsequent typing never inherits negative position or subscript drop
    try {
      range.Collapse(0); // 0 = wdCollapseEnd
      range.Select();
      var curSel = app.Selection;
      if (curSel) {
        if (curSel.Font) {
          if (typeof curSel.Font.Reset === 'function') {
            try { curSel.Font.Reset(); } catch (_) {}
          }
          curSel.Font.Position = 0;
          curSel.Font.Subscript = false;
          curSel.Font.Superscript = false;
        }
      }
    } catch (_) {}

    wpsLog('InsertFlatOpcToSelection: finished successfully.');
  } catch (err) {
    wpsLog('InsertFlatOpcToSelection: unhandled error: ' + err);
    console.error('InsertFlatOpcToSelection error:', err);
  }
}

/**
 * Helper: Open Taskpane in WPS
 */
function OpenTaskpane() {
  try {
    if (typeof wps.CreateTaskpane === "function") {
      var tp = wps.CreateTaskpane("taskpane.html");
      tp.DockPosition = 2; // Right
      tp.Show();
    } else {
      window.open("taskpane.html", "_blank", "width=380,height=600");
    }
  } catch (e) {
    window.open("taskpane.html", "_blank", "width=380,height=600");
  }
}

/**
 * Helper: Bring running desktop editor window to front via local HTTP bridge.
 * Instant, non-blocking, and never spawns redundant background processes.
 */
function LaunchDesktopEditor(param) {
  var sessionId = param === "batch" ? "" : (param || "");
  ajaxPost(
    BRIDGE_URL + "/window/show",
    JSON.stringify({ session_id: sessionId, tab: param === "batch" ? "batch" : "editor" }),
    function () {}
  );
}

/**
 * Helper: Cold-start desktop editor if bridge daemon is not running.
 */
function ColdStartDesktopEditor() {
  var binPath = "/home/vananh2801/Documents/mathtype-convert/omml-converter-rust/src-tauri/target/release/omml-editor-desktop";
  var uri = "matheditor://open";

  if (wps.OAAssist && typeof wps.OAAssist.ShellExecute === "function") {
    try {
      wps.OAAssist.ShellExecute(binPath, uri);
      return;
    } catch (_) {}
  }

  try {
    var a = document.createElement("a");
    a.href = uri;
    a.style.display = "none";
    document.body.appendChild(a);
    a.click();
    setTimeout(function () {
      try {
        document.body.removeChild(a);
      } catch (_) {}
    }, 1000);
  } catch (_) {}
}

/**
 * Helper: Long-poll wait for session result
 */
function WaitSession(sessionId, callback) {
  g_currentSessionId = sessionId;
  if (g_currentWaitXhr) {
    try {
      g_currentWaitXhr.abort();
    } catch (_) {}
  }
  g_currentWaitXhr = ajaxGet(BRIDGE_URL + "/session/" + sessionId + "/wait?timeout=300", function (status, responseText) {
    g_currentWaitXhr = null;
    if (status === 200) {
      try {
        var json = JSON.parse(responseText);
        callback(json);
      } catch (e) {
        console.error("WaitSession parse error:", e);
      }
    }
  });
}

/**
 * Standard AJAX GET
 */
function ajaxGet(url, callback) {
  var xhr = new XMLHttpRequest();
  xhr.open("GET", url, true);
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      callback(xhr.status, xhr.responseText);
    }
  };
  xhr.send();
  return xhr;
}

/**
 * Standard AJAX POST
 */
function ajaxPost(url, payload, callback) {
  var xhr = new XMLHttpRequest();
  xhr.open("POST", url, true);
  xhr.setRequestHeader("Content-Type", "application/json");
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      callback(xhr.status, xhr.responseText);
    }
  };
  xhr.send(payload);
  return xhr;
}
