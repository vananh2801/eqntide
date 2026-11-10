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
          HandleSessionResult(waitData);
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
      var tStart = targetRange.Start;
      var tEnd = targetRange.End;
      // If targetRange contains an InlineShape, pin range strictly to 1 character (do NOT delete adjacent text)
      if (isEquationFound && (tEnd - tStart > 1)) {
        wpsLog("EditEquation: clamped range from [" + tStart + ".." + tEnd + "] to [" + tStart + ".." + (tStart + 1) + "]");
        tEnd = tStart + 1;
      }
      // Check if immediate next character is U+200C (reset anchor), include it in deletion
      try {
        var doc = app.ActiveDocument;
        if (doc && tEnd < (doc.Content ? doc.Content.End : 999999)) {
          var nextChar = doc.Range(tEnd, tEnd + 1);
          if (nextChar && nextChar.Text === "‌") {
            tEnd = tEnd + 1;
            wpsLog("EditEquation: included adjacent U+200C in editRange");
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
 * Handle session completion: prefers native EMF AddPicture (prevents lazy-load cache drop in WPS Linux)
 * and falls back to Flat OPC InsertXML if needed.
 */
function HandleSessionResult(waitData) {
  if (!waitData || waitData.status !== "ready") return;

  // Try inserting EMF picture via AddPicture (Claude Test 2 / Clean Media Insertion)
  if (waitData.emf_path) {
    var success = InsertEmfPicture(waitData);
    if (success) {
      wpsLog("HandleSessionResult: completed successfully via InsertEmfPicture");
      return;
    }
    wpsLog("HandleSessionResult: InsertEmfPicture returned false, falling back to InsertFlatOpcToSelection");
  }

  // Fallback to InsertFlatOpcToSelection
  if (waitData.flat_opc) {
    InsertFlatOpcToSelection(waitData.flat_opc);
  }
}

/**
 * Insert vector EMF picture with embedded MTEF using InlineShapes.AddPicture.
 * Does not import Flat OPC packages, completely bypassing WPS Linux relationship cache corruption.
 */
function InsertEmfPicture(waitData) {
  try {
    var app = wps.Application;
    var doc = app ? app.ActiveDocument : null;
    var sel = app ? app.Selection : null;
    var range = null;

    wpsLog("InsertEmfPicture: start. editRangeStart=" + g_editRangeStart + ", editRangeEnd=" + g_editRangeEnd);

    var isEdit = (doc && g_editRangeStart !== null && g_editRangeEnd !== null);
    var insertPos = isEdit ? g_editRangeStart : (sel ? sel.Range.Start : 0);

    if (isEdit) {
      try {
        var oldRange = doc.Range(g_editRangeStart, g_editRangeEnd);
        wpsLog("InsertEmfPicture: deleting old equation at range [" + g_editRangeStart + ".." + g_editRangeEnd + "]");
        oldRange.Delete();
        range = doc.Range(insertPos, insertPos);
      } catch (e) {
        wpsLog("InsertEmfPicture: error deleting old equation: " + e);
      }
    }
    g_editRangeStart = null;
    g_editRangeEnd = null;
    g_editTargetRange = null;

    if (!range) {
      range = sel ? sel.Range : null;
    }
    if (!range) return false;

    var emfPath = waitData.emf_path;
    if (!emfPath) return false;

    wpsLog("InsertEmfPicture: calling AddPicture(" + emfPath + ") at pos " + insertPos);
    var pic = null;
    try {
      pic = range.InlineShapes.AddPicture(emfPath, false, true, range);
    } catch (e) {
      wpsLog("InsertEmfPicture: AddPicture exception: " + e);
      return false;
    }

    if (!pic) {
      wpsLog("InsertEmfPicture: AddPicture returned null");
      return false;
    }

    try {
      if (typeof waitData.width_pt === "number" && waitData.width_pt > 0) {
        pic.Width = waitData.width_pt;
      }
      if (typeof waitData.height_pt === "number" && waitData.height_pt > 0) {
        pic.Height = waitData.height_pt;
      }

      var docFontSize = 12.0;
      try {
        if (range && range.Font && range.Font.Size > 0) {
          docFontSize = range.Font.Size;
        } else if (pic.Range && pic.Range.Font && pic.Range.Font.Size > 0) {
          docFontSize = pic.Range.Font.Size;
        }
      } catch (_) {}

      var depthPt = (typeof waitData.depth_pt === "number") ? waitData.depth_pt : 0;
      // In Word/WPS JS API, Font.Position is in points (NOT half-points like OpenXML w:val).
      // WPS aligns the bottom of an InlineShape with the font descent/line box bottom.
      // To align the formula baseline with adjacent text, add font baseline offset (~11-12% of font size).
      var fontBaselineOffset = docFontSize * 0.045; // ~0.54 pt for 12pt font
      var pos = -Number((depthPt + fontBaselineOffset).toFixed(1));
      if (pic.Range && pic.Range.Font) {
        pic.Range.Font.Position = pos;
        wpsLog("InsertEmfPicture: set baseline Font.Position=" + pos + " (depthPt=" + depthPt + ", offset=" + fontBaselineOffset.toFixed(2) + ", docFontSize=" + docFontSize + ")");
      }

      // Add U+200C reset run with position 0 immediately after picture
      var picEnd = pic.Range ? pic.Range.End : range.End;
      var resetRange = doc.Range(picEnd, picEnd);
      resetRange.InsertAfter("‌"); // U+200C
      var zwnjRange = doc.Range(picEnd, picEnd + 1);
      if (zwnjRange && zwnjRange.Font) {
        zwnjRange.Font.Position = 0;
        zwnjRange.Font.Subscript = false;
        zwnjRange.Font.Superscript = false;
      }
      zwnjRange.Select();
      var curSel = app.Selection;
      if (curSel && typeof curSel.Collapse === "function") {
        curSel.Collapse(0); // wdCollapseEnd
      }
    } catch (e) {
      wpsLog("InsertEmfPicture: formatting error: " + e);
    }

    wpsLog("InsertEmfPicture: finished successfully.");
    return true;
  } catch (err) {
    wpsLog("InsertEmfPicture: unhandled error: " + err);
    return false;
  }
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
        range = doc.Range(g_editRangeStart, g_editRangeEnd);
        wpsLog('InsertFlatOpcToSelection: target replace range [' + g_editRangeStart + '..' + g_editRangeEnd + ']');
      } catch (e) {
        wpsLog('InsertFlatOpcToSelection: error obtaining edit range: ' + e);
      }
    }
    g_editRangeStart = null;
    g_editRangeEnd = null;
    g_editTargetRange = null;

    if (!range) {
      range = sel ? sel.Range : null;
    }
    if (!range) return;

    var startPos = (range && typeof range.Start === "number") ? range.Start : 0;

    if (typeof range.InsertXML === "function") {
      range.InsertXML(flatOpcXml);
    }

        if (doc) {
      try {
        var docEnd = doc.Content ? doc.Content.End : (startPos + 200);
        var fullText = doc.Range(0, Math.min(docEnd, 200)).Text;
        wpsLog("DOC FULL TEXT START: " + JSON.stringify(fullText));

        // Find the newly inserted InlineShape closest to startPos
        var targetShape = null;
        if (doc.InlineShapes && doc.InlineShapes.Count > 0) {
          var minDiff = 999999;
          for (var i = 1; i <= doc.InlineShapes.Count; i++) {
            var shp = doc.InlineShapes.Item(i);
            if (shp && shp.Range && shp.Range.Start >= startPos) {
              var diff = Math.abs(shp.Range.Start - startPos);
              if (diff < minDiff) {
                minDiff = diff;
                targetShape = shp;
                if (diff <= 5) break; // Exact match found
              }
            }
          }
        }

        var shpEnd = targetShape ? targetShape.Range.End : (startPos + 1);
        wpsLog("targetShape: Start=" + (targetShape ? targetShape.Range.Start : "none") + ", End=" + shpEnd);

        // U+200C is placed immediately after the InlineShape: [shpEnd .. shpEnd + 1]
        var actualZwnjPos = shpEnd;
        
        // Log character at shpEnd
        try {
          var zwnjRange = doc.Range(actualZwnjPos, actualZwnjPos + 1);
          var zText = zwnjRange ? zwnjRange.Text : "";
          var zCode = zText ? zText.charCodeAt(0) : 0;
          wpsLog("zwnjRange pos=" + actualZwnjPos + ", code=" + zCode + ", text=" + JSON.stringify(zText)
                + ", PosBefore=" + (zwnjRange.Font ? zwnjRange.Font.Position : "none"));

          if (zwnjRange && zwnjRange.Font) {
            zwnjRange.Font.Position = 0;
            zwnjRange.Font.Subscript = false;
            zwnjRange.Font.Superscript = false;
          }
          wpsLog("zwnjRange PosAfter=" + (zwnjRange.Font ? zwnjRange.Font.Position : "none"));
        } catch (e) {
          wpsLog("error resetting zwnjRange: " + e);
        }

        // Target cursor: immediately after U+200C [shpEnd + 1]
        var cursorTarget = actualZwnjPos + 1;

        // Check if there is a trailing paragraph break right after U+200C
        try {
          var afterRange = doc.Range(cursorTarget, cursorTarget + 1);
          var aText = afterRange ? afterRange.Text : "";
          var aCode = aText ? aText.charCodeAt(0) : 0;
          wpsLog("afterRange pos=" + cursorTarget + ", code=" + aCode + ", text=" + JSON.stringify(aText));
          if (aCode === 13 || aCode === 10 || aCode === 11 || aText === "\r" || aText === "\n") {
            afterRange.Delete();
            wpsLog("deleted trailing break at " + cursorTarget);
          }
        } catch (e) {
          wpsLog("error deleting trailing break: " + e);
        }

        // Target range is the reset character run itself [actualZwnjPos .. actualZwnjPos + 1]
        var zwnjSelRange = doc.Range(actualZwnjPos, actualZwnjPos + 1);
        zwnjSelRange.Select();
        var curSel = app.Selection;
        if (curSel) {
          if (curSel.Font) {
            curSel.Font.Position = 0;
            curSel.Font.Subscript = false;
            curSel.Font.Superscript = false;
          }
          // Collapse selection to end (0 = wdCollapseEnd)
          if (typeof curSel.Collapse === "function") {
            curSel.Collapse(0);
          }
        }
        wpsLog("InsertFlatOpcToSelection: selection collapsed to end of U+200C at " + actualZwnjPos);
      } catch (e) {
        wpsLog("InsertFlatOpcToSelection: error: " + e);
      }
    }

    // Claude workaround 2.4: Trigger document recalculation & refresh so subsequent shapes reload correctly
    try {
      if (app) {
        app.ScreenUpdating = true;
        if (typeof app.ScreenRefresh === "function") {
          app.ScreenRefresh();
        }
        if (doc && typeof doc.Repaginate === "function") {
          doc.Repaginate();
        }
        var win = app.ActiveWindow;
        if (win && win.ActivePane && win.ActivePane.View && win.ActivePane.View.Zoom) {
          var curZoom = win.ActivePane.View.Zoom.Percentage;
          win.ActivePane.View.Zoom.Percentage = curZoom + 1;
          win.ActivePane.View.Zoom.Percentage = curZoom;
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
