// Taskpane Script for WPS Office Add-in
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

var BRIDGE_URL = "http://127.0.0.1:17890";

window.addEventListener("DOMContentLoaded", function () {
  checkBridgeHealth();
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
    btnDiskUpgrade.textContent = "Converting on disk...";

    var targetMode = upgradeTarget.value === "omml" ? "upgrade_omml" : "upgrade_mathtype";

    ajaxPost(
      BRIDGE_URL + "/convert/document",
      JSON.stringify({
        input_path: filePath,
        target: targetMode
      }),
      function (status, responseText) {
        btnDiskUpgrade.disabled = false;
        btnDiskUpgrade.textContent = "Upgrade Active File via Rust Core";

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

    if (scope === "selection") {
      var selRange = app.Selection.Range;
      var xml = selRange.WordOpenXML || "";
      if (
        xml.indexOf("Equation.3") !== -1 ||
        xml.indexOf("Equation.DSMT4") !== -1 ||
        xml.indexOf("w:object") !== -1 ||
        xml.indexOf("m:oMath") !== -1
      ) {
        foundCount++;
        statFound.textContent = "1";
        upgradeProgressBar.style.width = "50%";

        ajaxPost(
          BRIDGE_URL + "/convert/snippet",
          JSON.stringify({
            data: xml,
            from: "flat_opc",
            to: targetMode
          }),
          function (status, responseText) {
            if (status === 200) {
              var convData = JSON.parse(responseText);
              if (convData.result) {
                selRange.InsertXML(convData.result);
                upgradedCount++;
                statUpgraded.textContent = "1";
              }
            } else {
              errorCount++;
              statErrors.textContent = "1";
            }
            finishUpgrade(foundCount, upgradedCount);
          }
        );
        return;
      } else {
        alert("No equations found in selection.");
        finishUpgrade(0, 0);
        return;
      }
    }

    // Document Scope
    var doc = app.ActiveDocument;
    var paragraphs = doc.Paragraphs;
    var count = paragraphs.Count;
    upgradeProgressText.textContent = "Checking " + count + " paragraphs...";

    for (var i = 1; i <= count; i++) {
      var p = paragraphs.Item(i);
      var pXml = p.Range.WordOpenXML || "";

      if (
        pXml.indexOf("Equation.3") !== -1 ||
        pXml.indexOf("Equation.DSMT4") !== -1 ||
        pXml.indexOf("w:object") !== -1 ||
        pXml.indexOf("m:oMath") !== -1
      ) {
        foundCount++;
        statFound.textContent = String(foundCount);

        (function (paraRange, xmlContent) {
          ajaxPost(
            BRIDGE_URL + "/convert/snippet",
            JSON.stringify({
              data: xmlContent,
              from: "flat_opc",
              to: targetMode
            }),
            function (status, responseText) {
              if (status === 200) {
                var res = JSON.parse(responseText);
                if (res.result) {
                  paraRange.InsertXML(res.result);
                  upgradedCount++;
                  statUpgraded.textContent = String(upgradedCount);
                }
              } else {
                errorCount++;
                statErrors.textContent = String(errorCount);
              }
            }
          );
        })(p.Range, pXml);
      }
    }

    finishUpgrade(foundCount, upgradedCount);
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
  xhr.onreadystatechange = function () {
    if (xhr.readyState === 4) {
      callback(xhr.status, xhr.responseText);
    }
  };
  xhr.send(payload);
}
