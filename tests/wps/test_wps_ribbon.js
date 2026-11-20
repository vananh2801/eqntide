// Run via npm test --prefix tests/wps. No WPS process is controlled by this test.
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const { DOMParser } = require('@xmldom/xmldom');
function setup(saved = null) {
  let nextTimer = 0;
  const jobs = new Map();
  const storage = new Map(saved === null ? [] : [['eqntide.autoEquationTab', saved]]);
  const calls = [];
  const c = vm.createContext({
    console: { log() {}, warn() {} },
    setTimeout(fn) { const id = ++nextTimer; jobs.set(id, fn); return id; },
    clearTimeout(id) { jobs.delete(id); },
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) },
    atob: value => Buffer.from(value, 'base64').toString('latin1')
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), c);
  jobs.clear();
  c.wpsLog = () => {};
  const ribbon = { ActivateTab(id) { calls.push(id); }, InvalidateControl() {} };
  c.wps = { Application: { ActiveDocument: { FullName: '/sample.docx' }, Selection: null } };
  c.OnAddinLoad(ribbon);
  function select(kind, start = 5) {
    const picture = { Range: { Start: start, End: start + 1, WordOpenXML: '' } };
    if (kind === 'tagged') picture.AlternativeText = 'EqnTide equation';
    if (kind === 'ole') picture.OLEFormat = { ProgID: 'Equation.DSMT4' };
    if (kind === 'xml-ole') picture.Range.WordOpenXML = '<o:OLEObject ProgID="Equation.3"/>';
    if (kind === 'emf' || kind === 'plain-emf') {
      picture.Range.WordOpenXML = '<pkg:binaryData>' + Buffer.from(kind === 'emf' ? '\x00AppsMFCC\x01' : 'ordinary EMF').toString('base64') + '</pkg:binaryData>';
    }
    const selection = { Start: start, End: start + 1, InlineShapes: { Count: 1, Item: () => picture } };
    if (kind === 'cursor') selection.End = start;
    if (kind === 'paragraph') selection.End += 10;
    if (kind === 'multiple') selection.InlineShapes.Count = 2;
    if (kind === 'omml') selection.InlineShapes.Count = 0;
    c.wps.Application.Selection = selection;
    c.onWindowSelectionChange(selection);
    return selection;
  }
  function flush() { const batch = [...jobs.values()]; jobs.clear(); batch.forEach(fn => fn()); }
  return { c, ribbon, calls, storage, jobs, select, flush };
}
for (const kind of ['tagged', 'ole', 'xml-ole', 'emf']) {
  const t = setup(); t.select(kind);
  assert.equal(t.calls.length, 0, 'activation must be deferred');
  t.flush(); assert.deepEqual(t.calls, ['TabEqnTide']);
  t.c.onWindowSelectionChange(t.c.wps.Application.Selection); t.flush();
  assert.equal(t.calls.length, 1, 'do not fight manual tab changes on same selection');
  t.select('cursor'); t.flush(); t.select(kind); t.flush();
  assert.equal(t.calls.length, 2);
}
for (const kind of ['ordinary', 'plain-emf', 'cursor', 'paragraph', 'multiple', 'omml']) {
  const t = setup(); t.select(kind); t.flush(); assert.equal(t.calls.length, 0, kind);
}
{
  const t = setup();
  const selection = t.select('tagged');
  selection.Range = { InlineShapes: selection.InlineShapes };
  selection.InlineShapes = { Count: 0 };
  t.flush(); assert.deepEqual(t.calls, ['TabEqnTide'], 'support Range.InlineShapes fallback');
}
{
  const t = setup('false');
  t.select('tagged'); t.flush();
  assert.deepEqual(t.calls, ['TabEqnTide'], 'legacy disabled setting must be ignored');
  assert.equal(t.storage.get('eqntide.autoEquationTab'), 'false');
}
{
  const t = setup(); t.select('tagged'); t.select('ordinary', 20); t.flush();
  assert.equal(t.calls.length, 0, 'cancel stale selection');
  t.select('tagged'); t.c.wps.Application.ActiveDocument.FullName = '/other.docx'; t.flush();
  assert.equal(t.calls.length, 0, 'cancel stale document');
}
{
  const t = setup();
  t.ribbon.ActivateTab = id => { t.calls.push(id); t.c.onWindowSelectionChange(t.c.wps.Application.Selection); };
  t.select('tagged'); t.flush(); t.flush(); assert.equal(t.calls.length, 1, 'no recursive activation');
}
{
  const t = setup(); delete t.ribbon.ActivateTab;
  t.select('tagged'); assert.doesNotThrow(t.flush);
}
// No user-facing switch or storage callbacks remain.
const xml = new DOMParser().parseFromString(fs.readFileSync(path.join(__dirname, '../../wps-addin/ribbon.xml'), 'utf8'), 'application/xml');
const toggle = Array.from(xml.getElementsByTagName('toggleButton')).find(n => n.getAttribute('id') === 'btnAutoEquationTab');
assert.equal(toggle, undefined);
assert.equal(setup().c.OnAutoEquationTab, undefined);

// Ribbon GetImage resolves distinct PNG assets per control ID and taskpane renders Tabler SVGs.
{
  const { c } = setup();
  const expected = {
    btnInsertEquation: 'assets/icons/pencil-plus-32.png',
    btnEditEquation: 'assets/icons/edit-32.png',
    btnToggleLatex: 'assets/icons/tex-32.png',
    btnUpgradeLegacy: 'assets/icons/arrow-big-up-lines-32.png',
    btnBatchConverter: 'assets/icons/file-type-docx-32.png'
  };
  for (const [id, relPath] of Object.entries(expected)) {
    assert.equal(c.GetImage({ Id: id }), relPath, `GetImage(${id})`);
    assert.ok(fs.existsSync(path.join(__dirname, '../../wps-addin', relPath)), `Missing ${relPath}`);
  }
  assert.equal(c.GetImage({ Id: 'unknown' }), 'assets/icon-32.png');

  const taskpaneHtml = fs.readFileSync(path.join(__dirname, '../../wps-addin/taskpane.html'), 'utf8');
  assert.match(taskpaneHtml, /id="btn-start-upgrade"[\s\S]*?<svg[\s\S]*?<\/svg>/);
  assert.match(taskpaneHtml, /id="btn-disk-upgrade"[\s\S]*?<svg[\s\S]*?<\/svg>/);
}

// Bridge token header X-EqnTide-Token is sent on API calls (and skipped on /health).
{
  function makeXhrCtx(sourceRel) {
    const sent = [];
    class MockXhr {
      open(method, url) { this.method = method; this.url = url; this.headers = {}; }
      setRequestHeader(k, v) { this.headers[k] = v; }
      send() { sent.push({ method: this.method, url: this.url, headers: this.headers }); }
    }
    const ctx = vm.createContext({
      console,
      XMLHttpRequest: MockXhr,
      setTimeout() { return 1; },
      clearTimeout() {},
      setInterval() { return 1; },
      window: { addEventListener() {}, EQNTIDE_BRIDGE_TOKEN: 'secret-token-value' },
      document: { getElementById: () => ({ addEventListener() {} }) }
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin', sourceRel), 'utf8'), ctx);
    return { ctx, sent };
  }

  for (const rel of ['main.js', 'taskpane.js']) {
    const { ctx, sent } = makeXhrCtx(rel);
    ctx.ajaxGet('http://127.0.0.1:17890/health', () => {});
    ctx.ajaxGet('http://127.0.0.1:17890/session/abc/wait', () => {});
    ctx.ajaxPost('http://127.0.0.1:17890/convert/snippet', '{}', () => {});
    assert.equal(sent[0].headers['X-EqnTide-Token'], undefined, `${rel} /health omits token`);
    assert.equal(sent[1].headers['X-EqnTide-Token'], 'secret-token-value', `${rel} GET includes token`);
    assert.equal(sent[2].headers['X-EqnTide-Token'], 'secret-token-value', `${rel} POST includes token`);
  }
}
console.log('WPS equation tab regression tests passed');
