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
        var homePath = wps.Env.GetHomePath();
        candidates.push(homePath + "/.local/share/eqntide/bridge.token");
        candidates.push(homePath + "/Library/Application Support/com.eqntide.EqnTide/bridge.token");
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

function isWindowsClient(filePath) {
  try {
    if (typeof navigator !== "undefined") {
      if (/win/i.test(navigator.platform || "") || /windows/i.test(navigator.userAgent || "")) {
        return true;
      }
    }
  } catch (_) {}
  if (typeof filePath === "string" && (/^[a-zA-Z]:[\\/]/.test(filePath) || /^\\\\/.test(filePath))) {
    return true;
  }
  return false;
}

function isDocumentOpenInApp(app, filePath) {
  try {
    if (!app || !app.Documents || !filePath) return false;
    var target = String(filePath).toLowerCase();
    for (var i = 1; i <= app.Documents.Count; i++) {
      var d = app.Documents.Item(i);
      if (d && d.FullName && String(d.FullName).toLowerCase() === target) {
        return true;
      }
    }
  } catch (_) {}
  return false;
}

function reopenDocumentInApp(app, filePath) {
  var targetApp = app;
  try {
    if (!targetApp || !targetApp.Documents || typeof targetApp.Documents.Open !== "function") {
      if (window.wps) {
        targetApp = (typeof window.wps.WpsApplication === "function") ? window.wps.WpsApplication() : window.wps.Application;
      }
    }
  } catch (_) {}
  var opened = targetApp.Documents.Open(filePath);
  try {
    if (opened && typeof opened.Activate === "function") opened.Activate();
  } catch (_) {}
  return opened;
}

// Instant Disk Upgrade (WPS Native FullName)
btnDiskUpgrade.addEventListener("click", function () {
  var tempBlankDoc = null;
  try {
    var app = window.wps ? window.wps.Application : null;
    var doc = app ? app.ActiveDocument : null;
    if (!doc || !doc.FullName || !doc.FullName.toLowerCase().endsWith(".docx")) {
      alert("Please save this document as a .docx file on disk first.");
      return;
    }
    if (doc.ReadOnly) {
      alert("This document is read-only. Save a writable copy before upgrading it.");
      return;
    }

    var filePath = doc.FullName;
    var needsPreClose = isWindowsClient(filePath);
    doc.Save();
    if (doc.Saved === false) {
      alert("WPS did not save the document. Upgrade cancelled.");
      return;
    }
    if (needsPreClose) {
      // Keep WPS Writer & this taskpane alive when closing the only open document.
      if (app.Documents && app.Documents.Count <= 1 && typeof app.Documents.Add === "function") {
        try { tempBlankDoc = app.Documents.Add(); } catch (_) { tempBlankDoc = null; }
      }
      doc.Close(false);
      if (isDocumentOpenInApp(app, filePath)) {
        try { if (tempBlankDoc) tempBlankDoc.Close(false); } catch (_) {}
        alert("WPS did not close the document. Close it manually or use Scan & Upgrade Equations above.");
        return;
      }
    }

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

        if (needsPreClose) {
          var reopenErr = null;
          try {
            reopenDocumentInApp(app, filePath);
          } catch (openErr) {
            reopenErr = openErr;
          }
          try { if (tempBlankDoc) tempBlankDoc.Close(false); } catch (_) {}

          if (status === 200) {
            var res = JSON.parse(responseText);
            var okMsg = "Instant Conversion Done!\n\nFound: " + res.found + "\nUpgraded: " + res.converted + "\nErrors: " + res.errors;
            if (reopenErr) okMsg += "\n\nPlease reopen the file at: " + filePath;
            alert(okMsg);
          } else {
            var msg = "Conversion failed: " + responseText;
            if (reopenErr) msg += "\n\nOpen the saved document at: " + filePath;
            alert(msg);
          }
        } else {
          if (status === 200) {
            var resLinux = JSON.parse(responseText);
            alert("Instant Conversion Done!\n\nFound: " + resLinux.found + "\nUpgraded: " + resLinux.converted + "\nErrors: " + resLinux.errors);
            try {
              doc.Close(false);
              reopenDocumentInApp(app, filePath);
            } catch (_) {}
          } else {
            alert("Conversion failed: " + responseText);
          }
        }
      }
    );
  } catch (err) {
    try { if (tempBlankDoc) tempBlankDoc.Close(false); } catch (_) {}
    btnDiskUpgrade.disabled = false;
    (btnDiskUpgradeLabel || btnDiskUpgrade).textContent = "Upgrade Active File via Rust Core";
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

function isBloatedFlatOpcPart(lowerPartName) {
  if (!lowerPartName) return false;
  if (
    lowerPartName === "/word/styles.xml" ||
    lowerPartName === "/word/styleswitheffects.xml" ||
    lowerPartName === "/word/numbering.xml" ||
    lowerPartName === "/word/fonttable.xml" ||
    lowerPartName === "/word/settings.xml" ||
    lowerPartName === "/word/websettings.xml"
  ) {
    return true;
  }
  return (
    lowerPartName.indexOf("/word/theme/") === 0 ||
    lowerPartName.indexOf("/docprops/") === 0 ||
    lowerPartName.indexOf("/customxml/") === 0
  );
}

function compactFlatOpcXml(xml) {
  if (!xml || typeof xml !== "string" || xml.length < 16384) return xml;
  var firstPart = xml.indexOf("<pkg:part ");
  if (firstPart === -1) return xml;
  var out = [xml.slice(0, firstPart)];
  var pos = firstPart;
  var tailPos = firstPart;
  while (pos !== -1) {
    var tagEnd = xml.indexOf(">", pos);
    if (tagEnd === -1) return xml;
    var partEnd;
    if (xml.charAt(tagEnd - 1) === "/") {
      partEnd = tagEnd + 1;
    } else {
      var closeIdx = xml.indexOf("</pkg:part>", tagEnd + 1);
      if (closeIdx === -1) return xml;
      partEnd = closeIdx + 11;
    }
    var header = xml.slice(pos, tagEnd + 1);
    var m = /(?:pkg:)?name=["']([^"']+)["']/i.exec(header);
    var partName = m ? m[1].toLowerCase() : "";
    if (partName && partName.charAt(0) !== "/") partName = "/" + partName;
    if (!isBloatedFlatOpcPart(partName)) {
      out.push(xml.slice(pos, partEnd));
    }
    tailPos = partEnd;
    pos = xml.indexOf("<pkg:part ", partEnd);
  }
  if (tailPos < xml.length) out.push(xml.slice(tailPos));
  return out.join("");
}

function quickClassifyShape(shape) {
  if (!shape) return 0;
  try {
    if (shape.AlternativeText === "EqnTide equation") return 1;
  } catch (_) {}
  var st = 0;
  try {
    if (typeof shape.Type === "number") st = shape.Type;
  } catch (_) {}
  if (st === 1 || st === 2) {
    try {
      var progId = shape.OLEFormat && shape.OLEFormat.ProgID ? String(shape.OLEFormat.ProgID).toLowerCase() : "";
      if (progId) {
        return (progId.indexOf("equation.") === 0 || progId.indexOf("mathtype") === 0) ? 1 : -1;
      }
    } catch (_) {}
    return 0;
  }
  if (st === 3 || st === 4 || st === 0) return 0;
  return -1;
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
  var candidates = [];
  try {
    if (range && range.Font) candidates.push(Number(range.Font.Size));
  } catch (_) {}
  try {
    if (app && app.Selection && app.Selection.Font) candidates.push(Number(app.Selection.Font.Size));
  } catch (_) {}
  for (var i = 0; i < candidates.length; i++) {
    var size = candidates[i];
    if (typeof size === "number" && isFinite(size) && size >= 1.0 && size <= 200.0) {
      return size;
    }
  }
  return 12.0;
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
      var cls = quickClassifyShape(shape);
      if (cls < 0) return;
      var sXml = "";
      try { sXml = shape.Range.WordOpenXML || ""; } catch (_) {}
      if (!sXml || (cls !== 1 && !isEquationShape(shape, sXml))) return;
      var sStart = shape.Range.Start;
      var sEnd = shape.Range.End;
      if (typeof sStart !== "number" || typeof sEnd !== "number") return;
      if (sEnd > sStart + 1) sEnd = sStart + 1;
      list.push({
        start: sStart,
        end: sEnd,
        xml: compactFlatOpcXml(sXml),
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
        xml: compactFlatOpcXml(oXml),
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

function findInsertedShapeByPos(doc, insertedRange, insertPos) {
  var pic = null;
  var bestDiff = 999999;
  var shapeCollections = [];
  if (insertedRange && insertedRange.InlineShapes) shapeCollections.push(insertedRange.InlineShapes);
  if (doc && doc.InlineShapes) shapeCollections.push(doc.InlineShapes);
  try {
    if (doc && doc.Content && doc.Content.InlineShapes) shapeCollections.push(doc.Content.InlineShapes);
  } catch (_) {}
  for (var c = 0; c < shapeCollections.length; c++) {
    var shapes = shapeCollections[c];
    var count = shapes && shapes.Count ? shapes.Count : 0;
    if (!count) continue;
    if (count > 16) {
      var lo = 1, hi = count;
      while (lo <= hi) {
        var mid = (lo + hi) >> 1;
        var midItem = shapes.Item(mid);
        if (!midItem || !midItem.Range || typeof midItem.Range.Start !== "number") break;
        var midDiff = midItem.Range.Start - insertPos;
        if (midDiff >= 0 && midDiff <= 5 && midDiff < bestDiff) {
          bestDiff = midDiff;
          pic = midItem;
          if (midDiff === 0) return pic;
        }
        if (midItem.Range.Start < insertPos) lo = mid + 1;
        else hi = mid - 1;
      }
      var winStart = Math.max(1, lo - 3);
      var winEnd = Math.min(count, lo + 3);
      for (var wi = winStart; wi <= winEnd; wi++) {
        var wCand = shapes.Item(wi);
        if (!wCand || !wCand.Range || typeof wCand.Range.Start !== "number") continue;
        var wDiff = wCand.Range.Start - insertPos;
        if (wDiff >= 0 && wDiff <= 5 && wDiff < bestDiff) {
          bestDiff = wDiff;
          pic = wCand;
          if (wDiff === 0) return pic;
        }
      }
      if (pic) return pic;
    }
    for (var idx = 1; idx <= count; idx++) {
      var candidate = shapes.Item(idx);
      if (!candidate || !candidate.Range || typeof candidate.Range.Start !== "number") continue;
      var diff = candidate.Range.Start - insertPos;
      if (diff >= 0 && diff <= 5 && diff < bestDiff) {
        bestDiff = diff;
        pic = candidate;
        if (diff === 0) return pic;
      }
    }
    if (pic) return pic;
  }
  return null;
}

function findInsertedOMathByPos(doc, omathRange, insertPos) {
  if (omathRange && omathRange.OMaths && omathRange.OMaths.Count > 0) {
    return omathRange.OMaths.Item(1);
  }
  var omaths = doc && doc.OMaths ? doc.OMaths : null;
  var count = omaths && omaths.Count ? omaths.Count : 0;
  if (!count) return null;
  if (count > 16) {
    var lo = 1, hi = count;
    while (lo <= hi) {
      var mid = (lo + hi) >> 1;
      var midItem = omaths.Item(mid);
      if (!midItem || !midItem.Range || typeof midItem.Range.Start !== "number") break;
      var mStart = midItem.Range.Start;
      if (mStart >= insertPos && mStart <= insertPos + 1) return midItem;
      if (mStart < insertPos) lo = mid + 1;
      else hi = mid - 1;
    }
  }
  for (var i = 1; i <= count; i++) {
    var oCandidate = omaths.Item(i);
    if (oCandidate && oCandidate.Range && oCandidate.Range.Start >= insertPos && oCandidate.Range.Start <= insertPos + 1) {
      return oCandidate;
    }
  }
  return null;
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
  var origLeadingBreaks = 0;
  while (origLeadingBreaks < 2 && targetRange.End + origLeadingBreaks < originalEnd) {
    try {
      var bCh = doc.Range(targetRange.End + origLeadingBreaks, targetRange.End + origLeadingBreaks + 1).Text;
      if (bCh !== "\r" && bCh !== "\n") break;
      origLeadingBreaks++;
    } catch (_) {
      break;
    }
  }
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
      newEnd = getDocEnd(doc);
      if (insertedEnd < insertPos) insertedEnd = insertPos;
      if (insertedEnd > newEnd) insertedEnd = newEnd;
      var insertedRange = doc.Range(insertPos, insertedEnd);
      var pic = findInsertedShapeByPos(doc, insertedRange, insertPos);
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
        var shapeEnd = (pic.Range && typeof pic.Range.End === "number" && pic.Range.End > actualStart)
          ? pic.Range.End
          : (actualStart + 1);
        var curBreaks = 0;
        var curEnd = getDocEnd(doc);
        while (curBreaks < origLeadingBreaks + 2 && shapeEnd + curBreaks < curEnd) {
          try {
            var aCh = doc.Range(shapeEnd + curBreaks, shapeEnd + curBreaks + 1).Text;
            if (aCh !== "\r" && aCh !== "\n") break;
            curBreaks++;
          } catch (_) {
            break;
          }
        }
        for (var eb = 0; eb < curBreaks - origLeadingBreaks; eb++) {
          try { doc.Range(shapeEnd, shapeEnd + 1).Delete(); } catch (_) { break; }
        }
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
            if (pEnd > shapeEnd) {
              var pMark = doc.Range(pEnd - 1, pEnd);
              if (pMark && (pMark.Text === "\r" || pMark.Text === "\n") && pMark.Font) {
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
      var equation = findInsertedOMathByPos(doc, omathRange, insertPos);
      if (equation) {
        equation.Type = 1;
        if (equation.Range && equation.Range.Font) equation.Range.Font.Position = 0;
      }
    } catch (_) {}
  }

  return true;
}

var TASKPANE_HTTP_CONCURRENCY = 8;
var TASKPANE_DOM_CHUNK_SIZE = 10;

function runBoundedQueue(items, maxConcurrent, worker, onDone) {
  var total = items.length;
  if (total === 0) {
    onDone();
    return;
  }
  var nextIndex = 0;
  var finished = 0;
  var inFlight = 0;
  var concurrency = Math.max(1, Math.min(maxConcurrent, total));

  function pump() {
    while (inFlight < concurrency && nextIndex < total) {
      var idx = nextIndex++;
      inFlight++;
      worker(items[idx], idx, function () {
        inFlight--;
        finished++;
        if (finished === total) {
          onDone();
        } else {
          pump();
        }
      });
    }
  }
  pump();
}

function applyItemsInChunks(items, chunkSize, applyItem, onDone) {
  var total = items.length;
  var idx = 0;
  var canYield = (typeof setTimeout === "function") && total > chunkSize;

  function step() {
    var end = Math.min(total, idx + chunkSize);
    while (idx < end) {
      applyItem(items[idx], idx);
      idx++;
    }
    if (idx < total) {
      if (canYield) {
        setTimeout(step, 0);
      } else {
        step();
      }
    } else {
      onDone();
    }
  }
  step();
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

    runBoundedQueue(
      targets,
      TASKPANE_HTTP_CONCURRENCY,
      function (item, _idx, done) {
        var payloadXml = item.xml;
        item.xml = null;
        ajaxPost(
          BRIDGE_URL + "/convert/snippet",
          JSON.stringify({
            data: payloadXml,
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
            done();
          }
        );
      },
      function () {
        targets.sort(function (a, b) { return b.start - a.start; });
        applyItemsInChunks(
          targets,
          TASKPANE_DOM_CHUNK_SIZE,
          function (targetItem) {
            if (!targetItem.convData) return;
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
            targetItem.convData = null;
          },
          function () {
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
        );
      }
    );
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
