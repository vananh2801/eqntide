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
          } else if (attempts >= 30) {
            clearInterval(poll);
            setTimeout(function () {
              alert("EqnTide is currently not running.\n\nPlease launch EqnTide from your application menu, then try again.");
            }, 50);
          }
        });
      }, 500);
    }
  });
}

// WPS Office JS Add-in: Main Controller & Ribbon Handlers
// Compatible with WPS Office Writer on Windows, Linux, and macOS
// Connected to the EqnTide local bridge (http://127.0.0.1:17890)

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
    (t.startsWith("\\(") && t.endsWith("\\)")) ||
    t.indexOf("\\frac") !== -1 ||
    t.indexOf("\\dfrac") !== -1 ||
    t.indexOf("\\sqrt") !== -1 ||
    t.startsWith("\\begin") ||
    t.indexOf("\\sum") !== -1 ||
    t.indexOf("\\int") !== -1 ||
    t.indexOf("\\alpha") !== -1 ||
    t.indexOf("\\beta") !== -1 ||
    t.indexOf("\\gamma") !== -1 ||
    t.indexOf("\\times") !== -1 ||
    t.indexOf("\\pm") !== -1 ||
    t.indexOf("\\le") !== -1 ||
    t.indexOf("\\ge") !== -1 ||
    t.indexOf("\\ne") !== -1 ||
    (t.indexOf("=") !== -1 && (t.indexOf("+") !== -1 || t.indexOf("-") !== -1 || t.indexOf("^") !== -1 || t.indexOf("_") !== -1))
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

var g_eventListenerRegistered = false;

var g_equationTabTimer = null;
var g_lastEquationTabSelection = null;
var g_activatingEquationTab = false;
function selectedEquationPicture(selection) {
  if (!selection || selection.Start === selection.End) return false;
  var shapes = selection.InlineShapes;
  if (!shapes || shapes.Count === 0) shapes = selection.Range && selection.Range.InlineShapes;
  if (!shapes || shapes.Count !== 1) return false;
  var picture = shapes.Item(1);
  if (!picture || !picture.Range || selection.Start !== picture.Range.Start ||
      selection.End !== picture.Range.End) return false;
  try { if (picture.AlternativeText === "EqnTide equation") return true; } catch (_) {}
  try {
    if (/^(Equation\.|MathType)/i.test(picture.OLEFormat.ProgID)) return true;
  } catch (_) {}
  var xml = picture.Range.WordOpenXML || "";
  if (/ProgID\s*=\s*["'](?:Equation\.|MathType)/i.test(xml)) return true;
  // Existing AddPicture equations have no OLE object: identify their embedded
  // MathType comment. Do not treat every EMF or every drawing as an equation.
  var binary = /<(?:[\w.-]+:)?binaryData\b[^>]*>([\s\S]*?)<\/(?:[\w.-]+:)?binaryData\s*>/g;
  var match;
  while ((match = binary.exec(xml)) !== null) {
    try {
      if (atob(match[1].replace(/\s/g, "")).indexOf("AppsMFCC") >= 0) return true;
    } catch (_) {}
  }
  return false;
}

function equationTabSelectionKey(app, selection) {
  var doc = app.ActiveDocument;
  if (!doc || !selection) return null;
  return String(doc.FullName || doc.Name) + ":" + selection.Start + ":" + selection.End;
}

function scheduleEquationTab(selection) {
  if (g_activatingEquationTab) return;
  if (g_equationTabTimer !== null) clearTimeout(g_equationTabTimer);
  g_equationTabTimer = null;
  try {
    var app = wps.Application;
    selection = selection || app.Selection;
    var key = equationTabSelectionKey(app, selection);
    if (key !== g_lastEquationTabSelection) g_lastEquationTabSelection = null;
    if (!key || !selection || selection.Start === selection.End) return;
    if (key === g_lastEquationTabSelection) return;
    g_equationTabTimer = setTimeout(function () {
      g_equationTabTimer = null;
      try {
        var currentApp = wps.Application;
        var current = currentApp.Selection;
        if (equationTabSelectionKey(currentApp, current) !== key ||
            !selectedEquationPicture(current)) return;
        var ribbon = g_ribbonUI || currentApp.ribbonUI;
        if (!ribbon || typeof ribbon.ActivateTab !== "function") return;
        g_lastEquationTabSelection = key;
        g_activatingEquationTab = true;
        try { ribbon.ActivateTab("TabEqnTide"); }
        finally { g_activatingEquationTab = false; }
      } catch (_) { g_lastEquationTabSelection = null; }
    }, 60);
  } catch (_) {}
}

function onWindowSelectionChange(Sel) {
  scheduleEquationTab(Sel);
  try {
    if (!Sel) {
      Sel = (wps.Application && wps.Application.Selection) ? wps.Application.Selection : null;
    }
    if (!Sel) return;

    var isCollapsed = false;
    try {
      if (typeof Sel.Type === "number") {
        isCollapsed = (Sel.Type === 1); // wdSelectionIP
      } else {
        isCollapsed = (Sel.Start === Sel.End);
      }
    } catch (_) {
      isCollapsed = (Sel.Start === Sel.End);
    }

    if (isCollapsed && Sel.Font) {
      if (typeof Sel.Font.Position === "number" && Sel.Font.Position !== 0) {
        Sel.Font.Position = 0;
        Sel.Font.Subscript = false;
        Sel.Font.Superscript = false;
      }

      // Fix a plain-text character immediately before the cursor that leaked a
      // negative baseline from an adjacent equation run (e.g. after typing,
      // clicking, or moving with arrow keys). Skip actual equations/shapes.
      try {
        var activeDoc = wps.Application ? wps.Application.ActiveDocument : null;
        var curPos = (typeof Sel.Start === "number") ? Sel.Start : null;
        if (activeDoc && curPos !== null && curPos > 0) {
          var prevChar = activeDoc.Range(curPos - 1, curPos);
          var isEquationOrShape =
            (prevChar.InlineShapes && prevChar.InlineShapes.Count > 0) ||
            (prevChar.OMaths && prevChar.OMaths.Count > 0);
          if (!isEquationOrShape && prevChar.Font && prevChar.Font.Position !== 0) {
            prevChar.Font.Position = 0;
            prevChar.Font.Subscript = false;
            prevChar.Font.Superscript = false;
          }
        }
      } catch (_) {}

      try {
        var para = Sel.Paragraphs.Item(1);
        if (para && para.Range) {
          var pEnd = para.Range.End;
          if (pEnd > 0) {
            var activeDoc2 = wps.Application ? wps.Application.ActiveDocument : null;
            if (activeDoc2) {
              var pMark = activeDoc2.Range(pEnd - 1, pEnd);
              if (pMark && pMark.Font && pMark.Font.Position !== 0) {
                pMark.Font.Position = 0;
                pMark.Font.Subscript = false;
                pMark.Font.Superscript = false;
              }
            }
          }
        }
      } catch (_) {}
    }
  } catch (_) {}
}

function onDocumentChange() {
  try {
    var app = wps.Application;
    if (!app || !app.Selection) return;
    var sel = app.Selection;
    if (sel.Start !== sel.End) return;

    // 1. Reset the pending (not-yet-typed) character format at the cursor,
    // so the NEXT character typed does not inherit a leaked baseline.
    if (sel.Font && sel.Font.Position !== 0) {
      sel.Font.Position = 0;
      sel.Font.Subscript = false;
      sel.Font.Superscript = false;
    }

    // 2. Fix the character ALREADY typed immediately before the cursor.
    // DocumentChange fires AFTER the new character has been inserted, so by the
    // time we get here that character may have already inherited a negative
    // w:position from an adjacent equation run. Setting Selection.Font above only
    // affects future typing, not text that already exists, so we must patch it
    // directly here. Skip if it is part of an equation/shape (legitimate use).
    var doc = app.ActiveDocument;
    var pos = sel.Start;
    if (doc && pos > 0) {
      try {
        var prevChar = doc.Range(pos - 1, pos);
        var isEquationOrShape =
          (prevChar.InlineShapes && prevChar.InlineShapes.Count > 0) ||
          (prevChar.OMaths && prevChar.OMaths.Count > 0);
        if (!isEquationOrShape && prevChar.Font && prevChar.Font.Position !== 0) {
          prevChar.Font.Position = 0;
          prevChar.Font.Subscript = false;
          prevChar.Font.Superscript = false;
        }
      } catch (_) {}
    }
  } catch (_) {}
}

function initAppEventListeners() {
  if (g_eventListenerRegistered) return;
  try {
    var eventApi = wps.ApiEvent || (wps.Application && wps.Application.ApiEvent);
    if (eventApi && typeof eventApi.AddApiEventListener === "function") {
      eventApi.AddApiEventListener("WindowSelectionChange", onWindowSelectionChange);
      eventApi.AddApiEventListener("DocumentChange", onDocumentChange);
      g_eventListenerRegistered = true;
      wpsLog("Attached WindowSelectionChange and DocumentChange event listeners");
    }
  } catch (e) {
    wpsLog("initAppEventListeners error: " + e);
  }
}

/**
 * Called when WPS loads the add-in
 */
function OnAddinLoad(ribbonUI) {
  g_ribbonUI = ribbonUI;
  console.log("EqnTide WPS JS Add-in loaded successfully.");
  initAppEventListeners();
  scheduleEquationTab();
}

try {
  setTimeout(initAppEventListeners, 200);
} catch (_) {}


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
  initAppEventListeners();
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

function getDocumentFontSize(range) {
  var size = 12.0;
  try {
    if (range && range.Font && range.Font.Size > 0) {
      size = Number(range.Font.Size);
    } else {
      var app = wps.WpsApplication();
      if (app && app.Selection && app.Selection.Font && app.Selection.Font.Size > 0) {
        size = Number(app.Selection.Font.Size);
      } else if (app && app.ActiveDocument && app.ActiveDocument.ActiveWindow && app.ActiveDocument.ActiveWindow.Selection && app.ActiveDocument.ActiveWindow.Selection.Font && app.ActiveDocument.ActiveWindow.Selection.Font.Size > 0) {
        size = Number(app.ActiveDocument.ActiveWindow.Selection.Font.Size);
      }
    }
  } catch (e) {
    wpsLog("getDocumentFontSize error: " + e);
  }
  return (size > 0 && isFinite(size)) ? size : 12.0;
}

function InsertEquation() {
  wpsLog("InsertEquation clicked");
  cancelPendingSession();
  var docFontSize = getDocumentFontSize();
  ensureBridgeRunning(function () {
    ajaxPost(
      BRIDGE_URL + "/session/open",
      JSON.stringify({ data: "", format: "auto", target_mode: "mathtype", font_pt: docFontSize }),
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
          HandleSessionResult(waitData);
        });
      }
    );
  });
}

/**
 * Detect equation (OMML, MathType OLE, or EMF inline shape) in selection or adjacent to cursor.
 */
function findEquationTarget(selection, doc) {
  if (!selection) return null;
  var targetRange = null;

  // 1. Check selection.OMaths
  try {
    if (selection.OMaths && selection.OMaths.Count > 0) {
      targetRange = selection.OMaths.Item(1).Range;
    }
  } catch (_) {}

  // 2. Check selection.Range.OMaths
  if (!targetRange) {
    try {
      if (selection.Range && selection.Range.OMaths && selection.Range.OMaths.Count > 0) {
        targetRange = selection.Range.OMaths.Item(1).Range;
      }
    } catch (_) {}
  }

  // 3. Check selection.InlineShapes
  if (!targetRange) {
    try {
      if (selection.InlineShapes && selection.InlineShapes.Count > 0) {
        targetRange = selection.InlineShapes.Item(1).Range;
      }
    } catch (_) {}
  }

  // 4. Check selection.Range.InlineShapes
  if (!targetRange) {
    try {
      if (selection.Range && selection.Range.InlineShapes && selection.Range.InlineShapes.Count > 0) {
        targetRange = selection.Range.InlineShapes.Item(1).Range;
      }
    } catch (_) {}
  }

  // 5. Check floating ShapeRange
  if (!targetRange) {
    try {
      if (selection.ShapeRange && selection.ShapeRange.Count > 0) {
        targetRange = selection.ShapeRange.Item(1).Anchor;
      }
    } catch (_) {}
  }

  // 6. Check adjacent word
  if (!targetRange) {
    try {
      if (selection.Words && selection.Words.Count > 0) {
        var word = selection.Words.Item(1);
        if (word.InlineShapes && word.InlineShapes.Count > 0) {
          targetRange = word.InlineShapes.Item(1).Range;
        } else if (word.OMaths && word.OMaths.Count > 0) {
          targetRange = word.OMaths.Item(1).Range;
        }
      }
    } catch (_) {}
  }

  // 7. Check adjacent characters (left and right of cursor)
  if (!targetRange && doc) {
    try {
      var start = selection.Start;
      var end = selection.End;
      if (start > 0) {
        var prevChar = doc.Range(start - 1, start);
        if (prevChar.InlineShapes && prevChar.InlineShapes.Count > 0) {
          targetRange = prevChar.InlineShapes.Item(1).Range;
        } else if (prevChar.OMaths && prevChar.OMaths.Count > 0) {
          targetRange = prevChar.OMaths.Item(1).Range;
        } else if (prevChar.Text === "‌" && start > 1) {
          var beforeZwnj = doc.Range(start - 2, start - 1);
          if (beforeZwnj.InlineShapes && beforeZwnj.InlineShapes.Count > 0) {
            targetRange = beforeZwnj.InlineShapes.Item(1).Range;
          } else if (beforeZwnj.OMaths && beforeZwnj.OMaths.Count > 0) {
            targetRange = beforeZwnj.OMaths.Item(1).Range;
          }
        }
      }
      if (!targetRange && end < (doc.Content ? doc.Content.End : 999999)) {
        var nextChar = doc.Range(end, end + 1);
        if (nextChar.InlineShapes && nextChar.InlineShapes.Count > 0) {
          targetRange = nextChar.InlineShapes.Item(1).Range;
        } else if (nextChar.OMaths && nextChar.OMaths.Count > 0) {
          targetRange = nextChar.OMaths.Item(1).Range;
        }
      }
    } catch (_) {}
  }

  return targetRange;
}

/**
 * Find enclosing LaTeX formula delimiters ($...$, $$...$$, \\[...\\], \\(...\\)) around relPos in paragraph text.
 */
function findEnclosingLatexInPara(text, relPos) {
  if (!text || typeof text !== "string") return null;

  var patterns = [
    { regex: /\$\$[\s\S]*?\$\$/g, display: true },
    { regex: /\\\[[\s\S]*?\\\]/g, display: true },
    { regex: /\\\([\s\S]*?\\\)/g, display: false },
    { regex: /\$(?:\\\$|[^\$\r\n])+\$/g, display: false }
  ];

  for (var i = 0; i < patterns.length; i++) {
    var p = patterns[i];
    var match;
    p.regex.lastIndex = 0;
    while ((match = p.regex.exec(text)) !== null) {
      var start = match.index;
      var end = match.index + match[0].length;
      if (relPos >= start && relPos <= end) {
        return {
          start: start,
          end: end,
          text: match[0],
          isDisplay: p.display
        };
      }
    }
  }
  return null;
}

/**
 * Find all LaTeX math formulas ($...$, $$...$$, \[...\], \(...\)) within text.
 * Returns array of { start, end, raw, latex, isDisplay } sorted by start ascending.
 */
function findAllLatexMatches(text) {
  if (!text || typeof text !== "string") return [];
  var regex = /(\$\$[\s\S]*?\$\$)|(\\\[[\s\S]*?\\\])|(\\\([\s\S]*?\\\))|(\$(?:\\\$|[^\$\r\n])+?\$)/g;
  var matches = [];
  var m;
  while ((m = regex.exec(text)) !== null) {
    var raw = m[0];
    var isDisplay = false;
    var inner = raw;
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
    var doc = app.ActiveDocument;
    var targetRange = findEquationTarget(selection, doc);
    var isEquationFound = (targetRange !== null);

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
      var tStart = targetRange.Start;
      var tEnd = targetRange.End;
      // If targetRange contains an InlineShape, pin range strictly to 1 character (do NOT delete adjacent text)
      if (isEquationFound && (tEnd - tStart > 1)) {
        wpsLog("EditEquation: clamped range from [" + tStart + ".." + tEnd + "] to [" + tStart + ".." + (tStart + 1) + "]");
        tEnd = tStart + 1;
      }
      // Check if immediate next character is U+200C (reset anchor), include in deletion (loop for all adjacent)
      try {
        var doc = app.ActiveDocument;
        while (doc && tEnd < (doc.Content ? doc.Content.End : 999999)) {
          var nextChar = doc.Range(tEnd, tEnd + 1);
          if (nextChar && nextChar.Text === "‌") {
            tEnd = tEnd + 1;
            wpsLog("EditEquation: included adjacent U+200C in editRange");
          } else {
            break;
          }
        }
      } catch (_) {}
      g_editRangeStart = tStart;
      g_editRangeEnd = tEnd;
      wpsLog("EditEquation: pinned editRange [" + g_editRangeStart + ".." + g_editRangeEnd + "]");
    } else {
      g_editRangeStart = null;
      g_editRangeEnd = null;
    }
    var docFontSize = getDocumentFontSize(targetRange);
    wpsLog("EditEquation: Found equation, payload len=" + selData.length + ", font_pt=" + docFontSize + ", sending /session/open...");
    cancelPendingSession();

    ensureBridgeRunning(function () {
      ajaxPost(
        BRIDGE_URL + "/session/open",
        JSON.stringify({
          data: selData,
          format: "auto",
          target_mode: "mathtype",
          font_pt: docFontSize
        }),
        function (status, responseText) {
          wpsLog("EditEquation: /session/open response (" + status + "): " + responseText);
          if (status !== 200) {
            console.error("Bridge error (" + status + "): " + responseText);
            alert("Lỗi kết nối tới EqnTide (" + status + ")");
            return;
          }

          var res = JSON.parse(responseText);
          var sessionId = res.session_id;
          g_currentSessionId = sessionId;

          LaunchDesktopEditor(sessionId);

          WaitSession(sessionId, function (waitData) {
            HandleSessionResult(waitData);
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
 * Insert vector EMF picture with embedded MTEF at specified doc range.
 * Applies baseline alignment (Font.Position), sanitizes paragraph mark,
 * and optionally places cursor after shape.
 */
function insertMathTypeAtDocRange(doc, targetRange, waitData, shouldMoveCursor) {
  var packageXml = waitData && (waitData.flat_opc || waitData.result);
  if (!doc || !targetRange || !packageXml || !/ProgID=["']Equation\./i.test(packageXml)) return null;
  var insertPos = targetRange.Start;
  var replaced = targetRange.End - insertPos;
  var originalEnd = doc.Range().End;
  var suffix = doc.Range(targetRange.End, Math.min(targetRange.End + 32, originalEnd)).Text;
  targetRange.InsertXML(packageXml);
  var newEnd = doc.Range().End;
  var insertedEnd = insertPos + newEnd - originalEnd + replaced;
  if (insertedEnd > insertPos &&
      doc.Range(insertedEnd, Math.min(insertedEnd + suffix.length, newEnd)).Text === suffix) {
    for (var breaks = 0; breaks < 2; breaks++) {
      var trailing = doc.Range(insertedEnd - 1, insertedEnd);
      if (trailing.Text !== "\r" && trailing.Text !== "\n") break;
      trailing.Delete();
      insertedEnd--;
    }
  }
  var range = doc.Range(insertPos, insertedEnd);
  var pic = null;
  if (range.InlineShapes) {
    for (var index = 1; index <= range.InlineShapes.Count; index++) {
      var candidate = range.InlineShapes.Item(index);
      if (candidate.Range.Start === insertPos) { pic = candidate; break; }
    }
  }
  if (!pic) return null;
  try { pic.AlternativeText = "EqnTide equation"; } catch (_) {}

  try {
    if (typeof waitData.width_pt === "number" && waitData.width_pt > 0) {
      pic.Width = waitData.width_pt;
    }
    if (typeof waitData.height_pt === "number" && waitData.height_pt > 0) {
      pic.Height = waitData.height_pt;
    }

    var docFontSize = 12.0;
    if (typeof waitData.font_pt === "number" && waitData.font_pt > 0) {
      docFontSize = waitData.font_pt;
    } else {
      try {
        if (targetRange && targetRange.Font && targetRange.Font.Size > 0) {
          docFontSize = targetRange.Font.Size;
        } else if (range && range.Font && range.Font.Size > 0) {
          docFontSize = range.Font.Size;
        }
      } catch (_) {}
    }

    var depthPt = (typeof waitData.depth_pt === "number") ? waitData.depth_pt : 0;
    var fontBaselineOffset = docFontSize * 0.045; // ~0.54 pt for 12pt font
    var pos = -Number((depthPt + fontBaselineOffset).toFixed(1));

    var actualStart = (pic.Range && typeof pic.Range.Start === "number") ? pic.Range.Start : insertPos;
    var shapeEnd = actualStart + 1;

    // Set baseline position strictly on the 1-character shape range
    var shapeRange = doc.Range(actualStart, shapeEnd);
    if (shapeRange && shapeRange.Font) {
      shapeRange.Font.Position = pos;
      wpsLog("insertMathTypeAtDocRange: set baseline Font.Position=" + pos + " on shapeRange [" + actualStart + ".." + shapeEnd + "] (depthPt=" + depthPt + ")");
    }
    try {
      if (pic && pic.Range && pic.Range.Font) {
        pic.Range.Font.Position = pos;
      }
    } catch (_) {}

    // Sanitize paragraph mark
    try {
      var para = shapeRange.Paragraphs.Item(1);
      if (para && para.Range) {
        var pEnd = para.Range.End;
        if (pEnd > 0) {
          var pMark = doc.Range(pEnd - 1, pEnd);
          if (pMark && pMark.Font) {
            pMark.Font.Position = 0;
            pMark.Font.Subscript = false;
            pMark.Font.Superscript = false;
          }
        }
        // Fix plain text immediately following shape if it leaked negative position
        if (shapeEnd < pEnd - 1) {
          var nextChar = doc.Range(shapeEnd, shapeEnd + 1);
          var isNextShape = (nextChar.InlineShapes && nextChar.InlineShapes.Count > 0) ||
                            (nextChar.OMaths && nextChar.OMaths.Count > 0);
          if (!isNextShape && nextChar.Font && nextChar.Font.Position !== 0) {
            nextChar.Font.Position = 0;
            nextChar.Font.Subscript = false;
            nextChar.Font.Superscript = false;
          }
        }
      }
    } catch (_) {}

    if (shouldMoveCursor) {
      try {
        var cursorTarget = shapeEnd;
        var endRange = doc.Range(cursorTarget, cursorTarget);
        endRange.Select();
        var curSel = wps.Application ? wps.Application.Selection : null;
        if (curSel) {
          if (typeof curSel.Collapse === "function") curSel.Collapse(0);
          if (curSel.Font) {
            curSel.Font.Position = 0;
            curSel.Font.Subscript = false;
            curSel.Font.Superscript = false;
          }
        }
      } catch (_) {}
    }

    return pic;
  } catch (err) {
    wpsLog("insertMathTypeAtDocRange: formatting error: " + err);
    return pic;
  }
}

/**
 * Scan a range for all equation targets (InlineShapes with MathType/OMML XML and native OMaths).
 * Returns array of { start, end, xml, isDisplay } sorted by start ascending.
 */
function findAllEquationTargetsInRange(range, doc) {
  var list = [];
  if (!range) return list;

  function processShape(shape) {
    if (!shape || !shape.Range) return;
    try {
      var sXml = "";
      try { sXml = shape.Range.WordOpenXML || ""; } catch (_) {}
      if (hasEquationInXml(sXml)) {
        var sStart = shape.Range.Start;
        var sEnd = shape.Range.End;
        if (sEnd > sStart + 1) sEnd = sStart + 1;
        list.push({
          start: sStart,
          end: sEnd,
          xml: sXml,
          isDisplay: false
        });
      }
    } catch (_) {}
  }

  function processOMath(omath) {
    if (!omath || !omath.Range) return;
    try {
      var oXml = "";
      try { oXml = omath.Range.WordOpenXML || ""; } catch (_) {}
      if (hasEquationInXml(oXml)) {
        var isDisp = false;
        try { isDisp = (omath.ParentOMathPara !== null && typeof omath.ParentOMathPara !== "undefined"); } catch (_) {}
        list.push({
          start: omath.Range.Start,
          end: omath.Range.End,
          xml: oXml,
          isDisplay: isDisp
        });
      }
    } catch (_) {}
  }

  // 1. Shapes from range.InlineShapes or selection.InlineShapes
  try {
    var shapes = range.InlineShapes;
    if ((!shapes || shapes.Count === 0) && wps.Application && wps.Application.Selection && wps.Application.Selection.InlineShapes) {
      shapes = wps.Application.Selection.InlineShapes;
    }
    if (shapes && shapes.Count > 0) {
      for (var i = 1; i <= shapes.Count; i++) {
        processShape(shapes.Item(i));
      }
    }
  } catch (e) {
    wpsLog("findAllEquationTargetsInRange InlineShapes error: " + e);
  }

  // 2. OMaths from range.OMaths or selection.OMaths
  try {
    var omaths = range.OMaths;
    if ((!omaths || omaths.Count === 0) && wps.Application && wps.Application.Selection && wps.Application.Selection.OMaths) {
      omaths = wps.Application.Selection.OMaths;
    }
    if (omaths && omaths.Count > 0) {
      for (var j = 1; j <= omaths.Count; j++) {
        processOMath(omaths.Item(j));
      }
    }
  } catch (e) {
    wpsLog("findAllEquationTargetsInRange OMaths error: " + e);
  }

  // Sort ascending by start offset
  list.sort(function (a, b) { return a.start - b.start; });

  // Filter out duplicates (if any item shares the exact start offset)
  var uniqueList = [];
  for (var k = 0; k < list.length; k++) {
    if (k === 0 || list[k].start >= uniqueList[uniqueList.length - 1].end) {
      uniqueList.push(list[k]);
    }
  }

  return uniqueList;
}

/**
 * Batch convert multiple LaTeX formulas in a highlighted range to MathType EMF pictures in-place.
 */
function batchLatexToMathType(doc, selection, latexMatches) {
  var baseStart = selection.Start;
  var docFontSize = getDocumentFontSize(selection.Range);
  wpsLog("batchLatexToMathType: found " + latexMatches.length + " formulas in selection [start=" + baseStart + "]");

  // Collapse selection immediately so the highlight is cleared and no selection-level formatting leaks
  try {
    if (selection && typeof selection.Collapse === "function") {
      selection.Collapse(0);
    }
  } catch (_) {}

  var total = latexMatches.length;
  var completed = 0;

  for (var i = 0; i < total; i++) {
    (function (m) {
      ajaxPost(
        BRIDGE_URL + "/convert/snippet",
        JSON.stringify({
          data: m.latex,
          from: "latex",
          to: "mathtype",
          font_pt: docFontSize
        }),
        function (status, responseText) {
          completed++;
          if (status === 200) {
            try {
              m.convData = JSON.parse(responseText);
            } catch (_) {}
          } else {
            wpsLog("batchLatexToMathType: error converting '" + m.latex + "': " + responseText);
          }

          if (completed === total) {
            // Apply DOM replacements in DESCENDING ORDER (right to left) so preceding offsets remain valid
            latexMatches.sort(function (a, b) { return b.start - a.start; });
            var convertedCount = 0;

            for (var j = 0; j < latexMatches.length; j++) {
              var match = latexMatches[j];
              if (match.convData && match.convData.result) {
                var rStart = baseStart + match.start;
                var rEnd = baseStart + match.end;
                var targetRange = doc.Range(rStart, rEnd);
                var pic = insertMathTypeAtDocRange(doc, targetRange, match.convData, false);
                if (pic) convertedCount++;
              }
            }

            // Collapse selection at the end so subsequent typing does not inherit baseline
            try {
              var app = wps.Application;
              var curSel = app ? app.Selection : null;
              if (curSel) {
                if (typeof curSel.Collapse === "function") {
                  curSel.Collapse(0);
                }
                if (curSel.Font) {
                  curSel.Font.Position = 0;
                  curSel.Font.Subscript = false;
                  curSel.Font.Superscript = false;
                }
              }
            } catch (_) {}

            wpsLog("batchLatexToMathType: converted " + convertedCount + "/" + total + " formulas.");
          }
        }
      );
    })(latexMatches[i]);
  }
}

/** Replace a math node in the paragraph package, not its editable contents. */
function ommlPackageToPlainText(xml, ordinal, latex) {
  var nsPkg = "http://schemas.microsoft.com/office/2006/xmlPackage";
  var nsW = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
  var nsM = "http://schemas.openxmlformats.org/officeDocument/2006/math";
  var tree = new DOMParser().parseFromString(xml, "application/xml");
  if (tree.getElementsByTagName("parsererror").length) throw new Error("Invalid paragraph XML");
  var parts = tree.getElementsByTagNameNS(nsPkg, "part");
  if (!parts.length) {
    nsPkg = "http://schemas.microsoft.com/office/2006/xml/packaging";
    parts = tree.getElementsByTagNameNS(nsPkg, "part");
  }
  var documentPart = null;
  for (var i = 0; i < parts.length; i++) {
    if (parts[i].getAttributeNS(nsPkg, "name") === "/word/document.xml") documentPart = parts[i];
  }
  if (!documentPart) throw new Error("Missing paragraph document part");
  var maths = documentPart.getElementsByTagNameNS(nsM, "oMath");
  var math = maths[ordinal];
  if (!math) throw new Error("Cannot locate equation in paragraph XML");
  var run = tree.createElementNS(nsW, "w:r");
  var properties = tree.createElementNS(nsW, "w:rPr");
  for (var j = 0; j < 2; j++) {
    var property = tree.createElementNS(nsW, j === 0 ? "w:i" : "w:position");
    property.setAttributeNS(nsW, "w:val", "0");
    properties.appendChild(property);
  }
  run.appendChild(properties);
  var text = tree.createElementNS(nsW, "w:t");
  text.setAttributeNS("http://www.w3.org/XML/1998/namespace", "xml:space", "preserve");
  text.appendChild(tree.createTextNode(latex));
  run.appendChild(text);
  var container = math.parentNode;
  if (container.namespaceURI === nsM && container.localName === "oMathPara" &&
      container.getElementsByTagNameNS(nsM, "oMath").length === 1) {
    container.parentNode.replaceChild(run, container);
  } else {
    container.replaceChild(run, math);
  }
  return new XMLSerializer().serializeToString(tree);
}

function replaceEquationWithLatex(doc, start, end, latex) {
  var target = doc.Range(start, end);
  var equation = null;
  try {
    if (target.OMaths && target.OMaths.Count > 0) equation = target.OMaths.Item(1);
  } catch (_) {}
  if (!equation) {
    // WPS sometimes exposes OMaths only through Selection or Document while
    // the freshly created document Range reports none.
    var owners = [wps.Application.Selection, doc];
    for (var ownerIndex = 0; !equation && ownerIndex < owners.length; ownerIndex++) {
      try {
        var collection = owners[ownerIndex].OMaths;
        if (!collection) continue;
        for (var mathIndex = 1; mathIndex <= collection.Count; mathIndex++) {
          var candidate = collection.Item(mathIndex);
          if (candidate.Range.Start === start) { equation = candidate; break; }
        }
      } catch (_) {}
    }
  }
  if (equation) {
    start = equation.Range.Start;
    var paragraph = target.Paragraphs.Item(1).Range;
    var maths = paragraph.OMaths;
    var ordinal = -1;
    for (var i = 1; i <= maths.Count; i++) {
      if (maths.Item(i).Range.Start === start) { ordinal = i - 1; break; }
    }
    if (ordinal < 0) throw new Error("Cannot locate equation in its paragraph");
    var xml = ommlPackageToPlainText(paragraph.WordOpenXML, ordinal, latex);
    // Replacing the enclosing paragraph exits WPS's equation editing context.
    // All other runs, equations, bookmarks and paragraph formatting stay in XML.
    doc.Range(paragraph.Start, paragraph.End).InsertXML(xml);
  } else {
    var nativeXml = "";
    try { nativeXml = target.WordOpenXML || ""; } catch (_) {}
    if (/<(?:[A-Za-z_][\w.-]*:)?oMath(?:\s|>)/.test(nativeXml)) {
      throw new Error("Cannot locate the native OMML container; document was not changed");
    }
    try {
      if (target.InlineShapes && target.InlineShapes.Count > 0 && end > start + 1) {
        target = doc.Range(start, start + 1);
      }
    } catch (_) {}
    target.Text = latex;
  }
  var inserted = doc.Range(start, start + latex.length);
  if (inserted.Font) {
    inserted.Font.Position = 0;
    inserted.Font.Subscript = false;
    inserted.Font.Superscript = false;
  }
  return start + latex.length;
}

/**
 * Batch convert multiple equations in a highlighted range to LaTeX $...$ strings in-place.
 */
function batchEquationToLatex(doc, selection, eqTargets) {
  wpsLog("batchEquationToLatex: found " + eqTargets.length + " equations in selection");

  var total = eqTargets.length;
  var completed = 0;

  for (var i = 0; i < total; i++) {
    (function (eq) {
      ajaxPost(
        BRIDGE_URL + "/convert/snippet",
        JSON.stringify({
          data: eq.xml,
          from: "auto",
          to: "latex"
        }),
        function (status, responseText) {
          completed++;
          if (status === 200) {
            try {
              var res = JSON.parse(responseText);
              if (res && res.result) {
                eq.latex = res.result.trim();
              }
            } catch (_) {}
          } else {
            wpsLog("batchEquationToLatex: conversion error: " + responseText);
          }

          if (completed === total) {
            // Apply DOM replacements in DESCENDING ORDER (right to left) so preceding offsets remain valid
            eqTargets.sort(function (a, b) { return b.start - a.start; });
            var convertedCount = 0;

            for (var j = 0; j < eqTargets.length; j++) {
              var item = eqTargets[j];
              if (item.latex) {
                var rStart = item.start;
                var rEnd = item.end;
                var latexStr = item.isDisplay ? ("$$" + item.latex + "$$") : ("$" + item.latex + "$");
                replaceEquationWithLatex(doc, rStart, rEnd, latexStr);

                // Check and remove adjacent trailing ZWNJ if present
                try {
                  var afterPos = rStart + latexStr.length;
                  var nextChar = doc.Range(afterPos, afterPos + 1);
                  if (nextChar && nextChar.Text === "‌") {
                    nextChar.Delete();
                  }
                } catch (_) {}

                convertedCount++;
              }
            }

            // Reset selection formatting
            try {
              var app = wps.Application;
              var curSel = app ? app.Selection : null;
              if (curSel) {
                if (typeof curSel.Collapse === "function") {
                  curSel.Collapse(0);
                }
                if (curSel.Font) {
                  curSel.Font.Position = 0;
                  curSel.Font.Subscript = false;
                  curSel.Font.Superscript = false;
                }
              }
            } catch (_) {}

            wpsLog("batchEquationToLatex: converted " + convertedCount + "/" + total + " equations to LaTeX.");
          }
        }
      );
    })(eqTargets[i]);
  }
}

/**
 * 3. Toggle LaTeX:
 * Two-way conversion between LaTeX text and MathType equation in WPS.
 * Supports:
 * - Highlighting an entire paragraph / range with multiple LaTeX formulas -> batch converts to MathType equations in-place
 * - Highlighting an entire paragraph / range with multiple equations -> batch converts to LaTeX $...$ text in-place
 * - Single click / collapsed cursor inside/adjacent to equation or LaTeX -> toggles single target
 */
function ToggleLatex() {
  try {
    wpsLog("ToggleLatex clicked");
    var app = wps.Application;
    var selection = app ? app.Selection : null;
    if (!selection) return;
    var doc = app.ActiveDocument;
    if (!doc) return;

    var isRangeSelected = (typeof selection.Start === "number" && typeof selection.End === "number" && selection.Start !== selection.End);

    // ========================================================
    // CASE 1: User highlighted / selected a range (paragraph, sentence, etc.)
    // ========================================================
    if (isRangeSelected) {
      var selText = selection.Text || "";
      var latexMatches = findAllLatexMatches(selText);

      // If no delimited matches found, check if the entire selected text itself is bare LaTeX (e.g. \\frac{1}{2})
      if (latexMatches.length === 0 && isLatexText(selText)) {
        latexMatches.push({
          start: 0,
          end: selText.length,
          raw: selText,
          latex: selText.trim(),
          isDisplay: false
        });
      }

      if (latexMatches.length > 0) {
        // Selection contains LaTeX formulas -> Batch convert LaTeX to MathType
        wpsLog("ToggleLatex: range contains " + latexMatches.length + " LaTeX formulas -> converting to MathType");
        ensureBridgeRunning(function () {
          batchLatexToMathType(doc, selection, latexMatches);
        });
        return;
      }

      // If no LaTeX in selection text, check for equations in the selection
      var eqTargets = findAllEquationTargetsInRange(selection.Range, doc);
      if (eqTargets.length === 0) {
        // Fallback check single equation
        var singleEq = findEquationTarget(selection, doc);
        if (singleEq) {
          var sXml = "";
          try { sXml = singleEq.WordOpenXML || ""; } catch (_) {}
          if (hasEquationInXml(sXml)) {
            eqTargets.push({
              start: singleEq.Start,
              end: singleEq.End,
              xml: sXml,
              isDisplay: false
            });
          }
        }
      }

      if (eqTargets.length > 0) {
        // Selection contains equations -> Batch convert Equations to LaTeX text
        wpsLog("ToggleLatex: range contains " + eqTargets.length + " equations -> converting to LaTeX");
        ensureBridgeRunning(function () {
          batchEquationToLatex(doc, selection, eqTargets);
        });
        return;
      }

      // Selected text has neither LaTeX nor equations
      wpsLog("ToggleLatex: no equation or LaTeX found in selected range");
      alert("Không tìm thấy công thức hoặc mã LaTeX nào trong vùng chọn!\n\nBạn có thể bôi đen đoạn văn bản chứa mã LaTeX ($...$) hoặc công thức để chuyển đổi.");
      return;
    }

    // ========================================================
    // CASE 2: Selection is collapsed (single cursor insertion point)
    // ========================================================
    // 2.1 Check if an equation is selected or adjacent to cursor
    var eqRange = findEquationTarget(selection, doc);
    var eqXml = "";

    if (eqRange) {
      try {
        if (eqRange.WordOpenXML && hasEquationInXml(eqRange.WordOpenXML)) {
          eqXml = eqRange.WordOpenXML;
        }
      } catch (_) {}
    }

    if (!eqXml) {
      try {
        if (selection.WordOpenXML && hasEquationInXml(selection.WordOpenXML)) {
          eqXml = selection.WordOpenXML;
          if (!eqRange) eqRange = selection.Range;
        } else if (selection.Range && selection.Range.WordOpenXML && hasEquationInXml(selection.Range.WordOpenXML)) {
          eqXml = selection.Range.WordOpenXML;
          if (!eqRange) eqRange = selection.Range;
        }
      } catch (_) {}
    }

    if (eqXml && eqRange) {
      // Convert single equation to LaTeX text
      wpsLog("ToggleLatex: converting single equation to LaTeX");
      ensureBridgeRunning(function () {
        ajaxPost(
          BRIDGE_URL + "/convert/snippet",
          JSON.stringify({
            data: eqXml,
            from: "auto",
            to: "latex"
          }),
          function (status, responseText) {
            if (status === 200) {
              var snip = JSON.parse(responseText);
              if (snip && snip.result) {
                var clean = snip.result.trim();
                var isDisplay = false;
                try {
                  if (eqRange.OMaths && eqRange.OMaths.Count > 0 && eqRange.OMaths.Item(1).ParentOMathPara) {
                    isDisplay = true;
                  }
                } catch (_) {}

                var latexStr = isDisplay ? ("$$" + clean + "$$") : ("$" + clean + "$");

                var rStart = eqRange.Start;
                var rEnd = eqRange.End;
                try {
                  if (eqRange.InlineShapes && eqRange.InlineShapes.Count > 0 && rEnd - rStart > 1) {
                    rEnd = rStart + 1;
                  }
                } catch (_) {}

                var nextPos = replaceEquationWithLatex(doc, rStart, rEnd, latexStr);

                // Clean up any trailing ZWNJ if present
                try {
                  var afterPos = rStart + latexStr.length;
                  var nextChar = doc.Range(afterPos, afterPos + 1);
                  if (nextChar && nextChar.Text === "‌") {
                    nextChar.Delete();
                  }
                } catch (_) {}

                // Move cursor after the inserted LaTeX text
                var endRange = doc.Range(nextPos, nextPos);
                endRange.Select();
                var curSel = app.Selection;
                if (curSel) {
                  if (typeof curSel.Collapse === "function") curSel.Collapse(0);
                  if (curSel.Font) {
                    curSel.Font.Position = 0;
                    curSel.Font.Subscript = false;
                    curSel.Font.Superscript = false;
                  }
                }
                wpsLog("ToggleLatex: converted single equation to LaTeX string successfully");
              }
            } else {
              wpsLog("ToggleLatex: equation to LaTeX failed: " + responseText);
              alert("Không thể chuyển đổi công thức sang LaTeX: " + responseText);
            }
          }
        );
      });
      return;
    }

    // 2.2 Cursor is collapsed inside or adjacent to LaTeX in current paragraph
    var targetLatexRange = null;
    var rawLatexText = "";
    try {
      var para = selection.Paragraphs.Item(1);
      if (para && para.Range) {
        var pStart = para.Range.Start;
        var curPos = selection.Start;
        var pText = para.Range.Text || "";
        var relPos = curPos - pStart;
        var found = findEnclosingLatexInPara(pText, relPos);
        if (found) {
          targetLatexRange = doc.Range(pStart + found.start, pStart + found.end);
          rawLatexText = found.text;
          wpsLog("ToggleLatex: auto-detected enclosing LaTeX at [" + (pStart + found.start) + ".." + (pStart + found.end) + "]: " + rawLatexText);
        }
      }
    } catch (e) {
      wpsLog("ToggleLatex: auto-detection error: " + e);
    }

    if (!targetLatexRange || !rawLatexText) {
      wpsLog("ToggleLatex: no equation or LaTeX found at cursor");
      alert("Không tìm thấy công thức hoặc mã LaTeX nào tại vị trí con trỏ!\n\nBạn có thể đặt con trỏ bên trong mã LaTeX dạng $...$ hoặc bôi đen đoạn văn bản cần chuyển đổi.");
      return;
    }

    // Clean outer delimiters
    var cleanLatex = rawLatexText.trim();
    if (cleanLatex.startsWith("$$") && cleanLatex.endsWith("$$") && cleanLatex.length >= 4) {
      cleanLatex = cleanLatex.slice(2, -2).trim();
    } else if (cleanLatex.startsWith("$") && cleanLatex.endsWith("$") && cleanLatex.length >= 2) {
      cleanLatex = cleanLatex.slice(1, -1).trim();
    } else if (cleanLatex.startsWith("\\[") && cleanLatex.endsWith("\\]") && cleanLatex.length >= 4) {
      cleanLatex = cleanLatex.slice(2, -2).trim();
    } else if (cleanLatex.startsWith("\\(") && cleanLatex.endsWith("\\)") && cleanLatex.length >= 4) {
      cleanLatex = cleanLatex.slice(2, -2).trim();
    }

    var docFontSize = getDocumentFontSize(targetLatexRange);
    wpsLog("ToggleLatex: converting single LaTeX '" + cleanLatex + "' to MathType");

    ensureBridgeRunning(function () {
      ajaxPost(
        BRIDGE_URL + "/convert/snippet",
        JSON.stringify({
          data: cleanLatex,
          from: "latex",
          to: "mathtype",
          font_pt: docFontSize
        }),
        function (status, responseText) {
          if (status === 200) {
            var conv = JSON.parse(responseText);
            if (conv.result) {
              insertMathTypeAtDocRange(doc, targetLatexRange, conv, true);
            }
            wpsLog("ToggleLatex: successfully converted single LaTeX to MathType equation");
          } else {
            wpsLog("ToggleLatex: LaTeX conversion error: " + responseText);
            alert("Chuyển đổi LaTeX sang công thức thất bại:\n" + responseText);
          }
        }
      );
    });
  } catch (err) {
    wpsLog("ToggleLatex: unhandled error: " + err);
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
 * Import the selected output format. MathType always retains OLE plus EMF;
 * never degrade it to a standalone picture.
 */
function HandleSessionResult(waitData) {
  if (!waitData || waitData.status !== "ready") return;

  if (!waitData.flat_opc) {
    alert("EqnTide không trả về gói công thức để chèn.");
    return;
  }
  if (/ProgID=["']Equation\./i.test(waitData.flat_opc)) {
    if (!InsertMathTypeEquation(waitData)) alert("Không thể chèn đối tượng MathType OLE + EMF.");
  } else {
    InsertOmmlToSelection(waitData.flat_opc);
  }
}

/** Import native OMML through Flat OPC, then remove only its added break. */
function InsertOmmlToSelection(flatOpcXml) {
  try {
    var app = wps.Application;
    var doc = app ? app.ActiveDocument : null;
    var sel = app ? app.Selection : null;
    if (!doc) return;
    var range = (g_editRangeStart !== null && g_editRangeEnd !== null)
      ? doc.Range(g_editRangeStart, g_editRangeEnd)
      : (sel ? sel.Range : null);
    if (!range || typeof range.InsertXML !== "function") return;
    if (!/<(?:[A-Za-z_][\w.-]*:)?(?:package|xmlPackage)\b/.test(flatOpcXml)) {
      throw new Error("OMML insertion requires a Flat OPC package");
    }
    var start = range.Start;
    var replacedLength = range.End - start;
    var originalEnd = doc.Range().End;
    var suffix = doc.Range(range.End, Math.min(range.End + 32, originalEnd)).Text;
    range.InsertXML(flatOpcXml);
    // Follow 1bd42ab: WPS may include the paragraph mark in OMath.Range,
    // or not expose OMaths at all. Document growth defines the inserted span.
    var newDocEnd = doc.Range().End;
    var insertedEnd = start + newDocEnd - originalEnd + replacedLength;
    wpsLog("InsertOmmlToSelection: inserted span " + start + ".." + insertedEnd);
    if (insertedEnd > start && insertedEnd <= newDocEnd &&
        typeof suffix === "string" &&
        doc.Range(insertedEnd, Math.min(insertedEnd + suffix.length, newDocEnd)).Text === suffix) {
      // Only trim characters within the imported span; never the original
      // paragraph mark or text following it. Support CR, LF and CRLF.
      for (var breaks = 0; breaks < 2 && insertedEnd > start; breaks++) {
        var trailing = doc.Range(insertedEnd - 1, insertedEnd);
        if (trailing.Text !== "\r" && trailing.Text !== "\n") break;
        trailing.Delete();
        insertedEnd--;
        wpsLog("InsertOmmlToSelection: removed imported paragraph mark");
      }
      // WPS can append a normal-text spacer after imported OMML. It belongs
      // to the newly inserted span, not the original suffix (verified above).
      // Preserve intentional spaces within the incoming equation.
      var inputTree = new DOMParser().parseFromString(flatOpcXml, "application/xml");
      var mathText = inputTree.getElementsByTagNameNS(
        "http://schemas.openxmlformats.org/officeDocument/2006/math", "t");
      var lastMathText = mathText.length ? mathText[mathText.length - 1].textContent : "";
      if (insertedEnd > start && lastMathText && !/\s$/.test(lastMathText)) {
        var spacer = doc.Range(insertedEnd - 1, insertedEnd);
        if (spacer.Text === " ") {
          spacer.Delete();
          insertedEnd--;
          wpsLog("InsertOmmlToSelection: removed imported spacer");
        }
      }
    }
    g_editRangeStart = null;
    g_editRangeEnd = null;
    g_editTargetRange = null;

    // Inline formatting is best-effort; break cleanup must not depend on
    // OMaths support or the range returned by a particular WPS build.
    try {
      var equation = null;
      if (range.OMaths && range.OMaths.Count > 0) {
        equation = range.OMaths.Item(1);
      } else if (doc.OMaths) {
        for (var i = 1; i <= doc.OMaths.Count; i++) {
          var candidate = doc.OMaths.Item(i);
          if (candidate.Range.Start >= start && candidate.Range.Start <= start + 1) {
            equation = candidate;
            break;
          }
        }
      }
      if (equation) {
        equation.Type = 1; // wdOMathInline
        if (equation.Range.Font) equation.Range.Font.Position = 0;
      }
    } catch (formatError) {
      wpsLog("InsertOmmlToSelection: optional inline formatting: " + formatError);
    }
    var end = insertedEnd;
    if (typeof end === "number" && end >= start) {
      doc.Range(end, end).Select();
      if (app.Selection && app.Selection.Font) app.Selection.Font.Position = 0;
    }
  } catch (error) {
    wpsLog("InsertOmmlToSelection: " + error);
    alert("Không thể chèn công thức OMML: " + error);
  }
}

/**
 * Insert a complete MathType object: OLE/MTEF plus EMF/MTEF preview.
 */
function InsertMathTypeEquation(waitData) {
  try {
    var app = wps.Application;
    var doc = app ? app.ActiveDocument : null;
    var sel = app ? app.Selection : null;
    if (!doc) return false;

    wpsLog("InsertMathTypeEquation: start. editRangeStart=" + g_editRangeStart + ", editRangeEnd=" + g_editRangeEnd);

    var isEdit = (g_editRangeStart !== null && g_editRangeEnd !== null);
    var targetRange = null;

    if (isEdit) {
      try {
        targetRange = doc.Range(g_editRangeStart, g_editRangeEnd);
        wpsLog("InsertMathTypeEquation: targeting edit range [" + g_editRangeStart + ".." + g_editRangeEnd + "]");
      } catch (e) {
        wpsLog("InsertMathTypeEquation: error getting edit range: " + e);
      }
    }
    g_editRangeStart = null;
    g_editRangeEnd = null;
    g_editTargetRange = null;

    if (!targetRange) {
      targetRange = sel ? sel.Range : null;
    }
    if (!targetRange) return false;

    var pic = insertMathTypeAtDocRange(doc, targetRange, waitData, true);
    return pic !== null;
  } catch (err) {
    wpsLog("InsertMathTypeEquation: unhandled error: " + err);
    return false;
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
  // Once the registered native desktop starts, ensureBridgeRunning retries
  // the original bridge request.
  var uri = "eqntide://open";

  // WPS exposes OAAssist on Application in some Linux builds, rather than
  // directly on wps. Resolve each supported application object separately.
  var assistants = [];
  try { assistants.push(wps.OAAssist); } catch (_) {}
  try { assistants.push(wps.Application.OAAssist); } catch (_) {}
  try { assistants.push(window.Application.OAAssist); } catch (_) {}
  try { assistants.push(wps.WpsApplication().OAAssist); } catch (_) {}
  for (var i = 0; i < assistants.length; i++) {
    try {
      if (assistants[i] && typeof assistants[i].ShellExecute === "function") {
        // The installer/portable registration owns the executable location on
        // Windows and Linux. Prefer the registered URI.
        assistants[i].ShellExecute(uri, "");
        if (typeof navigator !== "undefined" && /linux/i.test(navigator.platform || "")) {
          assistants[i].ShellExecute("/usr/bin/xdg-open", uri);
        }
        break;
      }
    } catch (_) {}
  }

  // WPS may silently ignore ShellExecute on Linux. Always invoke the registered
  // URI handler too; the native desktop's single-instance IPC prevents duplicate windows.
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
