// Taskpane Script for Word Add-in: Legacy Equations Upgrader & Tools
// Connected to OMML Converter Local Bridge (http://127.0.0.1:17890)

const BRIDGE_URL = "http://127.0.0.1:17890";

Office.onReady(() => {
  checkBridgeHealth();
  setInterval(checkBridgeHealth, 10000);
});

// DOM Elements
const bridgeStatusBadge = document.getElementById("bridge-status-badge");
const bridgeStatusText = document.getElementById("bridge-status-text");
const upgradeTarget = document.getElementById("upgrade-target");
const btnStartUpgrade = document.getElementById("btn-start-upgrade");
const upgradeProgressBox = document.getElementById("upgrade-progress-box");
const upgradeProgressBar = document.getElementById("upgrade-progress-bar");
const upgradeProgressText = document.getElementById("upgrade-progress-text");
const statFound = document.getElementById("stat-found");
const statUpgraded = document.getElementById("stat-upgraded");
const statErrors = document.getElementById("stat-errors");
const btnOpenDesktopBatch = document.getElementById("btn-open-desktop-batch");

// Bridge Health Check
async function checkBridgeHealth() {
  try {
    const res = await fetch(`${BRIDGE_URL}/health`);
    if (res.ok) {
      const data = await res.json();
      bridgeStatusBadge.style.backgroundColor = "#ecfdf5";
      bridgeStatusBadge.style.borderColor = "#a7f3d0";
      bridgeStatusBadge.style.color = "#065f46";
      bridgeStatusText.textContent = `Bridge: Active (${data.version})`;
    } else {
      throw new Error(`HTTP ${res.status}`);
    }
  } catch (_) {
    bridgeStatusBadge.style.backgroundColor = "#fef2f2";
    bridgeStatusBadge.style.borderColor = "#fecaca";
    bridgeStatusBadge.style.color = "#991b1b";
    bridgeStatusText.textContent = "Bridge: Offline";
  }
}

// Open Standalone Desktop Batch Converter
btnOpenDesktopBatch.addEventListener("click", () => {
  const iframe = document.createElement("iframe");
  iframe.style.display = "none";
  iframe.src = "eqntide://open?tab=batch";
  document.body.appendChild(iframe);
  setTimeout(() => {
    try {
      document.body.removeChild(iframe);
    } catch (_) {}
  }, 2000);
});

// In-Document Legacy Equation Upgrader
btnStartUpgrade.addEventListener("click", async () => {
  const targetMode = upgradeTarget.value === "omml" ? "flat_opc_omml" : "flat_opc_mathtype";
  const scopeEl = document.querySelector('input[name="upgrade-scope"]:checked');
  const scope = scopeEl ? scopeEl.value : "document";

  btnStartUpgrade.disabled = true;
  upgradeProgressBox.classList.remove("hidden");
  upgradeProgressBar.style.width = "10%";
  upgradeProgressText.textContent = "Scanning document for legacy equations...";

  let foundCount = 0;
  let upgradedCount = 0;
  let errorCount = 0;

  statFound.textContent = "0";
  statUpgraded.textContent = "0";
  statErrors.textContent = "0";

  try {
    await Word.run(async (context) => {
      let targetRanges;
      if (scope === "selection") {
        targetRanges = [context.document.getSelection()];
      } else {
        const paragraphs = context.document.body.paragraphs;
        paragraphs.load("items");
        await context.sync();
        targetRanges = paragraphs.items;
      }

      upgradeProgressBar.style.width = "30%";
      upgradeProgressText.textContent = `Found ${targetRanges.length} paragraphs. Checking for equations...`;

      const total = targetRanges.length;
      for (let i = 0; i < total; i++) {
        const range = targetRanges[i];
        const ooxml = range.getOoxml();
        await context.sync();

        const xml = ooxml.value || "";
        // Check if paragraph contains legacy Equation 3.0 / MathType OLE / WMF
        if (
          xml.includes("Equation.3") ||
          xml.includes("Equation.DSMT4") ||
          xml.includes("w:object") ||
          xml.includes("v:imagedata") ||
          xml.includes("m:oMath")
        ) {
          foundCount++;
          statFound.textContent = String(foundCount);

          try {
            const convRes = await fetch(`${BRIDGE_URL}/convert/snippet`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                data: xml,
                from: "flat_opc",
                to: targetMode
              })
            });

            if (convRes.ok) {
              const convData = await convRes.json();
              if (convData.result && convData.result.includes("pkg:package")) {
                range.insertOoxml(convData.result, Word.InsertLocation.replace);
                await context.sync();
                upgradedCount++;
                statUpgraded.textContent = String(upgradedCount);
              }
            } else {
              errorCount++;
              statErrors.textContent = String(errorCount);
            }
          } catch (e) {
            console.warn("Conversion failed for paragraph:", e);
            errorCount++;
            statErrors.textContent = String(errorCount);
          }
        }

        const pct = Math.min(95, Math.round(30 + ((i + 1) / total) * 65));
        upgradeProgressBar.style.width = `${pct}%`;
        upgradeProgressText.textContent = `Processing paragraph ${i + 1} of ${total}...`;
      }

      upgradeProgressBar.style.width = "100%";
      upgradeProgressText.textContent = `Upgrade completed! ${upgradedCount} equations upgraded.`;
    });
  } catch (err) {
    console.error("Upgrade error:", err);
    upgradeProgressText.textContent = `Error during upgrade: ${err.message || err}`;
  } finally {
    btnStartUpgrade.disabled = false;
  }
});
