// Taskpane Script for WPS Office Add-in
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

var BRIDGE_URL = "http://127.0.0.1:17890";
var BRIDGE_TOKEN_HEADER = "X-EqnTide-Token";
var g_bridgeToken = null;

function getBridgeToken() {
  if (g_bridgeToken) return g_bridgeToken;
  try {
    if (typeof window !== "undefined" && window.EQNTIDE_BRIDGE_TOKEN) {
      g_bridgeToken = String(window.EQNTIDE_BRIDGE_TOKEN).trim();
      if (g_bridgeToken) return g_bridgeToken;
    }
  } catch (_) {}
  try {
    if (typeof wps !== "undefined" && wps && wps.FileSystem && typeof wps.FileSystem.ReadFile === "function") {
      var candidates = [];
      if (wps.Env && typeof wps.Env.GetHomePath === "function") {
        candidates.push(wps.Env.GetHomePath() + "/.local/share/eqntide/bridge.token");
      }
      if (typeof navigator !== "undefined" && /win/i.test(navigator.platform || "")) {
        try {
          if (wps.Env && typeof wps.Env.GetTempPath === "function") {
            var localApp = wps.Env.GetTempPath().replace(/[\\/]+Temp[\\/]?$/i, "");
            if (localApp) candidates.push(localApp + "\\eqntide\\EqnTide\\data\\bridge.token");
          }
        } catch (_) {}
      }
      for (var i = 0; i < candidates.length; i++) {
        try {
          var raw = wps.FileSystem.ReadFile(candidates[i]);
          if (raw && String(raw).trim()) {
            g_bridgeToken = String(raw).trim();
            return g_bridgeToken;
          }
        } catch (_) {}
      }
    }
  } catch (_) {}
  return "";
}

function refreshBridgeToken(callback) {
  if (getBridgeToken()) {
    if (callback) callback();
    return;
  }
  try {
    if (typeof document !== "undefined" && document && document.head) {
      var script = document.createElement("script");
      script.type = "text/javascript";
      script.src = "bridge-token.js?t=" + Date.now();
      script.onload = function () {
        getBridgeToken();
        if (callback) callback();
      };
      script.onerror = function () {
        if (callback) callback();
      };
      document.head.appendChild(script);
      return;
    }
  } catch (_) {}
  if (callback) callback();
}

window.addEventListener("DOMContentLoaded", function () {
  refreshBridgeToken(checkBridgeHealth);
  setInterval(checkBridgeHealth, 10000);
});

// DOM Elements
var bridgeStatusBadge = document.getElementById("bridge-status-badge");
var bridgeStatusText = document.getElementById("bridge-status-text");
var upgradeTarget = document.getElementById("upgrade-target");
var btnStartUpgrade = document.getElementById("btn-start-upgrade");
var upgradeProgressBox = document.getElementById("upgrade-progress-box");
var upgradeProgressBar = document.getElementById("upgrade-progress-bar");
var upgradeProgressText = document.getElementById("upgrade-progress-text");
var statFound = document.getElementById("stat-found");
var statUpgraded = document.getElementById("stat-upgraded");
var statErrors = document.getElementById("stat-errors");
var btnDiskUpgrade = document.getElementById("btn-disk-upgrade");
var btnDiskUpgradeLabel = document.getElementById("btn-disk-upgrade-label");

// Bridge Health Check
function checkBridgeHealth() {
  ajaxGet(BRIDGE_URL + "/health", function (status, responseText) {
    if (status === 200) {
      var data = JSON.parse(responseText);
      bridgeStatusBadge.style.backgroundColor = "#ecfdf5";
      bridgeStatusBadge.style.borderColor = "#a7f3d0";
      bridgeStatusBadge.style.color = "#065f46";
      bridgeStatusText.textContent = "Bridge: Active (" + data.version + ")";
    } else {
      bridgeStatusBadge.style.backgroundColor = "#fef2f2";
      bridgeStatusBadge.style.borderColor = "#fecaca";
      bridgeStatusBadge.style.color = "#991b1b";
      bridgeStatusText.textContent = "Bridge: Offline";
    }
  });
}

// Instant Disk Upgrade (WPS Native FullName)
btnDiskUpgrade.addEventListener("click", function () {
  try {
    var app = window.wps ? window.wps.Application : null;
    var doc = app ? app.ActiveDocument : null;
    if (!doc || !doc.FullName || !doc.FullName.toLowerCase().endsWith(".docx")) {
      alert("Please save this document as a .docx file on disk first.");
      return;
    }

    var filePath = doc.FullName;
    doc.Save();

    btnDiskUpgrade.disabled = true;
    (btnDiskUpgradeLabel || btnDiskUpgrade).textContent = "Converting on disk...";

    var targetMode = upgradeTarget.value === "omml" ? "upgrade_omml" : "upgrade_mathtype";

    ajaxPost(
      BRIDGE_URL + "/convert/document",
      JSON.stringify({
        input_path: filePath,
        target: targetMode
      }),
      function (status, responseText) {
        btnDiskUpgrade.disabled = false;
        (btnDiskUpgradeLabel || btnDiskUpgrade).textContent = "Upgrade Active File via Rust Core";

        if (status === 200) {
          var res = JSON.parse(responseText);
          alert("Instant Conversion Done!\n\nFound: " + res.found + "\nUpgraded: " + res.converted + "\nErrors: " + res.errors);
          try {
            doc.Close(false);
            app.Documents.Open(filePath);
          } catch (_) {}
        } else {
          alert("Conversion failed: " + responseText);
        }
      }
    );
  } catch (err) {
    alert("Error: " + err);
  }
});

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

function isEquationShape(shape, xmlStr) {
  if (hasEquationInXml(xmlStr)) return true;
  if (!shape) return false;
  try {
    if (shape.AlternativeText === "EqnTide equation") return true;
  } catch (_) {}
  try {
    if (shape.OLEFormat && /^(Equation\.|MathType)/i.test(shape.OLEFormat.ProgID || "")) {
      return true;
    }
  } catch (_) {}
  return false;
}

function getTargetFontSize(range, app) {
  var size = 12.0;
  try {
    if (range && range.Font && range.Font.Size > 0) {
      size = Number(range.Font.Size);
    } else if (app && app.Selection && app.Selection.Font && app.Selection.Font.Size > 0) {
      size = Number(app.Selection.Font.Size);
    }
  } catch (_) {}
  return (size > 0 && isFinite(size)) ? size : 12.0;
}

function getDocEnd(doc) {
  if (!doc) return 0;
  try {
    if (typeof doc.Range === "function") {
      var r = doc.Range();
      if (r && typeof r.End === "number") return r.End;
    }
  } catch (_) {}
  try {
    if (doc.Content && typeof doc.Content.End === "number") return doc.Content.End;
  } catch (_) {}
  return 0;
}

function findUpgradeTargets(scope, app, doc) {
  var scanRange = null;
  var fallbackOwner = null;
  if (scope === "selection") {
    var sel = app.Selection;
    scanRange = sel && sel.Range ? sel.Range : null;
    fallbackOwner = sel || null;
  } else {
    scanRange = (doc && doc.Content) ? doc.Content : ((doc && typeof doc.Range === "function") ? doc.Range() : null);
    fallbackOwner = doc || null;
  }
  if (!scanRange) return [];

  var list = [];

  function processShape(shape) {
    if (!shape || !shape.Range) return;
    try {
      var sXml = "";
      try { sXml = shape.Range.WordOpenXML || ""; } catch (_) {}
      if (!sXml || !isEquationShape(shape, sXml)) return;
      var sStart = shape.Range.Start;
      var sEnd = shape.Range.End;
      if (typeof sStart !== "number" || typeof sEnd !== "number") return;
      if (sEnd > sStart + 1) sEnd = sStart + 1;
      list.push({
        start: sStart,
        end: sEnd,
        xml: sXml,
        fontPt: getTargetFontSize(shape.Range, app)
      });
    } catch (_) {}
  }

  function processOMath(omath) {
    if (!omath || !omath.Range) return;
    try {
      var oXml = "";
      try { oXml = omath.Range.WordOpenXML || ""; } catch (_) {}
      if (!oXml || !hasEquationInXml(oXml)) return;
      var oStart = omath.Range.Start;
      var oEnd = omath.Range.End;
      if (typeof oStart !== "number" || typeof oEnd !== "number") return;
      list.push({
        start: oStart,
        end: oEnd,
        xml: oXml,
        fontPt: getTargetFontSize(omath.Range, app)
      });
    } catch (_) {}
  }

  try {
    var shapes = scanRange.InlineShapes;
    if ((!shapes || shapes.Count === 0) && fallbackOwner && fallbackOwner.InlineShapes) {
      shapes = fallbackOwner.InlineShapes;
    }
    if (shapes && shapes.Count > 0) {
      for (var i = 1; i <= shapes.Count; i++) {
        processShape(shapes.Item(i));
      }
    }
  } catch (_) {}

  try {
    var omaths = scanRange.OMaths;
    if ((!omaths || omaths.Count === 0) && fallbackOwner && fallbackOwner.OMaths) {
      omaths = fallbackOwner.OMaths;
    }
    if (omaths && omaths.Count > 0) {
      for (var j = 1; j <= omaths.Count; j++) {
        processOMath(omaths.Item(j));
      }
    }
  } catch (_) {}

  list.sort(function (a, b) { return a.start - b.start; });

  var uniqueList = [];
  for (var k = 0; k < list.length; k++) {
    if (k === 0 || list[k].start >= uniqueList[uniqueList.length - 1].end) {
      uniqueList.push(list[k]);
    }
  }

  if (uniqueList.length === 0 && scope === "selection") {
    try {
      var selXml = scanRange.WordOpenXML || "";
      if (typeof scanRange.Start === "number" && typeof scanRange.End === "number" &&
          scanRange.End > scanRange.Start && hasEquationInXml(selXml)) {
        uniqueList.push({
          start: scanRange.Start,
          end: scanRange.End,
          xml: selXml,
          fontPt: getTargetFontSize(scanRange, app)
        });
      }
    } catch (_) {}
  }

  return uniqueList;
}

function extractLastMathText(packageXml) {
  try {
    if (typeof DOMParser !== "undefined") {
      var tree = new DOMParser().parseFromString(packageXml, "application/xml");
      var mathText = tree.getElementsByTagNameNS(
        "http://schemas.openxmlformats.org/officeDocument/2006/math",
        "t"
      );
      if (mathText && mathText.length) {
        return mathText[mathText.length - 1].textContent || "";
      }
    }
  } catch (_) {}
  var regex = /<(?:[A-Za-z_][\w.-]*:)?t\b[^>]*>([\s\S]*?)<\/(?:[A-Za-z_][\w.-]*:)?t>/g;
  var last = "";
  var match;
  while ((match = regex.exec(packageXml)) !== null) {
    last = match[1];
  }
  return last;
}

function insertUpgradedFlatOpcAtRange(doc, targetRange, convData, targetMode, fallbackFontPt) {
  var packageXml = convData && (convData.flat_opc || convData.result);
  if (!doc || !targetRange || !packageXml || typeof targetRange.InsertXML !== "function") {
    return false;
  }

  while (targetRange.End > targetRange.Start) {
    var tail = doc.Range(targetRange.End - 1, targetRange.End).Text;
    if (tail !== "\r" && tail !== "\n") break;
    targetRange = doc.Range(targetRange.Start, targetRange.End - 1);
  }

  var insertPos = targetRange.Start;
  var replaced = targetRange.End - insertPos;
  var originalEnd = getDocEnd(doc);
  var suffix = doc.Range(targetRange.End, Math.min(targetRange.End + 32, originalEnd)).Text;

  targetRange.InsertXML(packageXml);

  var newEnd = getDocEnd(doc);
  var insertedEnd = insertPos + newEnd - originalEnd + replaced;
  var isMathTypePackage = /ProgID=["']Equation\./i.test(packageXml);

  if (insertedEnd > insertPos && insertedEnd <= newEnd &&
      typeof suffix === "string" &&
      doc.Range(insertedEnd, Math.min(insertedEnd + suffix.length, newEnd)).Text === suffix) {
    for (var breaks = 0; breaks < 2 && insertedEnd > insertPos; breaks++) {
      var trailing = doc.Range(insertedEnd - 1, insertedEnd);
      if (trailing.Text !== "\r" && trailing.Text !== "\n") break;
      trailing.Delete();
      insertedEnd--;
    }

    if (targetMode === "flat_opc_omml" || !isMathTypePackage) {
      var lastMathText = extractLastMathText(packageXml);
      if (insertedEnd > insertPos && lastMathText && !/\s$/.test(lastMathText)) {
        var spacer = doc.Range(insertedEnd - 1, insertedEnd);
        if (spacer.Text === " ") {
          spacer.Delete();
          insertedEnd--;
        }
      }
    }
  }

  if (isMathTypePackage || targetMode === "flat_opc_mathtype") {
    try {
      var insertedRange = doc.Range(insertPos, insertedEnd);
      var pic = null;
      if (insertedRange.InlineShapes) {
        for (var idx = 1; idx <= insertedRange.InlineShapes.Count; idx++) {
          var candidate = insertedRange.InlineShapes.Item(idx);
          if (candidate.Range.Start === insertPos) {
            pic = candidate;
            break;
          }
        }
      }
      if (pic) {
        try { pic.AlternativeText = "EqnTide equation"; } catch (_) {}
        if (typeof convData.width_pt === "number" && convData.width_pt > 0) {
          pic.Width = convData.width_pt;
        }
        if (typeof convData.height_pt === "number" && convData.height_pt > 0) {
          pic.Height = convData.height_pt;
        }

        var docFontSize = (typeof fallbackFontPt === "number" && fallbackFontPt > 0) ? fallbackFontPt : 12.0;
        var depthPt = (typeof convData.depth_pt === "number") ? convData.depth_pt : 0;
        var pos = -Number((depthPt + docFontSize * 0.045).toFixed(1));
        var actualStart = (pic.Range && typeof pic.Range.Start === "number") ? pic.Range.Start : insertPos;
        var shapeEnd = actualStart + 1;
        var shapeRange = doc.Range(actualStart, shapeEnd);
        if (shapeRange && shapeRange.Font) {
          shapeRange.Font.Position = pos;
        }
        try {
          if (pic.Range && pic.Range.Font) pic.Range.Font.Position = pos;
        } catch (_) {}

        try {
          var para = shapeRange.Paragraphs ? shapeRange.Paragraphs.Item(1) : null;
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
            if (shapeEnd < pEnd - 1) {
              var nextChar = doc.Range(shapeEnd, shapeEnd + 1);
              var isNextEq = (nextChar.InlineShapes && nextChar.InlineShapes.Count > 0) ||
                             (nextChar.OMaths && nextChar.OMaths.Count > 0);
              if (!isNextEq && nextChar.Font && nextChar.Font.Position !== 0) {
                nextChar.Font.Position = 0;
                nextChar.Font.Subscript = false;
                nextChar.Font.Superscript = false;
              }
            }
          }
        } catch (_) {}
      }
    } catch (_) {}
  } else {
    try {
      var omathRange = doc.Range(insertPos, insertedEnd);
      var equation = null;
      if (omathRange.OMaths && omathRange.OMaths.Count > 0) {
        equation = omathRange.OMaths.Item(1);
      } else if (doc.OMaths) {
        for (var i = 1; i <= doc.OMaths.Count; i++) {
          var oCandidate = doc.OMaths.Item(i);
          if (oCandidate.Range.Start >= insertPos && oCandidate.Range.Start <= insertPos + 1) {
            equation = oCandidate;
            break;
          }
        }
      }
      if (equation) {
        equation.Type = 1;
        if (equation.Range && equation.Range.Font) equation.Range.Font.Position = 0;
      }
    } catch (_) {}
  }

  return true;
}

// In-Memory Equation Upgrader
btnStartUpgrade.addEventListener("click", function () {
  var targetMode = upgradeTarget.value === "omml" ? "flat_opc_omml" : "flat_opc_mathtype";
  var scopeEl = document.querySelector('input[name="upgrade-scope"]:checked');
  var scope = scopeEl ? scopeEl.value : "document";

  btnStartUpgrade.disabled = true;
  upgradeProgressBox.classList.remove("hidden");
  upgradeProgressBar.style.width = "10%";
  upgradeProgressText.textContent = "Scanning document...";

  var foundCount = 0;
  var upgradedCount = 0;
  var errorCount = 0;

  statFound.textContent = "0";
  statUpgraded.textContent = "0";
  statErrors.textContent = "0";

  try {
    var app = window.wps ? window.wps.Application : null;
    if (!app) {
      alert("WPS Office environment not detected.");
      btnStartUpgrade.disabled = false;
      return;
    }

    var doc = app.ActiveDocument;
    if (!doc) {
      finishUpgrade(0, 0);
      return;
    }

    var targets = findUpgradeTargets(scope, app, doc);
    if (targets.length === 0) {
      if (scope === "selection") {
        alert("No equations found in selection.");
      }
      finishUpgrade(0, 0);
      return;
    }

    foundCount = targets.length;
    statFound.textContent = String(foundCount);
    upgradeProgressBar.style.width = "20%";
    upgradeProgressText.textContent = "Converting " + foundCount + " equations...";

    var total = targets.length;
    var completed = 0;

    for (var i = 0; i < total; i++) {
      (function (item) {
        ajaxPost(
          BRIDGE_URL + "/convert/snippet",
          JSON.stringify({
            data: item.xml,
            from: "flat_opc",
            to: targetMode,
            font_pt: item.fontPt
          }),
          function (status, responseText) {
            completed++;
            var pct = Math.min(85, Math.round(20 + (65 * completed) / total));
            upgradeProgressBar.style.width = pct + "%";

            if (status === 200) {
              try {
                var parsed = JSON.parse(responseText);
                if (parsed && (parsed.flat_opc || parsed.result)) {
                  item.convData = parsed;
                } else {
                  errorCount++;
                  statErrors.textContent = String(errorCount);
                }
              } catch (_) {
                errorCount++;
                statErrors.textContent = String(errorCount);
              }
            } else {
              errorCount++;
              statErrors.textContent = String(errorCount);
            }

            if (completed === total) {
              targets.sort(function (a, b) { return b.start - a.start; });
              for (var j = 0; j < targets.length; j++) {
                var targetItem = targets[j];
                if (!targetItem.convData) continue;
                try {
                  var targetRange = doc.Range(targetItem.start, targetItem.end);
                  if (insertUpgradedFlatOpcAtRange(doc, targetRange, targetItem.convData, targetMode, targetItem.fontPt)) {
                    upgradedCount++;
                    statUpgraded.textContent = String(upgradedCount);
                  } else {
                    errorCount++;
                    statErrors.textContent = String(errorCount);
                  }
                } catch (_) {
                  errorCount++;
                  statErrors.textContent = String(errorCount);
                }
              }

              try {
                var curSel = app.Selection;
                if (curSel && curSel.Start === curSel.End && curSel.Font) {
                  curSel.Font.Position = 0;
                  curSel.Font.Subscript = false;
                  curSel.Font.Superscript = false;
                }
              } catch (_) {}

              finishUpgrade(foundCount, upgradedCount);
            }
          }
        );
      })(targets[i]);
    }
  } catch (err) {
    console.error("WPS upgrade error:", err);
    finishUpgrade(foundCount, upgradedCount);
  }
});

function finishUpgrade(found, upgraded) {
  upgradeProgressBar.style.width = "100%";
  upgradeProgressText.textContent = "Scan completed! Upgraded " + upgraded + " equations.";
  btnStartUpgrade.disabled = false;
}

function ajaxGet(url, callback) {
  var xhr = new XMLHttpRequest();
  xhr.open("GET", url, true);
  if (url.indexOf("/health") === -1) {
    var token = getBridgeToken();
    if (token) xhr.setRequestHeader(BRIDGE_TOKEN_HEADER, token);
  }
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      callback(xhr.status, xhr.responseText);
    }
  };
  xhr.send();
}

function ajaxPost(url, payload, callback) {
  var xhr = new XMLHttpRequest();
  xhr.open("POST", url, true);
  xhr.setRequestHeader("Content-Type", "application/json");
  var token = getBridgeToken();
  if (token) xhr.setRequestHeader(BRIDGE_TOKEN_HEADER, token);
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      callback(xhr.status, xhr.responseText);
    }
  };
  xhr.send(payload);
}
