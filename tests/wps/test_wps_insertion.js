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
  const c = controller();
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
