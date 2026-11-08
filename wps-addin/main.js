// WPS Office JS Add-in: Main Controller & Ribbon Handlers
// Compatible with WPS Office Writer on Windows, Linux, and macOS
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

var BRIDGE_URL = "http://127.0.0.1:17890";
var g_ribbonUI = null;

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
 * Main Ribbon Action Dispatcher
 */
function OnAction(control) {
  switch (control.Id) {
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
      console.warn("Unknown control action:", control.Id);
  }
}

/**
 * 1. Insert Equation:
 * Opens a new session in desktop editor and inserts the resulting Flat OPC XML.
 */
function InsertEquation() {
  ajaxPost(
    BRIDGE_URL + "/session/open",
    JSON.stringify({ data: "", format: "auto", target_mode: "omml" }),
    function (status, responseText) {
      if (status !== 200) {
        if (status === 0) {
          alert("Could not connect to Math Editor Bridge (127.0.0.1:17890). Please ensure the editor app is running.");
        } else {
          alert("Bridge error (" + status + "): " + responseText);
        }
        return;
      }

      var res = JSON.parse(responseText);
      var sessionId = res.session_id;

      // Launch desktop popup editor
      LaunchDesktopEditor(sessionId);

      // Long-poll wait for user confirmation
      WaitSession(sessionId, function (waitData) {
        if (waitData && waitData.status === "ready" && waitData.flat_opc) {
          InsertFlatOpcToSelection(waitData.flat_opc);
        }
      });
    }
  );
}

/**
 * 2. Edit Equation:
 * Extracts selected equation XML or text from WPS Selection and opens it in editor.
 */
function EditEquation() {
  try {
    var app = wps.Application;
    var selection = app.Selection;
    if (!selection) {
      alert("Please select an equation to edit.");
      return;
    }

    var selData = "";
    try {
      if (selection.Range && selection.Range.WordOpenXML) {
        selData = selection.Range.WordOpenXML;
      }
    } catch (_) {}

    if (!selData) {
      try {
        if (selection.Text) {
          selData = selection.Text;
        }
      } catch (_) {}
    }

    ajaxPost(
      BRIDGE_URL + "/session/open",
      JSON.stringify({
        data: selData,
        format: "auto",
        target_mode: "omml"
      }),
      function (status, responseText) {
        if (status !== 200) {
          if (status === 0) {
            alert("Could not connect to Math Editor Bridge (127.0.0.1:17890). Please ensure the editor app is running.");
          } else {
            alert("Bridge error (" + status + "): " + responseText);
          }
          return;
        }

        var res = JSON.parse(responseText);
        var sessionId = res.session_id;

        LaunchDesktopEditor(sessionId);

        WaitSession(sessionId, function (waitData) {
          if (waitData && waitData.status === "ready" && waitData.flat_opc) {
            InsertFlatOpcToSelection(waitData.flat_opc);
          }
        });
      }
    );
  } catch (err) {
    alert("Error reading equation from selection: " + err);
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
            alert("LaTeX conversion failed.");
          }
        }
      );
      return;
    }

    // Case B: Selection contains an equation, convert to LaTeX string
    var ooxml = selection.Range.WordOpenXML || "";
    ajaxPost(
      BRIDGE_URL + "/convert/snippet",
      JSON.stringify({
        data: ooxml,
        from: "flat_opc",
        to: "mathml"
      }),
      function (status, responseText) {
        if (status === 200) {
          var snip = JSON.parse(responseText);
          selection.Text = "$" + snip.result + "$";
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
    var range = wps.Application.Selection.Range;
    if (typeof range.InsertXML === "function") {
      range.InsertXML(flatOpcXml);
    } else {
      // Fallback: clipboard paste
      var textarea = document.createElement("textarea");
      textarea.value = flatOpcXml;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      range.Paste();
    }
  } catch (err) {
    console.error("InsertFlatOpcToSelection error:", err);
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
 * Helper: Launch desktop editor via custom URI
 */
function LaunchDesktopEditor(param) {
  var sessionId = param === "batch" ? "" : (param || "");
  // 1. Direct local bridge HTTP call to immediately show & focus desktop window
  ajaxPost(
    BRIDGE_URL + "/window/show",
    JSON.stringify({ session_id: sessionId, tab: param === "batch" ? "batch" : "editor" }),
    function () {}
  );

  // 2. Custom URI fallback (for cold start)
  var uri = "matheditor://open?" + (param === "batch" ? "tab=batch" : "session_id=" + sessionId);
  if (wps.OAAssist && typeof wps.OAAssist.ShellExecute === "function") {
    try {
      wps.OAAssist.ShellExecute(uri);
    } catch (_) {}
  } else {
    var iframe = document.createElement("iframe");
    iframe.style.display = "none";
    iframe.src = uri;
    document.body.appendChild(iframe);
    setTimeout(function () {
      try {
        document.body.removeChild(iframe);
      } catch (_) {}
    }, 2000);
  }
}

/**
 * Helper: Long-poll wait for session result
 */
function WaitSession(sessionId, callback) {
  ajaxGet(BRIDGE_URL + "/session/" + sessionId + "/wait?timeout=300", function (status, responseText) {
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
}
