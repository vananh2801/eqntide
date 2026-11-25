// Install test-only XML DOM: npm ci --prefix tests/wps
// Run with: npm test --prefix tests/wps
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { DOMParser, XMLSerializer } = require('@xmldom/xmldom');
const nsM = 'http://schemas.openxmlformats.org/officeDocument/2006/math';
const nsW = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main';
function controller() {
  const context = vm.createContext({ console, DOMParser, XMLSerializer, alert: message => { throw Error(message); } });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), context);
  context.wpsLog = () => {};
  return context;
}

function mockToggleProgress(c) {
  const pane = { Visible: false };
  const messages = [];
  c.URL = URL;
  c.location = { href: 'file:///EqnTide/index.html' };
  c.wps.CreateTaskPane = url => {
    assert.equal(url, 'file:///EqnTide/toggle-progress.html');
    return pane;
  };
  c.wps.PluginStorage = {
    setItem(_key, message) { messages.push(message); },
    removeItem() { messages.push(null); }
  };
  return { pane, messages };
}

{
  const c = controller();
  const fragment = String.raw`0,25\text{\$}$`;
  assert.equal(c.findAllLatexMatches(fragment).length, 0);
  assert.equal(c.findEnclosingLatexInPara(fragment, fragment.length - 2), null);
  const valid = String.raw`$0,25\text{\$}$`;
  assert.equal(c.findAllLatexMatches(fragment + '\r' + valid)[0].raw, valid);
  let viewportPage = 9;
  let refreshes = 0;
  const zoom = { set Percentage(_value) { viewportPage = 1; } };
  const doc = { Repaginate() { viewportPage = 1; } };
  c.wps = { Application: {
    ActiveDocument: doc,
    ActiveWindow: { ActivePane: { View: { Zoom: zoom } } },
    ScreenRefresh() { refreshes++; }
  } };
  c.refreshToggleScreen({});
  c.refreshToggleScreen(doc);
  assert.equal(refreshes, 1);
  assert.equal(viewportPage, 9, 'refresh should not repaginate or reset the zoom');
}
// A partial toggle may repaint, but must leave document-wide screen and layout state alone.
{
  const c = controller();
  const timers = [];
  const calls = [];
  const doc = { Repaginate() { calls.push('repaginate'); } };
  const view = {
    set VerticalPercentScrolled(_value) { calls.push('scroll'); },
    ActivePane: { View: { Zoom: { set Percentage(_value) { calls.push('zoom'); } } } }
  };
  c.wps = { Application: {
    ActiveDocument: doc,
    ActiveWindow: view,
    get ScreenUpdating() { return true; },
    set ScreenUpdating(value) { calls.push(value); },
    ScreenRefresh() { calls.push('refresh'); }
  } };
  c.setTimeout = fn => timers.push(fn);
  c.refreshToggleScreen(doc);
  assert.deepEqual(calls, []);
  timers.shift()();
  assert.deepEqual(calls, ['refresh']);

  calls.length = 0;
  c.refreshToggleScreen(doc);
  c.wps.Application.ActiveDocument = {};
  timers.shift()();
  assert.deepEqual(calls, [], 'skip redraw if the user switched documents');
}
// Toggle must recognize bare \\dfrac as well as delimited formulas.
for (const source of [String.raw`\dfrac{1}{2}`, String.raw`$\dfrac{1}{2}$`, String.raw`\[\dfrac{1}{2}\]`]) {
  const c = controller();
  const selection = { Start: 10, End: 10 + source.length, Text: source };
  const doc = {};
  let captured;
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = callback => callback();
  c.batchLatexToMathType = (actualDoc, actualSelection, matches) => {
    assert.equal(actualDoc, doc);
    assert.equal(actualSelection, selection);
    captured = matches;
  };
  c.ToggleLatex();
  assert.equal(captured.length, 1);
  assert.equal(captured[0].latex, String.raw`\dfrac{1}{2}`);
}

// Model WPS's observed behavior: packages create native equations; raw
// oMath/XML is literal text. Exercise extra breaks and existing paragraph marks.
for (const editing of [false, true]) {
for (const addedBreak of ['', '\r', '\n', '\r\n']) {
for (const addedSpace of ['', ' ']) {
for (const omathMode of ['normal', 'includes-break', 'unavailable', 'throws']) {
for (const suffix of [', after equation.\r', '\rNext paragraph.\r', ' , keep whitespace.\r', ' ', '\r', '']) {
  const c = controller();
  const original = 'Start ' + (editing ? '#' : '') + suffix;
  let text = original;
  let cursor;
  let received;
  let equation = null;
  const selection = { Font: { Position: -8 } };
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start, End: end, Font: {},
        get Text() { return text.slice(this.Start, this.End); },
        get OMaths() {
          if (omathMode === 'throws') throw Error('OMaths unavailable');
          if (omathMode === 'unavailable') return undefined;
          return { Count: equation && start < 8 ? 1 : 0, Item: () => equation };
        },
        Select() { cursor = this.Start; },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML(xml) {
          received = xml;
          const packageInput = xml.includes('<pkg:package');
          const replacement = packageInput ? 'EQ' + addedSpace + addedBreak : xml;
          text = text.slice(0, this.Start) + replacement + text.slice(this.End);
          this.End = this.Start + replacement.length;
          if (packageInput) equation = {
            Type: 0, Range: { Start: start, End: start + 2 + (omathMode === 'includes-break' ? addedBreak.length : 0), Font: { Position: -8 } }
          };
        }
      };
    }
  };
  selection.Range = doc.Range(6, editing ? 7 : 6);
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  if (editing) { c.g_editRangeStart = 6; c.g_editRangeEnd = 7; }
  c.InsertFlatOpcToSelection = () => { throw Error('OMML must use its dedicated package handler'); };
  const omml = '<m:oMath xmlns:m="' + nsM + '"><m:r><m:t>x</m:t></m:r></m:oMath>';
  const flat_opc = '<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage"><w:p xmlns:w="' + nsW + '">' + omml + '</w:p></pkg:package>';
  c.HandleSessionResult({ status: 'ready', omml, flat_opc });
  assert.equal(received, flat_opc);
  assert.ok(equation, 'must insert a native equation, not literal XML');
  if (omathMode === 'normal' || omathMode === 'includes-break') {
    assert.equal(equation.Type, 1);
    assert.equal(equation.Range.Font.Position, 0);
  }
  assert.equal(cursor, 8);
  assert.equal(selection.Font.Position, 0);
  assert.equal(text, 'Start EQ' + suffix, 'preserve original suffix and paragraph marks');
}
}
}
}
}
// MathType must import the OLE+EMF package even if an emf_path is supplied.
for (const editing of [false, true]) {
for (const font of [12, 30]) {
for (const depth of [0, 4.24, 9.75]) {
  const c = controller();
  const shapeFont = {};
  let text = 'abc' + (editing ? '#' : '') + ',suffix';
  let imported = null;
  const pic = { Range: { Start: 3, End: 4, Font: shapeFont } };
  const doc = { Range(start, end) {
    if (start === undefined) return { End: text.length };
    return { Start: start, End: end, Font: shapeFont,
      get Text() { return text.slice(start, end); },
      Delete() { text = text.slice(0, start) + text.slice(end); },
      InsertXML(xml) { imported = xml; text = text.slice(0, start) + '#\r' + text.slice(end); },
      InlineShapes: { Count: 1, Item: () => pic,
        AddPicture() { throw Error('must retain OLE, not insert only EMF'); } },
      Paragraphs: { Item: () => null }
    };
  } };
  const target = doc.Range(3, editing ? 4 : 3);
  const packageXml = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/><pkg:binaryData>OLE+EMF</pkg:binaryData></pkg:package>';
  const result = c.insertMathTypeAtDocRange(doc, target,
    { result: packageXml, emf_path: '/test.emf', depth_pt: depth, font_pt: font }, false);
  assert.equal(result, pic);
  assert.equal(imported, packageXml);
  assert.equal(text, 'abc#,suffix');
  assert.equal(shapeFont.Position, -Number((depth + font * 0.045).toFixed(1)));
}
}
}
{
  const c = controller();
  let called = false;
  c.InsertMathTypeEquation = data => { called = true; assert.ok(data.flat_opc); return true; };
  c.InsertOmmlToSelection = () => { throw Error('must select MathType package even without emf_path'); };
  c.HandleSessionResult({ status: 'ready', flat_opc: '<o:OLEObject ProgID="Equation.DSMT4"/>', omml: '<m:oMath/>' });
  assert.ok(called);
}
console.log('WPS insertion regression tests passed');

// Use the user's saved XML: it contains a real $...$ run inside oMath,
// a separate spacer after x, and bookmarks immediately after the second math.
const documentXml = fs.readFileSync(path.join(__dirname, '../fixtures/bug_toggle_v1.xml'), 'utf8');
const source = '<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage"><pkg:part pkg:name="/word/document.xml"><pkg:xmlData>' +
  documentXml.replace(/<\?xml[^?]*\?>/, '') + '</pkg:xmlData></pkg:part></pkg:package>';
const c = controller();
const converted = c.ommlPackageToPlainText(source, 1, '$\\frac{1}{2}x=3$');
const parsed = new DOMParser().parseFromString(converted, 'application/xml');
assert.equal(parsed.getElementsByTagNameNS(nsM, 'oMath').length, 1);
assert.equal(parsed.getElementsByTagNameNS(nsM, 't')[0].textContent, 'x');
const plain = Array.from(parsed.getElementsByTagNameNS(nsW, 't'));
assert.ok(plain.some(t => t.textContent === '$\\frac{1}{2}x=3$'));
assert.ok(plain.some(t => t.textContent === ' '), 'existing spaces must be preserved');
assert.equal(parsed.getElementsByTagNameNS(nsW, 'bookmarkStart').length, 1);
assert.equal(parsed.getElementsByTagNameNS(nsW, 'bookmarkEnd').length, 1);
assert.equal(parsed.getElementsByTagNameNS(nsW, 'p').length,
  new DOMParser().parseFromString(source, 'application/xml').getElementsByTagNameNS(nsW, 'p').length);

// Cursor and batch paths must import the enclosing paragraph, even when WPS's
// Remove API is a no-op. Direct Text assignment deliberately retains the math.
for (const branch of ['single', 'selected', 'batch']) {
  const c = controller();
  let mathPresent = true;
  let imported = false;
  let cursor;
  const equation = { Range: { Start: 20, End: 35 }, Remove() {} };
  const paragraph = { Start: 0, End: 37, WordOpenXML: source,
    OMaths: { Count: 2, Item: i => i === 2 ? equation : { Range: { Start: 4, End: 5 } } } };
  const doc = { Range(start, end) {
    return { Start: start, End: end, Font: {}, WordOpenXML: '<m:oMath/>',
      OMaths: { Count: mathPresent && start >= 20 ? 1 : 0, Item: () => equation },
      Paragraphs: { Item: () => ({ Range: paragraph }) },
      get Text() { return ''; },
      set Text(value) { throw Error('must not assign Text inside OMML'); },
      InsertXML(xml) {
        assert.equal(start, 0); assert.equal(end, 37);
        const tree = new DOMParser().parseFromString(xml, 'application/xml');
        assert.equal(tree.getElementsByTagNameNS(nsM, 'oMath').length, 1);
        assert.ok(Array.from(tree.getElementsByTagNameNS(nsW, 't')).some(t => t.textContent === '$\\frac{1}{2}x=3$'));
        imported = true; mathPresent = false;
      },
      Select() { cursor = start; }
    };
  } };
  const selection = { Start: 21, End: 21, Font: {}, Collapse() {} };
  selection.Range = doc.Range(21, 21);
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = callback => callback();
  c.ajaxPost = (url, body, callback) => callback(200, JSON.stringify({ result: '\\frac{1}{2}x=3' }));
  const targets = [{ start: 20, end: 35, xml: '<m:oMath/>', isDisplay: false }];
  if (branch === 'single') {
    c.findEquationTarget = () => doc.Range(20, 35);
    c.ToggleLatex();
    assert.equal(cursor, 20 + '$\\frac{1}{2}x=3$'.length);
  } else if (branch === 'selected') {
    selection.Start = 20; selection.End = 35;
    selection.Text = '1/2x=3';
    c.findAllEquationTargetsInRange = () => targets;
    c.ToggleLatex();
  } else {
    c.batchEquationToLatex(doc, selection, targets);
  }
  assert.ok(imported);
  assert.equal(mathPresent, false);
}
// Batch LaTeX toggle skips 422 unsupported formulas while still converting supported ones.
{
  const alerts = [];
  const c = controller();
  c.alert = msg => alerts.push(msg);
  let text = 'Before $\\enclose{verticalstrike}{x}$ middle $\\frac{1}{2}$ after';
  const replaced = [];
  const doc = {
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return { Start: start, End: end, Font: {}, get Text() { return text.slice(this.Start, this.End); } };
    }
  };
  const selection = { Start: 0, End: text.length, Text: text, Range: doc.Range(0, text.length), Collapse() {} };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = callback => callback();
  c.insertMathTypeAtDocRange = (d, targetRange, convData) => {
    replaced.push({ start: targetRange.Start, end: targetRange.End, text: targetRange.Text, result: convData.result });
    return {};
  };
  c.ajaxPost = (url, body, callback) => {
    const req = JSON.parse(body);
    if (req.data.includes('verticalstrike')) {
      callback(422, JSON.stringify({ status: 'error', error: 'Vertical strike is not supported in MathType' }));
    } else {
      callback(200, JSON.stringify({ result: '<pkg/>' }));
    }
  };
  c.ToggleLatex();
  assert.equal(replaced.length, 1);
  assert.equal(replaced[0].text, '$\\frac{1}{2}$');
  assert.equal(alerts.length, 1);
  assert.match(alerts[0], /Found: 2[\s\S]*Converted: 1[\s\S]*Skipped\/Errors: 1/);
}
console.log('WPS toggle tests using bug_toggle_v1 XML fixture passed');

// Taskpane in-memory upgrade must replace individual equations in descending order,
// preserve surrounding text, and strip trailing breaks/spacers added by InsertXML.
function taskpaneController(targetValue, scopeValue) {
  function makeEl(extra = {}) {
    return Object.assign({
      textContent: '',
      value: '',
      disabled: false,
      style: {},
      classList: { remove() {}, add() {} },
      listeners: {},
      addEventListener(type, fn) { this.listeners[type] = fn; }
    }, extra);
  }
  const elements = {
    'bridge-status-badge': makeEl(),
    'bridge-status-text': makeEl(),
    'upgrade-target': makeEl({ value: targetValue }),
    'btn-start-upgrade': makeEl(),
    'upgrade-progress-box': makeEl(),
    'upgrade-progress-bar': makeEl(),
    'upgrade-progress-text': makeEl(),
    'stat-found': makeEl(),
    'stat-upgraded': makeEl(),
    'stat-errors': makeEl(),
    'btn-disk-upgrade': makeEl()
  };
  const windowObj = {
    addEventListener() {},
    wps: null
  };
  const context = vm.createContext({
    console,
    DOMParser,
    XMLSerializer,
    setInterval() { return 1; },
    alert: message => { throw Error(message); },
    window: windowObj,
    document: {
      getElementById: id => elements[id] || makeEl(),
      querySelector: selector => selector.includes('upgrade-scope') ? { value: scopeValue } : null
    }
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/taskpane.js'), 'utf8'), context);
  return { context, elements, windowObj };
}

for (const scope of ['document', 'selection']) {
for (const target of ['omml', 'mathtype']) {
for (const addedBreak of ['\r', '\n', '\r\n']) {
  const { context, elements, windowObj } = taskpaneController(target, scope);
  let text = 'Alpha # beta ## omega.\rNext paragraph.\r';
  const charFonts = new Map();
  const fontFor = pos => {
    if (!charFonts.has(pos)) charFonts.set(pos, { Position: 0, Subscript: false, Superscript: false });
    return charFonts.get(pos);
  };
  const shape1 = {
    Range: { Start: 6, End: 7, Font: { Size: 12 }, WordOpenXML: '<o:OLEObject ProgID="Equation.3"/>' }
  };
  const omath2 = {
    Range: { Start: 13, End: 15, Font: { Size: 12 }, WordOpenXML: '<m:oMath><m:r><m:t>y</m:t></m:r></m:oMath>' }
  };
  const insertedXmls = [];
  const doc = {
    get Content() {
      return {
        End: text.length,
        InlineShapes: { Count: 1, Item: () => shape1 },
        OMaths: { Count: 1, Item: () => omath2 }
      };
    },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: start + 1 === end ? fontFor(start) : { Position: 0 },
        get Text() { return text.slice(this.Start, this.End); },
        get InlineShapes() {
          const hits = [];
          if (start <= 6 && end >= 7) hits.push({ Range: { Start: 6, End: 7, Font: fontFor(6) } });
          if (start <= 13 && end >= 14) hits.push({ Range: { Start: 13, End: 14, Font: fontFor(13) } });
          return { Count: hits.length, Item: idx => hits[idx - 1] };
        },
        get OMaths() {
          const hits = [];
          if (start <= 6 && end >= 8) hits.push({ Type: 0, Range: { Start: 6, End: 8, Font: { Position: -5 } } });
          if (start <= 13 && end >= 15) hits.push({ Type: 0, Range: { Start: 13, End: 15, Font: { Position: -5 } } });
          return { Count: hits.length, Item: idx => hits[idx - 1] };
        },
        get Paragraphs() {
          const pEnd = text.indexOf('\r') + 1;
          return { Item: () => ({ Range: { Start: 0, End: pEnd } }) };
        },
        Delete() {
          text = text.slice(0, this.Start) + text.slice(this.End);
        },
        InsertXML(xml) {
          insertedXmls.push({ start: this.Start, end: this.End, xml });
          const isOmml = target === 'omml';
          const replacement = (isOmml ? 'EQ ' : 'M') + addedBreak;
          text = text.slice(0, this.Start) + replacement + text.slice(this.End);
          this.End = this.Start + replacement.length;
        }
      };
    }
  };
  const selRange = {
    Start: 0,
    End: text.indexOf('\r') + 1,
    InlineShapes: { Count: 1, Item: () => shape1 },
    OMaths: { Count: 1, Item: () => omath2 }
  };
  const selection = { Start: 0, End: 0, Font: { Position: -4, Subscript: true, Superscript: true }, Range: selRange };
  windowObj.wps = { Application: { ActiveDocument: doc, Selection: selection } };

  const sentRequests = [];
  const pendingCallbacks = [];
  context.ajaxPost = (url, payload, callback) => {
    sentRequests.push(JSON.parse(payload));
    pendingCallbacks.push(callback);
  };

  elements['btn-start-upgrade'].listeners.click();
  assert.equal(sentRequests.length, 2, 'must send each equation individually, not whole paragraph');
  assert.equal(sentRequests[0].data, shape1.Range.WordOpenXML);
  assert.equal(sentRequests[1].data, omath2.Range.WordOpenXML);
  // Button must stay disabled until async conversions finish.
  assert.equal(elements['btn-start-upgrade'].disabled, true);

  const ommlXml = '<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage"><m:oMath xmlns:m="' + nsM + '"><m:r><m:t>x</m:t></m:r></m:oMath></pkg:package>';
  const mathtypeXml = '<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage" xmlns:o="urn:schemas-microsoft-com:office:office"><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  // Resolve out of order to ensure descending DOM replacement still protects offsets.
  for (const idx of [1, 0]) {
    pendingCallbacks[idx](200, JSON.stringify({
      result: target === 'omml' ? ommlXml : mathtypeXml,
      width_pt: 20,
      height_pt: 12,
      depth_pt: 4
    }));
  }

  assert.equal(elements['btn-start-upgrade'].disabled, false);
  assert.equal(elements['stat-found'].textContent, '2');
  assert.equal(elements['stat-upgraded'].textContent, '2');
  assert.equal(elements['stat-errors'].textContent, '0');
  assert.equal(insertedXmls[0].start, 13, 'must replace right-to-left');
  assert.equal(insertedXmls[1].start, 6);
  const expectedText = target === 'omml'
    ? 'Alpha EQ beta EQ omega.\rNext paragraph.\r'
    : 'Alpha M beta M omega.\rNext paragraph.\r';
  assert.equal(text, expectedText, 'no extra newline or spacer after upgraded equations');
  assert.equal(selection.Font.Position, 0);
}
}
}
console.log('WPS taskpane upgrade regression tests passed');

// 1. EditEquation & ToggleLatex must preserve full multi-character OLE InlineShape ranges (e.g. [285..311])
//    instead of clamping them down to 1 character, while trimming any trailing CR/LF and keeping adjacent U+200C.
{
  const c = controller();
  const olePlaceholder = '<' + 'EMBED Equation.DSMT4____' + '>'; // 26 chars
  assert.equal(olePlaceholder.length, 26);
  let text = 'Prefix ' + olePlaceholder + '\u200c suffix\r';
  const oleStart = 'Prefix '.length;
  const oleEnd = oleStart + olePlaceholder.length;
  let capturedSessionReq = null;

  const oleRange = {
    Start: oleStart,
    End: oleEnd,
    Font: { Size: 12 },
    WordOpenXML: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>',
    Select() {}
  };
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: { Size: 12 },
        get Text() { return text.slice(this.Start, this.End); },
        get InlineShapes() {
          const hit = (start <= oleStart && end >= oleEnd)
            ? [{ Range: oleRange, OLEFormat: { ProgID: 'Equation.DSMT4' } }]
            : [];
          return { Count: hit.length, Item: idx => hit[idx - 1] };
        },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        Select() {}
      };
    }
  };
  const selection = {
    Start: oleStart,
    End: oleEnd,
    InlineShapes: { Count: 1, Item: () => ({ Range: oleRange, OLEFormat: { ProgID: 'Equation.DSMT4' } }) },
    Range: oleRange
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = cb => cb();
  c.LaunchDesktopEditor = () => {};
  c.WaitSession = () => {};
  c.ajaxPost = (url, body, cb) => {
    capturedSessionReq = JSON.parse(body);
    cb(200, JSON.stringify({ session_id: 'sess-ole-26' }));
  };

  c.EditEquation();
  assert.equal(c.g_editRangeStart, oleStart);
  assert.equal(c.g_editRangeEnd, oleEnd + 1, 'must preserve full 26-char OLE field plus trailing U+200C');
  assert.ok(capturedSessionReq);

  // ToggleLatex on the same 26-char OLE InlineShape (both collapsed-cursor and selected)
  // must replace the entire field, leaving zero orphan chars.
  let toggleCursor = null;
  const toggleShapeRange = {
    Start: oleStart,
    End: oleEnd,
    WordOpenXML: oleRange.WordOpenXML,
    Select() {}
  };
  const docToggle = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: { Position: -5, Subscript: false, Superscript: false },
        WordOpenXML: oleRange.WordOpenXML,
        get Text() { return text.slice(this.Start, this.End); },
        set Text(v) { text = text.slice(0, this.Start) + v + text.slice(this.End); },
        get InlineShapes() {
          const hit = (start <= oleStart && end >= oleEnd) || (start === oleStart && end === oleStart + 1)
            ? [{ Range: toggleShapeRange }]
            : [];
          return { Count: hit.length, Item: idx => hit[idx - 1] };
        },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        Select() { toggleCursor = this.Start; }
      };
    }
  };
  c.wps = {
    Application: {
      ActiveDocument: docToggle,
      Selection: {
        Start: oleStart,
        End: oleStart,
        InlineShapes: { Count: 1, Item: () => ({ Range: toggleShapeRange }) },
        Range: docToggle.Range(oleStart, oleStart),
        Collapse() {},
        Font: { Position: -5, Subscript: false, Superscript: false }
      }
    }
  };
  c.ajaxPost = (url, body, cb) => cb(200, JSON.stringify({ result: '\\frac{\\pi}{3}' }));
  c.ToggleLatex();
  assert.equal(text, 'Prefix $\\frac{\\pi}{3}$ suffix\r');
  assert.equal(toggleCursor, oleStart + '$\\frac{\\pi}{3}$'.length);
}

// 2. insertMathTypeAtDocRange + HandleSessionResult handle:
//    - replacing a 26-char OLE range at document end
//    - empty range.InlineShapes falling back to doc.InlineShapes
//    - +1 start offset tolerance
for (const atDocEnd of [false, true]) {
  const c = controller();
  const oldOle = '<' + 'OLD_EMBED_EQUATION_26CHR' + '>';
  const newOle = '<' + 'NEW_EMBED_EQUATION_26CHR' + '>';
  const suffix = atDocEnd ? '' : ' tail.\r';
  let text = 'Start ' + oldOle + '\u200c' + suffix;
  const editStart = 'Start '.length;
  const editEnd = editStart + oldOle.length + 1;
  const shapeFont = {};
  const pic = { Range: { Start: editStart + 1, End: editStart + 1 + newOle.length, Font: shapeFont } };
  let insertedAt = null;
  const doc = {
    get Content() { return { End: text.length }; },
    // range.InlineShapes is intentionally empty; only doc.InlineShapes exposes the new shape at +1 offset.
    get InlineShapes() { return { Count: 1, Item: () => pic }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: (start >= pic.Range.Start && end <= pic.Range.End) ? shapeFont : { Position: 0 },
        get Text() { return text.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: text.length } }) },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML(xml) {
          insertedAt = this.Start;
          const payload = ' ' + newOle + '\r';
          text = text.slice(0, this.Start) + payload + text.slice(this.End);
        },
        Select() {}
      };
    }
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: { Collapse() {}, Font: {} } } };
  c.g_editRangeStart = editStart;
  c.g_editRangeEnd = editEnd;
  const packageXml = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  c.HandleSessionResult({ status: 'ready', flat_opc: packageXml, width_pt: 24, height_pt: 14, depth_pt: 4, font_pt: 12 });
  assert.equal(insertedAt, editStart);
  assert.equal(text, 'Start  ' + newOle + suffix);
  assert.equal(shapeFont.Position, -Number((4 + 12 * 0.045).toFixed(1)));
  assert.equal(pic.Width, 24);
  assert.equal(pic.Height, 14);
}

// 3. Taskpane insertUpgradedFlatOpcAtRange also falls back to doc.InlineShapes with +1 offset tolerance.
{
  const { context, windowObj } = taskpaneController('mathtype', 'document');
  let text = 'Alpha # omega.\r';
  const shapeFont = {};
  const newPic = { Range: { Start: 7, End: 8, Font: shapeFont } };
  const doc = {
    get Content() { return { End: text.length }; },
    get InlineShapes() { return { Count: 1, Item: () => newPic }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: (start >= newPic.Range.Start && end <= newPic.Range.End) ? shapeFont : { Position: 0 },
        get Text() { return text.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: text.length } }) },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML() {
          text = text.slice(0, this.Start) + ' M\r' + text.slice(this.End);
          this.End = this.Start + 3;
        }
      };
    }
  };
  windowObj.wps = { Application: { ActiveDocument: doc, Selection: {} } };
  const mathtypeXml = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  const ok = context.insertUpgradedFlatOpcAtRange(
    doc,
    doc.Range(6, 7),
    { result: mathtypeXml, width_pt: 18, height_pt: 11, depth_pt: 2 },
    'flat_opc_mathtype',
    12
  );
  assert.equal(ok, true);
  assert.equal(newPic.Width, 18);
  assert.equal(shapeFont.Position, -Number((2 + 12 * 0.045).toFixed(1)));
}

// 4. Separate InsertXML success with unexposed InlineShape (logs warning, no fatal alert)
//    from genuine InsertXML error (logs error and alerts with reason).
{
  const alerts = [];
  const logs = [];
  const c = vm.createContext({
    console,
    DOMParser,
    XMLSerializer,
    alert: msg => alerts.push(msg)
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), c);
  c.wpsLog = msg => logs.push(msg);

  let text = 'A B';
  const docNoShape = {
    get Content() { return { End: text.length }; },
    InlineShapes: { Count: 0, Item: () => null },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: {},
        get Text() { return text.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML() { text = text.slice(0, this.Start) + 'EQ' + text.slice(this.End); },
        Select() {}
      };
    }
  };
  c.wps = { Application: { ActiveDocument: docNoShape, Selection: { Range: docNoShape.Range(2, 2), Collapse() {}, Font: {} } } };
  c.HandleSessionResult({
    status: 'ready',
    flat_opc: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>'
  });
  assert.equal(alerts.length, 0, 'successful InsertXML without exposed InlineShape must not raise fatal alert');
  assert.ok(logs.some(m => m.includes('no matching InlineShape was exposed')));
  assert.equal(text, 'A EQB');

  // Genuine InsertXML exception must log error and alert with concrete message.
  const docThrows = {
    get Content() { return { End: 3 }; },
    Range(start, end) {
      return {
        Start: start || 0,
        End: end || 0,
        get Text() { return ''; },
        InsertXML() { throw new Error('COM InsertXML rejected package'); }
      };
    }
  };
  c.wps = { Application: { ActiveDocument: docThrows, Selection: { Range: docThrows.Range(0, 0) } } };
  c.HandleSessionResult({
    status: 'ready',
    flat_opc: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>'
  });
  assert.equal(alerts.length, 1);
  assert.ok(alerts[0].includes('COM InsertXML rejected package'));
  assert.ok(logs.some(m => m.includes('COM InsertXML rejected package')));
}

// A caret beside an object is not a selection, even if its XML includes that object.
for (const markup of ['<w:drawing/>', '<o:OLEObject ProgID="Equation.DSMT4"/>']) {
  for (const position of [4, 5]) {
    const c = controller();
    const range = { Start: 4, End: 5, WordOpenXML: markup };
    const shapes = { Count: 1, Item: () => ({ Range: range }) };
    const selection = {
      Start: position, End: position, WordOpenXML: markup,
      Range: { Start: position, End: position, WordOpenXML: markup },
      Words: { Count: 1, Item: () => ({ InlineShapes: shapes }) },
    };
    const doc = { Content: { End: 10 }, Range: () => ({ Text: ' ', InlineShapes: shapes }) };
    c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
    c.ensureBridgeRunning = () => assert.fail('an adjacent object must not trigger conversion');
    const alerts = [];
    c.alert = message => alerts.push(message);
    c.ToggleLatex();
    assert.match(alerts[0], /No equation or LaTeX code found at the cursor/);
  }
}

// 5. Batch ToggleLatex must strip extra trailing breaks after shapeEnd even when an adjacent
//    inserted OLE shape causes the 32-char suffix comparison to mismatch.
{
  const alerts = [];
  const c = controller();
  c.alert = msg => alerts.push(msg);
  let text = 'Cho $a=1$ và $b=2$ xong.\r';
  const shapes = [];
  let docEndBias = 0; // Simulates WPS field/end drift causing pre-pic suffix check to miss.
  const doc = {
    get Content() { return { End: text.length + docEndBias }; },
    get InlineShapes() { return { Count: shapes.length, Item: idx => shapes[idx - 1] }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length + docEndBias };
      return {
        Start: start,
        End: end,
        Font: { Size: 12, Position: 0 },
        get Text() { return text.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: text.length } }) },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML() {
          const ole = '<' + 'OLE_26_CHARS_PLACEHOLDER' + '>';
          assert.equal(ole.length, 26);
          docEndBias = 1; // Causes insertedEnd to overshoot suffix match on pre-pic check.
          text = text.slice(0, this.Start) + ole + '\r' + text.slice(this.End);
          shapes.push({ Range: { Start: this.Start, End: this.Start + 26, Font: {} } });
        },
        Select() {}
      };
    }
  };
  const selection = {
    Start: 0,
    End: text.length - 1,
    get Text() { return text.slice(this.Start, this.End); },
    Range: doc.Range(0, text.length - 1),
    Collapse() {},
    Font: {}
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = cb => cb();
  const pkg = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  c.ajaxPost = (url, body, cb) => cb(200, JSON.stringify({ result: pkg }));
  c.ToggleLatex();
  const ole = '<' + 'OLE_26_CHARS_PLACEHOLDER' + '>';
  assert.equal(text, 'Cho ' + ole + ' và ' + ole + ' xong.\r', 'no extra newline after 1st or 2nd formula');
}

// 6. Collapsed cursor ToggleLatex inside $...$ after a 26-char OLE object (whose .Text is only 1 char '#')
//    must resolve the exact $...$ under the cursor rather than shifting by +25 chars.
{
  const c = controller();
  const ole26 = '<' + 'OLE_26_CHARS_PLACEHOLDER' + '>';
  // Document positions have 26 chars for the OLE object, while paragraph .Text represents it as 1 char '#'.
  const docText = 'Eq ' + ole26 + ' and $\\frac{3\\pi}{4}$ plus $\\frac{5\\pi}{4}$.\r';
  const paraVisibleText = 'Eq # and $\\frac{3\\pi}{4}$ plus $\\frac{5\\pi}{4}$.\r';
  const targetStart = docText.indexOf('$\\frac{3\\pi}{4}$');
  const targetEnd = targetStart + '$\\frac{3\\pi}{4}$'.length;
  const curPos = targetStart + 5; // inside $\frac{3\pi}{4}$
  function sliceToParaText(start, end) {
    const raw = docText.slice(start, end);
    return raw.replace(ole26, '#');
  }
  let convertedLatex = null;
  let insertedRange = null;
  const doc = {
    get Content() { return { End: docText.length }; },
    Range(start, end) {
      if (start === undefined) return { End: docText.length };
      return {
        Start: start,
        End: end,
        Font: { Size: 12 },
        get Text() { return sliceToParaText(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        OMaths: { Count: 0 }
      };
    }
  };
  const selection = {
    Start: curPos,
    End: curPos,
    Paragraphs: {
      Item: () => ({
        Range: { Start: 0, End: docText.length, Text: paraVisibleText }
      })
    },
    Range: doc.Range(curPos, curPos)
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = cb => cb();
  c.insertMathTypeAtDocRange = (d, rng) => {
    insertedRange = { start: rng.Start, end: rng.End, text: docText.slice(rng.Start, rng.End) };
    return {};
  };
  c.ajaxPost = (url, body, cb) => {
    convertedLatex = JSON.parse(body).data;
    cb(200, JSON.stringify({ result: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>' }));
  };
  c.ToggleLatex();
  assert.equal(convertedLatex, '\\frac{3\\pi}{4}');
  assert.deepEqual(insertedRange, { start: targetStart, end: targetEnd, text: '$\\frac{3\\pi}{4}$' });
}
console.log('WPS MathType OLE range, fallback, and error-diagnostic tests passed');

// 7. Highlighted range with mixed formatting (Windows WPS Font.Size = 9999999 / wdUndefined)
//    must clamp font_pt to a valid size instead of sending 9999999 and getting HTTP 400.
{
  const alerts = [];
  const c = controller();
  c.alert = msg => alerts.push(msg);
  let text = 'Cho $a=1$ và $b=2$ xong.\r';
  const sentFonts = [];
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start,
        End: end,
        Font: { Size: 9999999 },
        get Text() { return text.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: text.length } }) },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML() { text = text.slice(0, this.Start) + 'M' + text.slice(this.End); },
        Select() {}
      };
    }
  };
  const selection = {
    Start: 0,
    End: text.length - 1,
    get Text() { return text.slice(this.Start, this.End); },
    Range: { Start: 0, End: text.length - 1, Font: { Size: 9999999 } },
    Font: { Size: 9999999 },
    Collapse() {}
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = cb => cb();
  c.ajaxPost = (url, body, cb) => {
    const req = JSON.parse(body);
    sentFonts.push(req.font_pt);
    assert.ok(req.font_pt >= 1 && req.font_pt <= 200, 'font_pt must never be wdUndefined 9999999');
    cb(200, JSON.stringify({ result: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>' }));
  };
  c.ToggleLatex();
  assert.deepEqual(sentFonts, [12, 12]);
  assert.equal(text, 'Cho M và M xong.\r');
}

// 8. WPS UpgradeLegacyEquations & taskpane disk upgrade:
//    - On Windows, create a temporary blank tab before closing the sole document so WPS
//      stays open (keeping the add-in/taskpane alive), close target doc to release lock,
//      run /convert/document, reopen target doc, and close temp tab.
//    - On Linux, keep the document open during /convert/document and hot-refresh after alert.
{
  for (const status of [200, 500]) {
    const events = [];
    const alerts = [];
    const c = vm.createContext({
      console,
      DOMParser,
      XMLSerializer,
      confirm: () => true,
      alert: msg => alerts.push(msg)
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), c);
    c.wpsLog = () => {};
    let openDocs = 1;
    let isTargetOpen = true;
    const tempDoc = {
      Close(saveChanges) { events.push(['tempClose', saveChanges]); openDocs--; }
    };
    const doc = {
      FullName: 'C:\\docs\\sample.docx',
      ReadOnly: false,
      Saved: true,
      Save() { events.push('save'); },
      Close(saveChanges) { events.push(['close', saveChanges]); isTargetOpen = false; openDocs--; }
    };
    const app = {
      ActiveDocument: doc,
      Documents: {
        get Count() { return openDocs; },
        Item() { return isTargetOpen ? doc : tempDoc; },
        Add() { events.push('addTemp'); openDocs++; return tempDoc; },
        Open(p) { events.push(['open', p]); isTargetOpen = true; openDocs++; return doc; }
      }
    };
    c.wps = { Application: app };
    c.ensureBridgeRunning = cb => cb();
    c.ajaxPost = (url, body, cb) => {
      events.push(['post', url, isTargetOpen, openDocs]);
      assert.equal(isTargetOpen, false, 'target doc must be closed before /convert/document on Windows');
      assert.ok(openDocs >= 1, 'WPS must retain at least 1 open document so the window/add-in stays alive');
      cb(status, status === 200 ? JSON.stringify({ found: 3, converted: 3, errors: 0 }) : '{"error":"boom"}');
    };
    c.UpgradeLegacyEquations();
    assert.deepEqual(events, [
      'save',
      'addTemp',
      ['close', false],
      ['post', 'http://127.0.0.1:17890/convert/document', false, 1],
      ['open', 'C:\\docs\\sample.docx'],
      ['tempClose', false]
    ]);
    assert.equal(isTargetOpen, true, 'document must be reopened after conversion');
    assert.equal(openDocs, 1, 'temporary blank document must be closed');
    assert.equal(alerts.length, 1);
  }

  // Linux path: keeps document open during /convert/document, alerts, then hot-refreshes.
  {
    const events = [];
    const c = vm.createContext({
      console,
      DOMParser,
      XMLSerializer,
      navigator: { platform: 'Linux x86_64' },
      confirm: () => true,
      alert: () => events.push('alert')
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), c);
    c.wpsLog = () => {};
    let isTargetOpen = true;
    const doc = {
      FullName: '/home/user/sample.docx',
      ReadOnly: false,
      Saved: true,
      Save() { events.push('save'); },
      Close(saveChanges) { events.push(['close', saveChanges]); isTargetOpen = false; }
    };
    const app = {
      ActiveDocument: doc,
      Documents: {
        get Count() { return isTargetOpen ? 1 : 0; },
        Item() { return doc; },
        Open(p) { events.push(['open', p]); isTargetOpen = true; return doc; }
      }
    };
    c.wps = { Application: app };
    c.ensureBridgeRunning = cb => cb();
    c.ajaxPost = (url, body, cb) => {
      events.push(['post', url, isTargetOpen]);
      assert.equal(isTargetOpen, true, 'Linux does not close document before /convert/document');
      cb(200, JSON.stringify({ found: 3, converted: 3, errors: 0 }));
    };
    c.UpgradeLegacyEquations();
    assert.deepEqual(events, [
      'save',
      ['post', 'http://127.0.0.1:17890/convert/document', true],
      'alert',
      ['close', false],
      ['open', '/home/user/sample.docx']
    ]);
    assert.equal(isTargetOpen, true);
  }

  for (const status of [200, 500]) {
    const events = [];
    const alerts = [];
    function makeEl(extra = {}) {
      return Object.assign({
        textContent: '', value: '', disabled: false, style: {},
        classList: { remove() {}, add() {} }, listeners: {},
        addEventListener(type, fn) { this.listeners[type] = fn; }
      }, extra);
    }
    const elements = {
      'bridge-status-badge': makeEl(),
      'bridge-status-text': makeEl(),
      'upgrade-target': makeEl({ value: 'mathtype' }),
      'btn-start-upgrade': makeEl(),
      'upgrade-progress-box': makeEl(),
      'upgrade-progress-bar': makeEl(),
      'upgrade-progress-text': makeEl(),
      'stat-found': makeEl(),
      'stat-upgraded': makeEl(),
      'stat-errors': makeEl(),
      'btn-disk-upgrade': makeEl(),
      'btn-disk-upgrade-label': makeEl()
    };
    const windowObj = { addEventListener() {}, wps: null };
    const ctx = vm.createContext({
      console, DOMParser, XMLSerializer,
      setInterval() { return 1; },
      alert: msg => alerts.push(msg),
      window: windowObj,
      document: { getElementById: id => elements[id] || makeEl(), querySelector: () => null }
    });
    vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/taskpane.js'), 'utf8'), ctx);
    let openDocs = 1;
    let isTargetOpen = true;
    const tempDoc = {
      Close(saveChanges) { events.push(['tempClose', saveChanges]); openDocs--; }
    };
    const doc = {
      FullName: 'C:\\docs\\sample.docx',
      ReadOnly: false,
      Saved: true,
      Save() { events.push('save'); },
      Close(saveChanges) { events.push(['close', saveChanges]); isTargetOpen = false; openDocs--; }
    };
    windowObj.wps = {
      Application: {
        ActiveDocument: doc,
        Documents: {
          get Count() { return openDocs; },
          Item() { return isTargetOpen ? doc : tempDoc; },
          Add() { events.push('addTemp'); openDocs++; return tempDoc; },
          Open(p) { events.push(['open', p]); isTargetOpen = true; openDocs++; return doc; }
        }
      }
    };
    ctx.ajaxPost = (url, body, cb) => {
      events.push(['post', url, isTargetOpen, openDocs]);
      assert.equal(isTargetOpen, false, 'taskpane disk upgrade must close target doc before /convert/document on Windows');
      assert.ok(openDocs >= 1, 'taskpane must retain at least 1 open document so taskpane WebView stays alive');
      cb(status, status === 200 ? JSON.stringify({ found: 2, converted: 2, errors: 0 }) : '{"error":"fail"}');
    };
    elements['btn-disk-upgrade'].listeners.click();
    assert.deepEqual(events, [
      'save',
      'addTemp',
      ['close', false],
      ['post', 'http://127.0.0.1:17890/convert/document', false, 1],
      ['open', 'C:\\docs\\sample.docx'],
      ['tempClose', false]
    ]);
    assert.equal(isTargetOpen, true);
    assert.equal(openDocs, 1);
    assert.equal(elements['btn-disk-upgrade'].disabled, false);
    assert.equal(alerts.length, 1);
  }
}
console.log('WPS batch font_pt clamping and Windows disk-upgrade lock-release tests passed');

// 9. Large multi-page ToggleLatex batch must bound concurrent HTTP requests (<= 8, avoiding bridge 503),
//    yield DOM replacements across chunks via setTimeout, and use cumShift so OLE offsets stay O(1).
for (const platform of ['Linux x86_64', 'Win32']) {
  const timers = [];
  const alerts = [];
  const c = vm.createContext({
    console,
    DOMParser,
    XMLSerializer,
    navigator: { platform },
    setTimeout(fn) { timers.push(fn); return timers.length; },
    alert: msg => alerts.push(msg)
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/main.js'), 'utf8'), c);
  c.wpsLog = () => {};

  const ole26 = '<' + 'OLE_26_CHARS_PLACEHOLDER' + '>';
  const formulaCount = 35;
  let docText = 'Start ' + ole26 + ' ';
  let visibleText = 'Start # ';
  for (let i = 0; i < formulaCount; i++) {
    const f = `$x_${i}$`;
    docText += f + ' ';
    visibleText += f + ' ';
  }

  let rangeCalls = 0;
  const imported = [];
  let now = 0;
  c.batchNow = () => { now += 30; return now; };
  function visibleSlice(s, e) {
    return docText.slice(s, e).replace(ole26, '#');
  }
  const doc = {
    get Content() { return { End: docText.length }; },
    Range(start, end) {
      if (start === undefined) return { End: docText.length };
      return {
        Start: start,
        End: end,
        Font: { Size: 12 },
        get Text() { rangeCalls++; return visibleSlice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: docText.length } }) },
        Delete() { assert.fail('verified LaTeX should be replaced directly'); },
        InsertXML() {
          imported.push(this.Text);
          docText = docText.slice(0, this.Start) + 'M' + docText.slice(this.End);
        },
        Select() {}
      };
    }
  };
  let collapses = 0;
  const selection = {
    Start: 0,
    End: docText.length,
    get Text() { return visibleText; },
    Range: { Start: 0, End: docText.length, Font: { Size: 12 } },
    Collapse() { collapses++; }
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  const { pane, messages } = mockToggleProgress(c);
  c.ensureBridgeRunning = cb => cb();

  const pending = [];
  let maxInFlight = 0;
  let totalSent = 0;
  c.ajaxPost = (url, body, cb) => {
    totalSent++;
    pending.push(cb);
    if (pending.length > maxInFlight) maxInFlight = pending.length;
  };

  c.ToggleLatex();
  assert.equal(pane.DockPosition, 3);
  assert.equal(pane.Visible, true);
  c.ToggleLatex();
  assert.ok(rangeCalls < 200, `cumShift + chunked scan must avoid O(N^2) COM calls (got ${rangeCalls})`);
  assert.equal(maxInFlight, 8, 'must cap concurrent HTTP requests to 8');
  const response = JSON.stringify({ result: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>' });
  const first = pending.shift();
  pending.pop()(200, response);
  timers.shift()();
  assert.equal(imported.length, 0, 'later ready results must wait for the next target');
  assert.equal(totalSent, 8, 'completed results still occupy a slot');
  first(200, response);
  assert.equal(imported.length, 0, 'HTTP callback must yield before document writes');
  timers.shift()();
  assert.deepEqual(imported, ['$x_34$'], 'first replacement happens while requests are pending');
  assert.equal(totalSent, 9, 'retiring the first item refills one slot');
  c.ToggleLatex();
  assert.equal(totalSent, 9, 'a second toggle cannot start while the first batch is active');
  while (pending.length || timers.length) {
    while (pending.length) pending.shift()(200, response);
    if (timers.length) timers.shift()();
  }
  assert.equal(totalSent, formulaCount);
  assert.ok(!docText.includes('$x_'), 'all 35 formulas must be converted');
  assert.deepEqual(imported, Array.from({ length: formulaCount }, (_, i) => `$x_${formulaCount - 1 - i}$`));
  assert.equal(collapses, 0, 'LaTeX batch must leave the selection alone');
  assert.equal(c.g_latexBatchActive, false);
  assert.equal(pane.Visible, false);
  assert.equal(messages[0], 'Inserting (0 / 35)…');
  assert.ok(messages.some(message => /^Inserting \([1-9]/.test(message)));
  assert.equal(messages.at(-1), null);
  assert.equal(alerts.length, 1);
  assert.match(alerts[0], /Found: 35[\s\S]*Converted: 35[\s\S]*Skipped\/Errors: 0/);
}
console.log('WPS large-batch bounded concurrency, chunked DOM, and cumShift tests passed');

// A WPS OLE object can export its embedding through its full shape range only.
{
  const c = controller();
  const ole = '<o:OLEObject ProgID="Equation.DSMT4"/>';
  const shape = { AlternativeText: 'EqnTide equation', Range: { Start: 4, End: 28, WordOpenXML: ole } };
  const selection = { Range: { InlineShapes: { Count: 1, Item: () => shape } } };
  const doc = { Range: () => ({ WordOpenXML: '', Text: '' }) };
  const replacements = [];
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  const targets = c.findAllEquationTargetsInRange(selection.Range, doc);
  assert.equal(targets[0].end, 5, 'replacement remains limited to the object marker');
  c.ajaxPost = (_url, payload, done) => {
    assert.equal(JSON.parse(payload).data, ole);
    done(200, '{"result":"x^2"}');
  };
  c.replaceEquationWithLatex = (_doc, start, end, text) => replacements.push([start, end, text]);
  c.batchEquationToLatex(doc, selection, targets);
  assert.deepEqual(replacements, [[4, 5, '$x^2$']]);
}

// Re-export can rename parts and wrap base64 without changing an OLE equation.
{
  const c = controller();
  const packageXml = (name, data) => '<pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage">' +
    '<pkg:part pkg:name="/word/embeddings/' + name + '.bin"><pkg:binaryData>' + data +
    '</pkg:binaryData></pkg:part></pkg:package>';
  const before = packageXml('oleObject1', 'ABCDEF==');
  assert.equal(c.toggleSourceSignature(before, false), c.toggleSourceSignature(packageXml('oleObject9', 'ABC\r\nDEF=='), false));
  assert.notEqual(c.toggleSourceSignature(before, false), c.toggleSourceSignature(packageXml('oleObject9', 'ABCDZZ=='), false));
}

// Replacing an OMML paragraph can invalidate COM wrappers for other equations in it.
{
  const c = controller();
  const oldObjects = [];
  const responses = [];
  const written = [];
  const alerts = [];
  let generation = 0;
  const nativeXml = (start, revision) =>
    '<w:p><m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><m:r w:rsidR="' + revision +
    '"><m:t>' + start + '</m:t></m:r></m:oMath></w:p>';
  const liveObject = start => ({
    get Range() { return { Start: start, End: start + 2, WordOpenXML: nativeXml(start, generation) }; }
  });
  for (const start of [2, 6]) {
    oldObjects.push({ get Range() {
      if (generation) throw Error('stale WPS COM wrapper');
      return liveObject(start).Range;
    } });
  }
  const doc = { Range(start, end) {
    return { Start: start, End: end, Text: '', OMaths: { Count: 1, Item: () => liveObject(start) } };
  } };
  const selection = { Range: { OMaths: { Count: 2, Item: index => oldObjects[index - 1] } } };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ajaxPost = (_url, _payload, done) => { responses.push(done); };
  c.replaceEquationWithLatex = (_doc, start) => { written.push(start); generation++; };
  c.alert = message => alerts.push(message);
  c.batchEquationToLatex(doc, selection, c.findAllEquationTargetsInRange(selection.Range, doc));
  responses[1](200, '{"result":"x"}');
  responses[0](200, '{"result":"y"}');
  assert.deepEqual(written, [6, 2]);
  assert.match(alerts[0], /Found: 2[\s\S]*Converted: 2/);
}

// Equation-to-LaTeX extracts XML as the window advances and writes before the final request.
{
  const c = controller();
  const pending = [];
  const written = [];
  const reads = [];
  const targets = Array.from({ length: 12 }, (_, start) => ({ start, end: start + 1, isOmml: false }));
  const doc = { Range(start) {
    return {
      get WordOpenXML() { reads.push(start); return '<m:oMath>' + start + '</m:oMath>'; },
      Text: '',
    };
  } };
  let collapses = 0;
  const selection = { Start: 0, End: 12, Collapse() { collapses++; } };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.alert = () => {};
  c.replaceEquationWithLatex = (_doc, start) => written.push(start);
  c.ajaxPost = (_url, _payload, done) => { pending.push(done); };
  c.batchEquationToLatex(doc, selection, targets);
  assert.deepEqual(reads, [11, 10, 9, 8, 7, 6, 5, 4]);
  const first = pending.shift();
  const response = '{"result":"x"}';
  pending.shift()(200, response);
  assert.deepEqual(written, []);
  assert.equal(reads.length, 8);
  first(200, response);
  assert.deepEqual(written, [11, 10]);
  assert.deepEqual(reads.slice(8), [11, 3, 10, 2]);
  while (pending.length) pending.shift()(200, response);
  assert.deepEqual(written, Array.from({ length: 12 }, (_, index) => 11 - index));
  assert.equal(collapses, 0, 'equation batch must leave the selection alone');
  assert.ok(targets.every(target => !('xml' in target) && !('latex' in target)));
}

// A same-length user edit stops the active window and cancels outstanding requests.
{
  const c = controller();
  const callbacks = [];
  const alerts = [];
  let aborted = 0;
  let text = '$x$ $y$ $z$';
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      return { get Text() { return text.slice(start, end); }, Font: { Size: 12 },
        InsertXML() { assert.fail('an invalidated target must never be replaced'); } };
    }
  };
  const selection = { Start: 0, End: text.length, Text: text, Range: doc.Range(0, text.length) };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  const { pane } = mockToggleProgress(c);
  c.ensureBridgeRunning = done => done();
  c.ajaxPost = (_url, _payload, done) => {
    callbacks.push(done);
    return { abort() { aborted++; done(0, ''); } };
  };
  c.alert = message => alerts.push(message);
  c.ToggleLatex();
  text = text.replace('$z$', '$w$');
  c.onDocumentChange();
  callbacks[0](200, '{"result":"unused"}');
  assert.equal(aborted, 2);
  assert.equal(pane.Visible, false);
  assert.equal(c.g_latexBatchActive, false);
  assert.equal(text, '$x$ $y$ $w$');
  assert.match(alerts[0], /document changed.*Converted: 0/s);
}

// A partial Linux selection must import at a caret and preserve neighboring OLE markers.
for (const hostBehavior of ['insert', 'no-op import', 'no-op delete']) {
  const c = controller();
  c.navigator = { platform: 'Linux x86_64' };
  const original = 'OLE# $x$ and $y$ #OLE';
  let text = original;
  const alerts = [];
  const imports = [];
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      return {
        Start: start, End: end, Font: { Size: 12 },
        get Text() { return text.slice(start, end); },
        set Text(value) { text = text.slice(0, start) + value + text.slice(end); },
        Delete() {
          if (hostBehavior !== 'no-op delete') text = text.slice(0, start) + text.slice(end);
        },
        InsertXML() {
          imports.push([start, end]);
          if (hostBehavior === 'insert') text = text.slice(0, start) + 'M' + text.slice(end);
        }
      };
    }
  };
  const start = original.indexOf('$x$');
  const end = original.indexOf('$y$') + 3;
  const selection = { Start: start, End: end, Text: original.slice(start, end), Range: doc.Range(start, end) };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.alert = message => alerts.push(message);
  c.ensureBridgeRunning = ready => ready();
  c.ajaxPost = (_url, _body, done) => done(200, JSON.stringify({ result: '<o:OLEObject ProgID="Equation.DSMT4"/>' }));
  c.ToggleLatex();
  if (hostBehavior === 'insert') {
    assert.equal(text, 'OLE# M and M #OLE');
    assert.match(alerts[0], /Converted: 2/);
  } else {
    assert.equal(text, original);
    assert.match(alerts[0], /Converted: 0/);
  }
  assert.equal(imports.length, hostBehavior === 'no-op delete' ? 0 : 2);
  assert.ok(imports.every(([from, to]) => from === to));
}

// A failed direct import can be skipped only while the original range is intact or restored.
for (const failure of ['unchanged', 'deleted', 'unrestorable', 'noop']) {
  const c = controller();
  const alerts = [];
  const pending = [];
  const imports = [];
  const source = '$a$ $b$ $c$ $d$';
  let text = source;
  c.navigator = { platform: 'Linux x86_64' };
  c.alert = msg => alerts.push(msg);
  const doc = {
    get Content() { return { End: text.length }; },
    Range(start, end) {
      return {
        Start: start, End: end,
        get Text() { return text.slice(start, end); },
        set Text(value) {
          assert.equal(failure, 'deleted');
          assert.equal(start, end);
          text = text.slice(0, start) + value + text.slice(end);
        },
        InsertXML() {
          const target = this.Text;
          imports.push(target);
          if (target === '$c$') {
            if (failure === 'noop') return;
            if (failure !== 'unchanged') {
              text = text.slice(0, start) + (failure === 'unrestorable' ? 'X' : '') + text.slice(end);
            }
            throw Error('import failed');
          }
          text = text.slice(0, start) + '#' + text.slice(end);
        }
      };
    }
  };
  const selection = {
    Start: 0, End: source.length, Text: source,
    Range: { Font: { Size: 12 } }
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  const { pane, messages } = mockToggleProgress(c);
  c.ensureBridgeRunning = cb => cb();
  c.ajaxPost = (_url, body, cb) => pending.push([JSON.parse(body).data, cb]);

  c.ToggleLatex();
  for (const [latex, cb] of pending) {
    cb(latex === 'a' || latex === 'd' ? 422 : 200, JSON.stringify({ result: '<o:OLEObject ProgID="Equation.DSMT4"/>' }));
  }
  const stopped = failure === 'unrestorable';
  assert.deepEqual(imports, stopped ? ['$c$'] : ['$c$', '$b$']);
  assert.equal(text, stopped ? '$a$ $b$ X $d$' : '$a$ # $c$ $d$');
  assert.match(alerts[0], stopped ? /WPS changed the document.*Converted: 0/s : /Converted: 1/);
  assert.equal(c.g_latexBatchActive, false);
  assert.equal(pane.Visible, false);
  assert.equal(messages.at(-1), null);
}

{
  const c = controller();
  const shape = { Range: { Start: 20, End: 46 } };
  const local = { InlineShapes: { Count: 1, Item: () => shape } };
  let docLookups = 0;
  const doc = {
    get InlineShapes() { docLookups++; },
    get Content() { docLookups++; }
  };
  const located = c.findInsertedInlineShape(doc, local, 20);
  assert.equal(located.shape, shape);
  assert.equal(located.range, shape.Range);
  assert.equal(docLookups, 0, 'matching local shape must avoid document lookup');
}

{
  const c = controller();
  const equation = '<w:object w:dxaOrig="224" w:dyaOrig="232">' +
    '<v:shape id="shape1" type="#_x0000_t75" style="width:11.2pt;height:11.6pt" o:ole="">' +
    '<v:imagedata r:id="image1"/></v:shape>' +
    '<o:OLEObject ProgID="Equation.DSMT4" r:id="ole1"/></w:object>';
  const xml = '<pkg:package><w:r><w:rPr><w:position w:val="-6"/></w:rPr>' + equation +
    '</w:r><pkg:binaryData>AQID</pkg:binaryData></pkg:package>';
  const metadata = { width_pt: 11.27, height_pt: 11.54, depth_pt: 2.89 };
  const result = c.formatMathTypePackageForWps(xml, metadata, 12);
  assert.match(result, /<w:position w:val="-6"\/>/);
  assert.match(result, /w:dxaOrig="225" w:dyaOrig="231"/);
  assert.match(result, /style="width:11\.25pt;height:11\.55pt" alt="EqnTide equation"/);
  assert.match(result, /r:id="image1"[\s\S]*r:id="ole1"[\s\S]*<pkg:binaryData>AQID<\/pkg:binaryData>/);
  assert.equal(c.formatMathTypePackageForWps(equation, metadata, 12), null);
  assert.equal(c.formatMathTypePackageForWps(xml, { ...metadata, width_pt: 0 }, 12), null);
  assert.equal(c.formatMathTypePackageForWps(xml, metadata, 9999999), null);
}

// 10. Ctrl+A across a multi-page exam containing prose with '=', '-', and equations must convert
//     the equations to LaTeX (NOT treat the entire document as one bare LaTeX formula via isLatexText),
//     and stray cross-blank-line $$ must not swallow pages.
{
  const c = controller();
  const examText = 'ĐỀ 1\rĐỀ KIỂM TRA GIỮA HỌC KÌ I\rThời gian làm bài: 90 phút - không tính.\rCâu 1: Cho u_1 = 3 và u_2 = 5 - 1.\rCâu 2: Chọn đáp án đúng.';
  const selection = {
    Start: 0,
    End: examText.length,
    Text: examText,
    Range: { Start: 0, End: examText.length, Font: { Size: 12 } }
  };
  const doc = {};
  const fakeEq = [{ start: 80, end: 85, xml: '<m:oMath/>', isDisplay: false }];
  let latexCalled = false;
  let eqCalled = false;
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = cb => cb();
  c.findAllEquationTargetsInRange = () => fakeEq;
  c.batchLatexToMathType = () => { latexCalled = true; };
  c.batchEquationToLatex = (_d, _s, targets) => {
    eqCalled = true;
    assert.equal(targets.length, 1);
  };
  c.ToggleLatex();
  assert.equal(latexCalled, false, 'multi-page prose with = and - must not trigger bare-LaTeX batch');
  assert.equal(eqCalled, true, 'must fall through to converting equations in selection');

  // Even without equations, multi-paragraph prose with '=' and '-' must not become one equation,
  // and stray $$ across blank lines must not swallow multiple paragraphs.
  assert.equal(c.isLatexText(examText), false);
  assert.equal(c.isLatexText('Câu 10: Cho cấp số cộng với u_1 = 3 và u_2 = 5 - 1'), false);
  assert.equal(c.isLatexText('x^2 + 1 = 0'), true);
  assert.equal(c.findAllLatexMatches('$$start\r\n\r\nmiddle\r\n\r\nend$$').length, 0);
}
console.log('WPS Ctrl+A full-document prose vs equation priority tests passed');

// 11. compactFlatOpcXml strips bloated styles/numbering parts. Ribbon Toggle TeX reads
//     source XML during dispatch and verification. Taskpane btn-start-upgrade
//     bounds concurrent HTTP requests (<= 8) and chunks DOM updates via setTimeout.
{
  const c = controller();
  const bloatedStyles = 'S'.repeat(20000);
  const bloatedNumbering = 'N'.repeat(10000);
  const fullPkg =
    '<?xml version="1.0"?><pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage">' +
    '<pkg:part pkg:name="/word/document.xml" pkg:contentType="text/xml"><pkg:xmlData><w:document/></pkg:xmlData></pkg:part>' +
    '<pkg:part pkg:name="/word/styles.xml" pkg:contentType="text/xml"><pkg:xmlData>' + bloatedStyles + '</pkg:xmlData></pkg:part>' +
    '<pkg:part pkg:name="/word/numbering.xml" pkg:contentType="text/xml"><pkg:xmlData>' + bloatedNumbering + '</pkg:xmlData></pkg:part>' +
    '<pkg:part pkg:name="/word/embeddings/oleObject1.bin" pkg:contentType="application/vnd.openxmlformats-officedocument.oleObject"><pkg:binaryData>QUJD</pkg:binaryData></pkg:part>' +
    '</pkg:package>';
  const compacted = c.compactFlatOpcXml(fullPkg);
  assert.ok(compacted.length < 1000, 'compacted Flat OPC must strip styles.xml and numbering.xml');
  assert.ok(compacted.includes('/word/document.xml'));
  assert.ok(compacted.includes('/word/embeddings/oleObject1.bin'));
  assert.ok(!compacted.includes('/word/styles.xml'));
  assert.ok(!compacted.includes('/word/numbering.xml'));

  let docText = 'A # # B';
  let extraWordOpenXmlReads = 0;
  const doc = {
    get Content() { return { End: docText.length }; },
    Range(start, end) {
      if (start === undefined) return { End: docText.length };
      return {
        Start: start, End: end, Font: { Position: 0 },
        get Text() { return docText.slice(this.Start, this.End); },
        set Text(v) { docText = docText.slice(0, this.Start) + v + docText.slice(this.End); },
        get WordOpenXML() {
          extraWordOpenXmlReads++;
          return this.Text === '#' ? '<o:OLEObject ProgID="Equation.DSMT4"/>' : '';
        },
        InlineShapes: { Count: 0, Item: () => null },
        OMaths: { Count: 0, Item: () => null }
      };
    }
  };
  const targets = [2, 4].map(start => ({
    start, end: start + 1, isDisplay: false, isOmml: false
  }));
  c.wps = { Application: { ActiveDocument: doc, Selection: { Collapse() {}, Font: {} } } };
  const { pane, messages } = mockToggleProgress(c);
  const pending = [];
  let alerts = [];
  c.alert = message => alerts.push(message);
  c.ajaxPost = (_u, _b, cb) => pending.push(cb);
  c.batchEquationToLatex(doc, {}, targets);
  assert.equal(pane.Visible, true);
  pending[1](200, JSON.stringify({ result: 'x^2' }));
  assert.equal(docText, 'A # # B', 'out-of-order results wait for their turn');
  pending[0](422, '');
  assert.equal(extraWordOpenXmlReads, 3, 'read XML at dispatch and verify only the successful target');
  assert.ok(targets.every(item => !('xml' in item) && !('latex' in item)), 'target metadata retains no payload or result');
  assert.equal(docText, 'A $x^2$ # B');
  assert.equal(pane.Visible, false);
  assert.deepEqual(messages, ['Replacing with LaTeX (0 / 2)…', null]);
  assert.match(alerts[0], /Found: 2[\s\S]*Converted: 1[\s\S]*Skipped\/Errors: 1/);
}
{
  const timers = [];
  function makeEl(extra = {}) {
    return Object.assign({
      textContent: '', value: '', disabled: false, style: {},
      classList: { remove() {}, add() {} }, listeners: {},
      addEventListener(type, fn) { this.listeners[type] = fn; }
    }, extra);
  }
  const elements = {
    'bridge-status-badge': makeEl(),
    'bridge-status-text': makeEl(),
    'upgrade-target': makeEl({ value: 'mathtype' }),
    'btn-start-upgrade': makeEl(),
    'upgrade-progress-box': makeEl(),
    'upgrade-progress-bar': makeEl(),
    'upgrade-progress-text': makeEl(),
    'stat-found': makeEl(),
    'stat-upgraded': makeEl(),
    'stat-errors': makeEl(),
    'btn-disk-upgrade': makeEl()
  };
  const windowObj = { addEventListener() {}, wps: null };
  const ctx = vm.createContext({
    console, DOMParser, XMLSerializer,
    setTimeout(fn) { timers.push(fn); return timers.length; },
    setInterval() { return 1; },
    alert: msg => { throw Error(msg); },
    window: windowObj,
    document: { getElementById: id => elements[id] || makeEl(), querySelector: () => ({ value: 'document' }) }
  });
  vm.runInContext(fs.readFileSync(path.join(__dirname, '../../wps-addin/taskpane.js'), 'utf8'), ctx);

  const count = 30;
  let docText = '#'.repeat(count);
  const shapesArr = Array.from({ length: count }, (_, idx) => ({
    Type: 1,
    OLEFormat: { ProgID: 'Equation.3' },
    Range: { Start: idx, End: idx + 1, Font: { Size: 12 }, WordOpenXML: '<o:OLEObject ProgID="Equation.3"/>' }
  }));
  let shapeLookups = 0;
  const doc = {
    get Content() {
      return {
        End: docText.length,
        InlineShapes: { Count: count, Item: i => shapesArr[i - 1] },
        OMaths: { Count: 0, Item: () => null }
      };
    },
    InlineShapes: {
      get Count() { return count; },
      Item(i) { shapeLookups++; return shapesArr[i - 1]; }
    },
    Range(start, end) {
      if (start === undefined) return { End: docText.length };
      return {
        Start: start, End: end, Font: { Position: 0 },
        get Text() { return docText.slice(this.Start, this.End); },
        InlineShapes: { Count: 0, Item: () => null },
        Paragraphs: { Item: () => ({ Range: { Start: 0, End: docText.length } }) },
        Delete() { docText = docText.slice(0, this.Start) + docText.slice(this.End); },
        InsertXML() { docText = docText.slice(0, this.Start) + 'M' + docText.slice(this.End); }
      };
    }
  };
  windowObj.wps = { Application: { ActiveDocument: doc, Selection: { Start: 0, End: 0, Font: {} } } };
  const pending = [];
  let maxInFlight = 0;
  ctx.ajaxPost = (_u, _b, cb) => {
    pending.push(cb);
    if (pending.length > maxInFlight) maxInFlight = pending.length;
  };
  elements['btn-start-upgrade'].listeners.click();
  assert.equal(maxInFlight, 8, 'taskpane upgrade must cap concurrent HTTP requests to 8');
  while (pending.length > 0) {
    pending.shift()(200, JSON.stringify({ result: '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>' }));
  }
  assert.ok(timers.length >= 1, 'taskpane DOM replacements must yield between chunks');
  while (timers.length > 0) timers.shift()();
  assert.equal(elements['stat-upgraded'].textContent, String(count));
  assert.ok(shapeLookups < count * 10, `binary search must avoid O(N^2) InlineShapes scans (got ${shapeLookups})`);
}
console.log('WPS Flat OPC compaction & taskpane bounded-concurrency tests passed');

{
  const c = controller();
  c.atob = value => Buffer.from(value, 'base64').toString('latin1');
  const nsPkg = 'http://schemas.microsoft.com/office/2006/xmlPackage';
  const nsRel = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships';
  const nsPackageRel = 'http://schemas.openxmlformats.org/package/2006/relationships';
  const wmf = Buffer.alloc(48);
  wmf.writeUInt16LE(9, 2);
  wmf.writeUInt32LE(15, 18);
  wmf.writeUInt16LE(0x0626, 22);
  wmf.writeUInt16LE(15, 24);
  wmf.writeUInt16LE(19, 26);
  wmf.write('AppsMFCC', 28);
  wmf[46] = 3;
  const emf = Buffer.alloc(120);
  emf.writeUInt32LE(1, 0);
  emf.writeUInt32LE(88, 4);
  emf.write(' EMF', 40);
  emf.writeUInt32LE(70, 88);
  emf.writeUInt32LE(32, 92);
  emf.writeUInt32LE(19, 96);
  emf.write('AppsMFCC', 100);
  emf[118] = 5;
  const drawing = '<w:drawing><a:blip r:embed="selected"/></w:drawing>';
  const pict = '<w:pict><v:imagedata r:id="selected"/></w:pict>';
  const ole = '<w:object><o:OLEObject ProgID="Equation.DSMT4"/></w:object>';
  const math = '<m:oMath/>';
  const part = (name, value) => `<pkg:part pkg:name="/${name}">${value}</pkg:part>`;
  const binary = bytes => `<pkg:binaryData>${bytes.toString('base64')}</pkg:binaryData>`;
  function flatOpc(content, image = Buffer.from('ordinary image')) {
    return `<pkg:package xmlns:pkg="${nsPkg}">
      ${part('word/document.xml', `<pkg:xmlData><w:document xmlns:w="${nsW}" xmlns:m="${nsM}"
        xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
        xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office"
        xmlns:r="${nsRel}"><w:body>${content}</w:body></w:document></pkg:xmlData>`)}
      ${part('word/_rels/document.xml.rels', `<pkg:xmlData><Relationships xmlns="${nsPackageRel}">
        <Relationship Id="selected" Type="${nsRel}/image" Target="media/selected"/>
        <Relationship Id="unreferenced" Type="${nsRel}/image" Target="media/unreferenced"/>
      </Relationships></pkg:xmlData>`)}
      ${part('word/media/selected', binary(image))}
      ${part('word/media/unreferenced', binary(emf))}
    </pkg:package>`;
  }
  const shapes = [
    flatOpc(drawing), flatOpc(pict), flatOpc(ole),
    flatOpc(pict, wmf), flatOpc(drawing, emf),
  ].map((WordOpenXML, Start) => ({ Type: 3, Range: { WordOpenXML, Start, End: Start + 1 } }));
  const selection = {
    Start: 0, End: 7, Text: 'Mixed selection with illustrations and equations.',
    Range: {
      InlineShapes: { Count: shapes.length, Item: i => shapes[i - 1] },
      OMaths: { Count: 1, Item: () => ({ Range: { Start: 6, End: 7, WordOpenXML: flatOpc(math) } }) },
    },
  };
  assert.equal(c.hasBatchEquationInXml(flatOpc(math)), true);
  assert.equal(c.hasBatchEquationInXml(flatOpc(drawing)), false, 'ignore an unrelated equation preview');
  const targets = c.findAllEquationTargetsInRange(selection.Range, {});
  assert.deepEqual(Array.from(targets, target => target.start), [2, 3, 4, 6]);
  assert.equal(targets[3].isOmml, true);

  const doc = { Range: start => ({
    Text: '',
    WordOpenXML: start === 6 ? flatOpc(math) : shapes[start]?.Range.WordOpenXML,
  }) };
  const alerts = [];
  const requested = [];
  const replaced = [];
  c.wps = { Application: { ActiveDocument: doc, Selection: selection } };
  c.ensureBridgeRunning = callback => callback();
  c.alert = message => alerts.push(message);
  c.ajaxPost = (_url, body, done) => {
    const xml = JSON.parse(body).data;
    requested.push(xml);
    if (xml.includes('ProgID="Equation.DSMT4"')) done(422, '{}');
    else done(200, '{"result":"x"}');
  };
  c.replaceEquationWithLatex = (_doc, start) => replaced.push(start);
  c.ToggleLatex();
  assert.equal(requested.length, 4);
  assert.deepEqual(replaced, [6, 4, 3]);
  assert.match(alerts[0], /Found: 4\nConverted: 3\nSkipped\/Errors: 1/);

  c.batchEquationToLatex(doc, selection, [targets[0]]);
  assert.match(alerts[1], /Found: 1\nConverted: 0\nSkipped\/Errors: 1/);
}

// 12. Post-insert refresh toggles View.ShowFieldCodes under ScreenUpdating=false (with
//     doc.Fields.Update() fallback) once when existing equations (Count > 1) are present,
//     preserving selection/scroll without repaginating or mutating field values.
{
  const c = controller();
  const timers = [];
  const calls = [];
  let fieldCount = 1;
  let showFieldCodes = false;
  let screenUpdating = true;
  let vp = 42;
  let sel = { Start: 12, End: 12 };
  const doc = {
    Fields: {
      get Count() { return fieldCount; },
      Update() { calls.push('fields.update'); }
    },
    Range(s, e) {
      return { Select() { sel.Start = s; sel.End = e; calls.push(['select', s, e]); } };
    },
    Repaginate() { calls.push('repaginate'); }
  };
  const view = {
    get ShowFieldCodes() { return showFieldCodes; },
    set ShowFieldCodes(v) {
      showFieldCodes = v;
      calls.push(['fieldCodes', v, screenUpdating]);
      if (v) { sel.Start = 3; sel.End = 3; vp = 1; }
    }
  };
  c.wps = {
    Application: {
      ActiveDocument: doc,
      get Selection() { return sel; },
      get ScreenUpdating() { return screenUpdating; },
      set ScreenUpdating(v) { screenUpdating = v; calls.push(['screenUpdating', v]); },
      ActiveWindow: {
        View: view,
        ActivePane: {
          get VerticalPercentScrolled() { return vp; },
          set VerticalPercentScrolled(v) { vp = v; calls.push(['scroll', v]); },
          View: { Zoom: { set Percentage(_v) { calls.push('zoom'); } } }
        }
      },
      ScreenRefresh() { calls.push('refresh'); }
    }
  };
  c.setTimeout = fn => timers.push(fn);

  // Single-equation doc (Count <= 1): skip ShowFieldCodes toggle & Fields.Update().
  c.refreshToggleScreen(doc, true);
  timers.shift()();
  assert.deepEqual(calls, ['refresh']);

  // Multi-equation doc (Count > 1) without updateFields flag (e.g. Equation -> LaTeX): skip.
  calls.length = 0;
  fieldCount = 5;
  c.refreshToggleScreen(doc);
  timers.shift()();
  assert.deepEqual(calls, ['refresh']);

  // Multi-equation doc (Count > 1) with updateFields=true: toggle ShowFieldCodes under ScreenUpdating=false,
  // restore shifted selection/scroll, avoid calling slow doc.Fields.Update(), then ScreenRefresh().
  calls.length = 0;
  c.refreshToggleScreen(doc, true);
  timers.shift()();
  assert.deepEqual(calls, [
    ['screenUpdating', false],
    ['fieldCodes', true, false],
    ['fieldCodes', false, false],
    ['select', 12, 12],
    ['scroll', 42],
    ['screenUpdating', true],
    'refresh'
  ]);
  assert.equal(showFieldCodes, false);
  assert.equal(screenUpdating, true);
  assert.equal(vp, 42);
  assert.deepEqual(sel, { Start: 12, End: 12 });

  // Fallback when View.ShowFieldCodes is unavailable: calls doc.Fields.Update().
  calls.length = 0;
  c.wps.Application.ActiveWindow.View = {};
  c.refreshToggleScreen(doc, true);
  timers.shift()();
  assert.deepEqual(calls, ['fields.update', 'refresh']);

  // Active document switched before deferred redraw: skip both refresh work and ScreenRefresh().
  calls.length = 0;
  c.refreshToggleScreen(doc, true);
  c.wps.Application.ActiveDocument = {};
  timers.shift()();
  assert.deepEqual(calls, []);
}
{
  // InsertMathTypeEquation & single ToggleLatex (LaTeX -> MathType) trigger deferred Fields.Update()
  // when Count > 1; batchLatexToMathType triggers it once after all insertions.
  const c = controller();
  const timers = [];
  const calls = [];
  let text = 'Before $a=1$ middle $b=2$ after';
  const pic = { Range: { Start: 0, End: 1, Font: { Position: 0 } } };
  const doc = {
    get Content() { return { End: text.length }; },
    Fields: { Count: 10, Update() { calls.push('fields.update'); } },
    Range(start, end) {
      if (start === undefined) return { End: text.length };
      return {
        Start: start, End: end, Font: { Size: 12, Position: 0 },
        get Text() { return text.slice(this.Start, this.End); },
        Delete() { text = text.slice(0, this.Start) + text.slice(this.End); },
        InsertXML() { text = text.slice(0, this.Start) + '#\r' + text.slice(this.End); },
        InlineShapes: { Count: 1, Item: () => pic },
        Paragraphs: { Item: () => null },
        Select() {}
      };
    }
  };
  const selection = { Start: 0, End: 0, Font: { Size: 12 }, Range: doc.Range(0, 0) };
  c.wps = {
    Application: {
      ActiveDocument: doc,
      Selection: selection,
      ScreenRefresh() { calls.push('refresh'); }
    }
  };
  c.setTimeout = fn => timers.push(fn);
  c.ensureBridgeRunning = cb => cb();
  c.alert = () => {};
  const pkg = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  c.ajaxPost = (_u, _b, cb) => cb(200, JSON.stringify({ result: pkg, width_pt: 10, height_pt: 10, depth_pt: 0 }));

  // 1. InsertMathTypeEquation
  assert.equal(c.InsertMathTypeEquation({ flat_opc: pkg, width_pt: 10, height_pt: 10, depth_pt: 0 }), true);
  assert.deepEqual(calls, []);
  while (timers.length) timers.shift()();
  assert.deepEqual(calls, ['fields.update', 'refresh']);

  // 2. Batch ToggleLatex (2 formulas): Fields.Update runs once for the entire batch
  calls.length = 0;
  text = '$a=1$ $b=2$';
  selection.Start = 0; selection.End = text.length; selection.Text = text;
  selection.Range = doc.Range(0, text.length);
  c.batchLatexToMathType(doc, selection, c.findAllLatexMatches(text), { start: 0, end: text.length, fontPt: 12 });
  while (timers.length) timers.shift()();
  assert.deepEqual(calls, ['fields.update', 'refresh']);

  // 3. Single ToggleLatex (LaTeX -> MathType)
  calls.length = 0;
  text = 'x $y=1$ z';
  selection.Start = 4; selection.End = 4; selection.Text = '';
  selection.Range = doc.Range(4, 4);
  selection.Paragraphs = { Item: () => ({ Range: { Start: 0, End: text.length, Text: text } }) };
  c.ToggleLatex();
  while (timers.length) timers.shift()();
  assert.deepEqual(calls, ['fields.update', 'refresh']);
}
console.log('WPS Fields.Update post-insert refresh regression tests passed');

// 13. OLE CFB/EMF MTEF signature stability, abort HTTP-0 suppression, and fallback recovery.
{
  const c = controller();
  c.atob = s => Buffer.from(s, 'base64').toString('latin1');

  // Build a minimal valid v3 CFB (512-byte sector) with an Equation Native mini-stream.
  function makeCfb(mtefPayload, rootTimestampByte = 0) {
    const sector = 512;
    const totalSectors = 6; // header + FAT + dir + miniFAT + miniStream + slack
    const buf = Buffer.alloc(totalSectors * sector, 0);
    // Header
    Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]).copy(buf, 0);
    buf.writeUInt16LE(0x003e, 24);
    buf.writeUInt16LE(0x0003, 26);
    buf.writeUInt16LE(0xfffe, 28);
    buf.writeUInt16LE(9, 30); // sector shift = 512
    buf.writeUInt16LE(6, 32); // mini sector shift = 64
    buf.writeUInt32LE(1, 44); // 1 FAT sector
    buf.writeUInt32LE(1, 48); // firstDirSect = 1
    buf.writeUInt32LE(4096, 56); // miniCutoff
    buf.writeUInt32LE(2, 60); // firstMiniFatSect = 2
    buf.writeUInt32LE(1, 64); // numMiniFatSects = 1
    buf.writeUInt32LE(0xfffffffe, 68); // ENDOFCHAIN DIFAT
    buf.writeUInt32LE(0, 76); // DIFAT[0] = FAT sector 0
    for (let i = 1; i < 109; i++) buf.writeUInt32LE(0xffffffff, 76 + i * 4);

    // Sector 0 (offset 512): FAT
    const fatOff = 1 * sector;
    for (let i = 0; i < 128; i++) buf.writeUInt32LE(0xffffffff, fatOff + i * 4);
    buf.writeUInt32LE(0xfffffffd, fatOff + 0 * 4); // FATSECT
    buf.writeUInt32LE(0xfffffffe, fatOff + 1 * 4); // dir ENDOFCHAIN
    buf.writeUInt32LE(0xfffffffe, fatOff + 2 * 4); // miniFAT ENDOFCHAIN
    buf.writeUInt32LE(0xfffffffe, fatOff + 3 * 4); // miniStream ENDOFCHAIN

    // Sector 1 (offset 1024): Directory (Root Entry + Equation Native)
    const dirOff = 2 * sector;
    function writeDir(slot, name, type, startSect, size, tsByte) {
      const off = dirOff + slot * 128;
      const u16 = Buffer.from(name + '\0', 'utf16le');
      u16.copy(buf, off);
      buf.writeUInt16LE(u16.length, off + 64);
      buf[off + 66] = type;
      buf.writeUInt32LE(0xffffffff, off + 68);
      buf.writeUInt32LE(0xffffffff, off + 72);
      buf.writeUInt32LE(type === 5 ? 1 : 0xffffffff, off + 76);
      buf.fill(tsByte, off + 100, off + 116); // Creation/Modified FILETIME
      buf.writeUInt32LE(startSect, off + 116);
      buf.writeUInt32LE(size, off + 120);
    }
    const eqnHdr = Buffer.alloc(28, 0);
    eqnHdr.writeUInt16LE(28, 0);
    eqnHdr.writeUInt32LE(mtefPayload.length, 8);
    const eqnNative = Buffer.concat([eqnHdr, mtefPayload]);
    writeDir(0, 'Root Entry', 5, 3, 64, rootTimestampByte);
    writeDir(1, 'Equation Native', 2, 0, eqnNative.length, rootTimestampByte);

    // Sector 2 (offset 1536): MiniFAT
    const mfOff = 3 * sector;
    for (let i = 0; i < 128; i++) buf.writeUInt32LE(0xffffffff, mfOff + i * 4);
    buf.writeUInt32LE(0xfffffffe, mfOff);

    // Sector 3 (offset 2048): MiniStream
    eqnNative.copy(buf, 4 * sector);
    return buf.toString('base64');
  }

  function makeEmf(mtefPayload) {
    const comment = Buffer.concat([Buffer.from('AppsMFCC'), Buffer.alloc(10, 0), Buffer.from('Design Science, Inc.\0'), mtefPayload]);
    const recSize = Math.ceil((12 + comment.length) / 4) * 4;
    const buf = Buffer.alloc(88 + recSize + 8, 0);
    buf.writeUInt32LE(1, 0);
    buf.writeUInt32LE(88, 4);
    Buffer.from(' EMF').copy(buf, 40);
    buf.writeUInt32LE(70, 88); // EMR_COMMENT
    buf.writeUInt32LE(recSize, 92);
    buf.writeUInt32LE(comment.length, 96);
    comment.copy(buf, 100);
    buf.writeUInt32LE(14, 88 + recSize); // EMR_EOF
    buf.writeUInt32LE(8, 88 + recSize + 4);
    return buf.toString('base64');
  }

  const mtef1 = Buffer.from([5, 0, 1, 0, 1, 2, 3, 4]);
  const mtef2 = Buffer.from([5, 0, 1, 0, 1, 2, 3, 9]);
  const wrapOle = (b64, prefix = 'pkg:', objId = '_1') =>
    `<${prefix}package><${prefix}part ${prefix}name="/word/embeddings/oleObject1.bin"><${prefix}binaryData>${b64}</${prefix}binaryData></${prefix}part>` +
    `<${prefix}part ${prefix}name="/word/document.xml"><${prefix}xmlData><o:OLEObject ObjectID="${objId}" ProgID="Equation.DSMT4"/></${prefix}xmlData></${prefix}part></${prefix}package>`;
  const wrapEmf = b64 =>
    `<pkg:package><pkg:part pkg:name="/word/media/image1.emf"><pkg:binaryData>${b64}</pkg:binaryData></pkg:part></pkg:package>`;

  const sigTs1 = c.toggleSourceSignature(wrapOle(makeCfb(mtef1, 0x11), 'pkg:', '_111'), false);
  const sigTs2 = c.toggleSourceSignature(wrapOle(makeCfb(mtef1, 0x77), '', '_999'), false);
  const sigEmf = c.toggleSourceSignature(wrapEmf(makeEmf(mtef1)), false);
  const sigMutated = c.toggleSourceSignature(wrapOle(makeCfb(mtef2, 0x11)), false);
  assert.equal(sigTs1, sigTs2, 'OLE CFB timestamps and package prefix noise must not change signature');
  assert.equal(sigTs1, sigEmf, 'OLE MTEF and EMF AppsMFCC MTEF signatures must match');
  assert.notEqual(sigTs1, sigMutated, 'genuine MTEF equation change must change signature');
}
{
  // Aborted in-flight requests from finish(error) must NOT inject spurious "HTTP 0" into alert details.
  const c = controller();
  const callbacks = [];
  const alerts = [];
  const targets = [
    { start: 10, end: 11, isOmml: false },
    { start: 20, end: 21, isOmml: false }
  ];
  let sigCalls = 0;
  const doc = {
    get Content() { return { End: 100 }; },
    Range(start) {
      return {
        Start: start,
        End: start + 1,
        Text: '',
        get WordOpenXML() { return '<m:oMath>' + (++sigCalls) + '</m:oMath>'; }
      };
    }
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: { Start: 0, End: 100 } } };
  mockToggleProgress(c);
  c.alert = msg => alerts.push(msg);
  c.ajaxPost = (_url, _payload, done) => {
    callbacks.push(done);
    return { abort() { done(0, ''); } };
  };
  c.batchEquationToLatex(doc, c.wps.Application.Selection, targets);
  // First slot returns 200 OK; second slot gets aborted with status 0 during finish(error).
  callbacks[0](200, '{"result":"x"}');
  assert.equal(alerts.length, 1);
  assert.match(alerts[0], /The equation changed during Toggle LaTeX at 20\./);
  assert.equal(alerts[0].includes('HTTP 0'), false, 'aborted sibling request must not append HTTP 0');
}
{
  // findToggleObjectAt falls back cleanly when binary search hits a broken item;
  // readToggleEquationXml preserves full sourceEnd across fallback recovery.
  const c = controller();
  const targetXml = '<pkg:package><o:OLEObject ProgID="Equation.DSMT4"/></pkg:package>';
  const shape = { Range: { Start: 50, End: 76, WordOpenXML: targetXml } };
  const brokenMid = { get Range() { throw Error('stale'); } };
  const collection = {
    Count: 3,
    Item(i) { return i === 2 ? brokenMid : shape; }
  };
  assert.equal(c.findToggleObjectAt(collection, 50), shape);

  const item = {
    start: 50,
    end: 51,
    sourceEnd: 76,
    isOmml: false,
    sourceObject: { Range: { Start: 50, End: 76, WordOpenXML: '' } }
  };
  const liveShortShape = { Range: { Start: 50, End: 51, WordOpenXML: '' } };
  const doc = {
    InlineShapes: { Count: 1, Item: () => liveShortShape },
    Range(start, end) {
      return {
        Start: start,
        End: end,
        InlineShapes: { Count: 1, Item: () => liveShortShape },
        WordOpenXML: end === 76 ? targetXml : ''
      };
    }
  };
  c.wps = { Application: { ActiveDocument: doc, Selection: {} } };
  assert.equal(c.readToggleEquationXml(doc, item), targetXml);
  assert.equal(item.sourceEnd, 76, 'fallback must not shrink sourceEnd below full field range');
}
console.log('WPS CFB/EMF MTEF signature, HTTP-0 abort suppression, and fallback tests passed');
