Attribute VB_Name = "EqnTideMacros"
Option Explicit

Private Const BRIDGE_ROOT As String = "http://127.0.0.1:17890"
Private Const BRIDGE_URL As String = "http://127.0.0.1:17890/convert/snippet"

' Word Key and Category Constants
Private Const wdKeyAlt As Long = 1024
Private Const wdKeyBackslash As Long = 220
Private Const wdKeyCategoryMacro As Long = 1
Private Const wdCollapseEnd As Long = 0
Private Const wdInlineShapeEmbeddedOLEObject As Long = 1
Private Const wdInlineShapeLinkedOLEObject As Long = 2
Private Const wdInlineShapePicture As Long = 3
Private Const wdInlineShapeLinkedPicture As Long = 4

Private Const BRIDGE_ERR_NONE As Long = 0
Private Const BRIDGE_ERR_HTTP As Long = 1
Private Const BRIDGE_ERR_CONNECT As Long = 2

Private m_CachedBridgeToken As String
Private m_SnippetHttp As Object
Private m_LastYield As Double
Private m_LastStatus As Double

' =========================================================================
' Ribbon Callbacks
' =========================================================================

Public Sub RibbonInsertEquation(control As IRibbonControl)
    InsertEquationMacro
End Sub

Public Sub RibbonEditEquation(control As IRibbonControl)
    EditEquationMacro
End Sub

Public Sub RibbonToggleLatex(control As IRibbonControl)
    ToggleLatexMacro
End Sub

Public Sub RibbonUpgradeMathType(control As IRibbonControl)
    UpgradeMathTypeMacro
End Sub

Public Sub RibbonBatchConverter(control As IRibbonControl)
    BatchConverterMacro
End Sub

' =========================================================================
' AutoExec / Setup Shortcuts (Alt + \)
' =========================================================================

Public Sub AutoExec()
    ' Word loads the global template add-in and ribbon tab automatically from STARTUP.
End Sub

' =========================================================================
' Core Macro: Toggle LaTeX <-> Equation (Alt + \)
' =========================================================================

Public Sub ToggleLatexMacro()
    Dim prevScreenUpdating As Boolean
    Dim prevPagination As Boolean
    Dim hasUndoRecord As Boolean
    prevScreenUpdating = True
    prevPagination = True
    On Error Resume Next
    prevScreenUpdating = Application.ScreenUpdating
    Application.ScreenUpdating = False
    prevPagination = Application.Options.Pagination
    Application.Options.Pagination = False
    Application.UndoRecord.StartCustomRecord "EqnTide Toggle LaTeX"
    If Err.Number = 0 Then hasUndoRecord = True
    Err.Clear
    On Error GoTo ErrorHandler

    m_LastYield = Timer
    m_LastStatus = m_LastYield

    Dim sel As Object
    Set sel = Selection

    Dim scope As Object
    Set scope = sel.Range.Duplicate
    Dim atCaret As Boolean
    atCaret = (scope.Start = scope.End)
    Dim caretPos As Long
    caretPos = scope.Start
    If atCaret Then Set scope = scope.Paragraphs(1).Range.Duplicate
    Dim txt As String
    txt = scope.Text

    ' Check if text has LaTeX delimiters $...$, $$...$$, \[...\], \(...\)
    Dim reg As Object
    Set reg = CreateObject("VBScript.RegExp")
    reg.Global = True
    reg.IgnoreCase = True
    reg.MultiLine = True
    ' Pattern to match $...$, $$...$$, \[...\], \(...\) without crossing paragraph breaks for inline $ / \(
    reg.Pattern = "(\$\$[\s\S]+?\$\$|\\\[[\s\S]+?\\\]|\\\([^\r\n\x07]+?\\\)|\$(?!\$)(?:\\.|[^$\\\r\n])+\$)"

    Dim matches As Object
    Set matches = reg.Execute(MaskEscapedLatexDollars(txt))

    Dim i As Long, handled As Boolean, stepCount As Long
    Dim batchFound As Long, batchConverted As Long
    Dim bridgeAborted As Boolean, lastHttpErr As String, errKind As Long
    Dim m As Object, subRng As Object
    If matches.Count > 0 Then
        Dim matchValues() As String
        Dim matchFirstIdx() As Long
        Dim matchLengths() As Long
        Dim validCount As Long
        ReDim matchValues(0 To matches.Count - 1)
        ReDim matchFirstIdx(0 To matches.Count - 1)
        ReDim matchLengths(0 To matches.Count - 1)
        validCount = 0
        Dim rawMatch As String
        For Each m In matches
            rawMatch = Mid$(txt, m.FirstIndex + 1, m.Length)
            If IsValidLatexMatch(rawMatch) Then
                matchValues(validCount) = rawMatch
                matchFirstIdx(validCount) = m.FirstIndex
                matchLengths(validCount) = m.Length
                validCount = validCount + 1
            End If
            YieldIfNeeded m_LastYield
        Next m

        If validCount > 0 Then
            Dim matchStarts() As Long, matchEnds() As Long
            ReDim matchStarts(0 To validCount - 1)
            ReDim matchEnds(0 To validCount - 1)
            Dim scanMin As Long, cumShift As Long
            scanMin = scope.Start
            cumShift = 0
            For i = 0 To validCount - 1
                Set subRng = ResolveMatchRange(scope, matchValues(i), scope.Start + matchFirstIdx(i) + cumShift, scanMin, scope.End)
                If Not subRng Is Nothing Then
                    matchStarts(i) = subRng.Start
                    matchEnds(i) = subRng.End
                    scanMin = subRng.End
                    cumShift = subRng.End - (scope.Start + matchFirstIdx(i) + matchLengths(i))
                Else
                    matchStarts(i) = -1
                    matchEnds(i) = -1
                End If
                If (i And 31) = 0 Then UpdateProgressStatus m_LastStatus, i, 0
                YieldIfNeeded m_LastYield
            Next i

            Set subRng = scope.Duplicate
            For i = validCount - 1 To 0 Step -1
                If matchStarts(i) < 0 Then
                    If Not atCaret Then
                        Err.Raise vbObjectError + 112, , "Word's text range differs from the selected formula. Select the formula alone."
                    End If
                ElseIf Not atCaret Or (matchStarts(i) <= caretPos And caretPos <= matchEnds(i)) Then
                    subRng.SetRange matchStarts(i), matchEnds(i)
                    If subRng.Text <> matchValues(i) Then
                        Err.Raise vbObjectError + 112, , "Word's text range differs from the selected formula. Select the formula alone."
                    End If
                    batchFound = batchFound + 1
                    Dim rawLatex As String, flatOpc As String
                    rawLatex = StripDelimiters(matchValues(i))
                    flatOpc = CallBridgeEx(rawLatex, "latex", "flat_opc_mathtype", False, errKind, lastHttpErr)
                    If errKind = BRIDGE_ERR_CONNECT Then
                        bridgeAborted = True
                        Exit For
                    End If
                    If Len(flatOpc) > 0 Then
                        InsertFlatOpcInline subRng, flatOpc, False
                        batchConverted = batchConverted + 1
                    End If
                    handled = True
                    stepCount = stepCount + 1
                    UpdateProgressStatus m_LastStatus, stepCount, validCount
                    YieldIfNeeded m_LastYield
                    If atCaret Then Exit For
                End If
            Next i
        End If
    End If
    If bridgeAborted Or handled Then GoTo Cleanup

    Set scope = sel.Range.Duplicate
    Dim latexStr As String
    Dim omathsCol As Object, om As Object, mathRange As Object, omStart As Long
    Set omathsCol = scope.OMaths
    If omathsCol.Count = 0 And atCaret Then Set omathsCol = sel.OMaths
    Dim omCount As Long, omIdx As Long
    omCount = omathsCol.Count
    If omCount > 0 Then
        Dim omList() As Object
        ReDim omList(1 To omCount)
        omIdx = 0
        For Each om In omathsCol
            omIdx = omIdx + 1
            If omIdx > UBound(omList) Then ReDim Preserve omList(1 To omIdx + 64)
            Set omList(omIdx) = om
            YieldIfNeeded m_LastYield
        Next om
        For i = omIdx To 1 Step -1
            Set om = omList(i)
            Set mathRange = om.Range.Duplicate
            omStart = mathRange.Start
            batchFound = batchFound + 1
            latexStr = CallBridgeEx(CompactFlatOpc(mathRange.WordOpenXML), "flat_opc", "latex", False, errKind, lastHttpErr)
            If errKind = BRIDGE_ERR_CONNECT Then
                bridgeAborted = True
                Exit For
            End If
            If Len(latexStr) > 0 Then
                om.Remove
                mathRange.SetRange omStart, omStart
                mathRange.Text = "$" & latexStr & "$"
                mathRange.Font.Position = 0
                mathRange.Font.Subscript = False
                mathRange.Font.Superscript = False
                batchConverted = batchConverted + 1
            End If
            handled = True
            stepCount = stepCount + 1
            UpdateProgressStatus m_LastStatus, stepCount, omIdx
            YieldIfNeeded m_LastYield
        Next i
    End If
    If bridgeAborted Then GoTo Cleanup

    Dim shapesCol As Object, shape As Object, shapeRange As Object, shapeXml As String
    Set shapesCol = scope.InlineShapes
    If shapesCol.Count = 0 Then Set shapesCol = sel.InlineShapes
    Dim rawShapeCount As Long
    rawShapeCount = shapesCol.Count
    If rawShapeCount > 0 Then
        Dim shapeList() As Object
        Dim shapeClass() As Long
        ReDim shapeList(1 To rawShapeCount)
        ReDim shapeClass(1 To rawShapeCount)
        Dim eqCandidateCount As Long, scanIdx As Long, cls As Long
        eqCandidateCount = 0
        scanIdx = 0
        For Each shape In shapesCol
            scanIdx = scanIdx + 1
            cls = QuickClassifyShape(shape)
            If cls >= 0 Then
                eqCandidateCount = eqCandidateCount + 1
                If eqCandidateCount > UBound(shapeList) Then
                    ReDim Preserve shapeList(1 To eqCandidateCount + 64)
                    ReDim Preserve shapeClass(1 To eqCandidateCount + 64)
                End If
                Set shapeList(eqCandidateCount) = shape
                shapeClass(eqCandidateCount) = cls
            End If
            If (scanIdx And 31) = 0 Then UpdateProgressStatus m_LastStatus, scanIdx, 0
            YieldIfNeeded m_LastYield
        Next shape

        For i = eqCandidateCount To 1 Step -1
            Set shape = shapeList(i)
            Set shapeRange = shape.Range.Duplicate
            cls = shapeClass(i)
            shapeXml = ""
            On Error Resume Next
            shapeXml = shapeRange.WordOpenXML
            On Error GoTo ErrorHandler
            If cls = 1 Then
                batchFound = batchFound + 1
            ElseIf IsEquationShape(shape, shapeXml) Then
                batchFound = batchFound + 1
            Else
                shapeXml = ""
            End If
            If Len(shapeXml) > 0 Then
                latexStr = CallBridgeEx(CompactFlatOpc(shapeXml), "flat_opc", "latex", False, errKind, lastHttpErr)
                If errKind = BRIDGE_ERR_CONNECT Then
                    bridgeAborted = True
                    Exit For
                End If
                If Len(latexStr) > 0 Then
                    TrimTrailingRangeBreaks shapeRange
                    shapeRange.Text = "$" & latexStr & "$"
                    shapeRange.Font.Position = 0
                    shapeRange.Font.Subscript = False
                    shapeRange.Font.Superscript = False
                    batchConverted = batchConverted + 1
                End If
                handled = True
                stepCount = stepCount + 1
                UpdateProgressStatus m_LastStatus, stepCount, eqCandidateCount
            End If
            YieldIfNeeded m_LastYield
        Next i
    End If
    If bridgeAborted Or handled Then GoTo Cleanup
    If atCaret Then
        MsgBox "Place the cursor inside a delimited LaTeX formula or select an equation.", vbInformation, "EqnTide"
    Else
        If shapesCol.Count > 0 Or omathsCol.Count > 0 Or InStr(txt, ChrW$(1)) > 0 Or InStr(txt, ChrW$(&HFFFC)) > 0 Or InStr(txt, Chr$(7)) > 0 Or InStr(txt, vbTab) > 0 Then
            MsgBox "Place the cursor inside a delimited LaTeX formula or select an equation.", vbInformation, "EqnTide"
            GoTo Cleanup
        End If
        ' No delimiters, assume entire selected text is pure LaTeX formula (single short block)
        Dim cleanLatex As String
        cleanLatex = Trim(txt)
        Do While Len(cleanLatex) > 0 And (Right$(cleanLatex, 1) = vbCr Or Right$(cleanLatex, 1) = vbLf)
            cleanLatex = Left$(cleanLatex, Len(cleanLatex) - 1)
        Loop
        cleanLatex = Trim(cleanLatex)
        If Len(cleanLatex) > 1000 Or ((InStr(cleanLatex, vbCr) > 0 Or InStr(cleanLatex, vbLf) > 0) And Left$(cleanLatex, 6) <> "\begin") Then
            MsgBox "Place the cursor inside a delimited LaTeX formula or select an equation.", vbInformation, "EqnTide"
            GoTo Cleanup
        End If
        If Len(cleanLatex) > 0 Then
            Dim opc As String
            opc = CallBridge(cleanLatex, "latex", "flat_opc_mathtype")
            If Len(opc) > 0 Then
                InsertFlatOpcInline sel.Range.Duplicate, opc, True
            End If
        End If
    End If

Cleanup:
    On Error Resume Next
    If hasUndoRecord Then Application.UndoRecord.EndCustomRecord
    Application.Options.Pagination = prevPagination
    Application.ScreenUpdating = prevScreenUpdating
    Application.StatusBar = False
    On Error GoTo 0
    If Not bridgeAborted Then
        If batchFound > 1 Then
            MsgBox "Toggle LaTeX Succeeded!" & vbCrLf & vbCrLf & _
                   "Found: " & batchFound & vbCrLf & _
                   "Converted: " & batchConverted & vbCrLf & _
                   "Errors: " & (batchFound - batchConverted), vbInformation, "EqnTide"
        ElseIf batchFound = 1 And batchConverted = 0 And Len(lastHttpErr) > 0 Then
            MsgBox lastHttpErr, vbExclamation, "EqnTide Bridge"
        End If
    End If
    Exit Sub

ErrorHandler:
    On Error Resume Next
    If hasUndoRecord Then Application.UndoRecord.EndCustomRecord
    Application.Options.Pagination = prevPagination
    Application.ScreenUpdating = prevScreenUpdating
    Application.StatusBar = False
    On Error GoTo 0
    MsgBox "Error in Toggle LaTeX: " & Err.Description, vbExclamation, "EqnTide"
End Sub

' =========================================================================
' Insert / Edit Equations
' =========================================================================

Public Sub InsertEquationMacro()
    EditInNativeDesktop False
End Sub

Public Sub EditEquationMacro()
    EditInNativeDesktop True
End Sub

Public Sub UpgradeMathTypeMacro()
    On Error GoTo UpgradeError

    Dim doc As Object
    Set doc = ActiveDocument
    If doc Is Nothing Then
        MsgBox "No active document found.", vbExclamation, "EqnTide"
        Exit Sub
    End If

    ' Check if document is saved on disk as .docx
    Dim docPath As String
    docPath = doc.FullName
    If Len(doc.Path) = 0 Or LCase(Right(doc.Name, 5)) <> ".docx" Then
        Dim answer As VbMsgBoxResult
        answer = MsgBox("To upgrade all legacy MathType equations with the high-speed converter, please save your document as a .docx file first." & vbCrLf & vbCrLf & _
                        "Would you like to open the Batch Document Converter instead?", vbYesNo + vbInformation, "EqnTide")
        If answer = vbYes Then
            BatchConverterMacro
        End If
        Exit Sub
    End If

    Dim confirmMsg As String
    confirmMsg = "Upgrade all legacy MathType equations in:" & vbCrLf & docPath & vbCrLf & vbCrLf & _
                 "Click [Yes] to upgrade to Modern MathType 7 (OLE DSMT4 + EMF-MTEF)." & vbCrLf & _
                 "Click [No] to upgrade to Native Word OMML (<m:oMath>)." & vbCrLf & _
                 "Click [Cancel] to abort."

    Dim choice As VbMsgBoxResult
    choice = MsgBox(confirmMsg, vbYesNoCancel + vbQuestion, "EqnTide - Upgrade Equations")
    If choice = vbCancel Then Exit Sub

    Dim targetMode As String
    If choice = vbYes Then
        targetMode = "upgrade_mathtype"
    Else
        targetMode = "upgrade_omml"
    End If

    If doc.ReadOnly Then
        MsgBox "This document is read-only. Save a writable copy before upgrading it.", vbInformation, "EqnTide"
        Exit Sub
    End If
    Dim reopenNeeded As Boolean
    Dim http As Object
    Set http = CreateObject("MSXML2.ServerXMLHTTP.6.0")
    http.Open "POST", BRIDGE_ROOT & "/convert/document", False
    http.setRequestHeader "Content-Type", "application/json"
    SetBridgeTokenHeader http

    Dim payload As String
    payload = "{""input_path"":" & JsonEscape(docPath) & ",""target"":" & JsonEscape(targetMode) & "}"
    doc.Save
    If Not doc.Saved Then Err.Raise vbObjectError + 110, , "Word did not save the document."
    doc.Close 0
    Dim openDoc As Object
    For Each openDoc In Application.Documents
        If StrComp(openDoc.FullName, docPath, vbTextCompare) = 0 Then
            Err.Raise vbObjectError + 111, , "Word did not close the document."
        End If
    Next openDoc
    reopenNeeded = True
    http.send payload

    Documents.Open docPath
    reopenNeeded = False
    If http.Status = 200 Then
        Dim resp As String
        resp = http.responseText
        Dim found As String, converted As String, errors As String
        found = ExtractJsonField(resp, "found")
        converted = ExtractJsonField(resp, "converted")
        errors = ExtractJsonField(resp, "errors")

        MsgBox "Upgrade Succeeded!" & vbCrLf & vbCrLf & _
               "Equations Found: " & found & vbCrLf & _
               "Successfully Upgraded: " & converted & vbCrLf & _
               "Errors: " & errors, vbInformation, "EqnTide"
    Else
        MsgBox "Bridge Server returned HTTP " & http.Status & ": " & http.responseText, vbExclamation, "EqnTide"
    End If
    Exit Sub

UpgradeError:
    Dim detail As String
    detail = Err.Description
    If reopenNeeded Then
        On Error Resume Next
        Err.Clear
        Documents.Open docPath
        If Err.Number <> 0 Then detail = detail & vbCrLf & "Open the saved document at: " & docPath
        On Error GoTo 0
    End If
    MsgBox "Cannot upgrade equations: " & detail, vbExclamation, "EqnTide"
End Sub

Public Sub BatchConverterMacro()
    On Error GoTo LaunchError
    CreateObject("WScript.Shell").Run "eqntide://open?tab=batch", 1, False
    Exit Sub
LaunchError:
    MsgBox "Please install EqnTide desktop and register its URI handler.", vbExclamation, "EqnTide"
End Sub

Private Sub EditInNativeDesktop(ByVal editExisting As Boolean)
    On Error GoTo EditError
    Dim target As Object
    Set target = Selection.Range.Duplicate
    Dim inputData As String, inputFormat As String
    inputFormat = "mathml"
    If editExisting Then
        If Selection.OMaths.Count > 0 Then
            Set target = Selection.OMaths(1).Range.Duplicate
            inputData = target.WordOpenXML
            inputFormat = "flat_opc"
        ElseIf Selection.InlineShapes.Count > 0 Then
            Set target = Selection.InlineShapes(1).Range.Duplicate
            inputData = target.WordOpenXML
            inputFormat = "flat_opc"
        Else
            inputData = StripDelimiters(target.Text)
            inputFormat = "latex"
        End If
    End If

    ' Registered by the native NSIS installer or portable register-uri.ps1.
    CreateObject("WScript.Shell").Run "eqntide://open", 1, False
    Dim fontPt As Single
    fontPt = target.Font.Size
    If fontPt < 6 Or fontPt > 72 Then fontPt = 12
    Dim http As Object, response As String, sessionId As String
    Dim attempts As Long
    For attempts = 1 To 20
        On Error Resume Next
        Set http = CreateObject("MSXML2.ServerXMLHTTP.6.0")
        http.setTimeouts 1000, 1000, 2000, 3000
        http.Open "POST", BRIDGE_ROOT & "/session/open", False
        http.setRequestHeader "Content-Type", "application/json"
        SetBridgeTokenHeader http
        http.send "{""data"":" & JsonEscape(inputData) & ",""format"":" & JsonEscape(inputFormat) & ",""target_mode"":""mathtype"",""font_pt"":" & Replace(CStr(fontPt), ",", ".") & "}"
        response = ""
        If Err.Number = 0 Then
            If http.Status = 200 Then response = http.responseText
        End If
        Err.Clear
        On Error GoTo EditError
        sessionId = ExtractJsonField(response, "session_id")
        If Len(sessionId) > 0 Then Exit For
        DoEvents
    Next attempts
    If Len(sessionId) = 0 Then Err.Raise vbObjectError + 101, , "The local bridge did not start."

    ' The bridge also launches/focuses the native editor for this exact session.
    Do
        Set http = CreateObject("MSXML2.ServerXMLHTTP.6.0")
        http.setTimeouts 1000, 1000, 2000, 3000
        http.Open "GET", BRIDGE_ROOT & "/session/" & sessionId & "/wait?timeout=1", False
        SetBridgeTokenHeader http
        http.send
        If http.Status <> 200 Then Err.Raise vbObjectError + 102, , "The editing session is no longer available."
        response = http.responseText
        Dim state As String
        state = ExtractJsonField(response, "status")
        If state = "cancelled" Then Exit Sub
        If state = "ready" Then
            Dim flatOpc As String
            flatOpc = ExtractJsonField(response, "flat_opc")
            If Len(flatOpc) = 0 Then Err.Raise vbObjectError + 103, , "The editor returned an empty result."
            InsertFlatOpcInline target, flatOpc, True
            Exit Sub
        End If
        DoEvents
    Loop
EditError:
    Dim message As String
    message = Err.Description
    On Error Resume Next
    If Len(sessionId) > 0 Then
        http.Open "POST", BRIDGE_ROOT & "/session/" & sessionId & "/cancel", False
        http.setRequestHeader "Content-Type", "application/json"
        SetBridgeTokenHeader http
        http.send "{}"
    End If
    MsgBox "Cannot complete native editing: " & message, vbExclamation, "EqnTide"
End Sub

' =========================================================================
' Helper Functions
' =========================================================================

Private Sub YieldIfNeeded(ByRef lastYield As Double)
    Dim nowT As Double
    nowT = Timer
    If nowT < lastYield Or (nowT - lastYield) >= 0.15 Then
        DoEvents
        lastYield = Timer
    End If
End Sub

Private Sub UpdateProgressStatus(ByRef lastStatus As Double, ByVal doneCount As Long, ByVal totalCount As Long)
    Dim nowT As Double
    nowT = Timer
    If nowT < lastStatus Or (nowT - lastStatus) >= 0.25 Then
        On Error Resume Next
        If totalCount > 0 Then
            Application.StatusBar = "EqnTide: Converting equations (" & doneCount & " / " & totalCount & ")..."
        Else
            Application.StatusBar = "EqnTide: Scanning equations (" & doneCount & ")..."
        End If
        On Error GoTo 0
        lastStatus = Timer
    End If
End Sub

Private Sub TrimTrailingRangeBreaks(ByVal target As Object)
    Dim tail As Object
    Set tail = target.Duplicate
    Do While target.End > target.Start
        tail.SetRange target.End - 1, target.End
        If tail.Text <> vbCr And tail.Text <> vbLf And tail.Text <> Chr$(7) Then Exit Do
        target.End = target.End - 1
    Loop
End Sub

Private Function MaskEscapedLatexDollars(ByVal text As String) As String
    ' Keep offsets intact and read the actual formula from the original text.
    Dim maskedText As String
    maskedText = text
    Dim escapePos As Long, backslashStart As Long
    escapePos = InStr(1, text, "\$", vbBinaryCompare)
    Do While escapePos > 0
        backslashStart = escapePos
        Do While backslashStart > 1
            If Mid$(text, backslashStart - 1, 1) <> "\" Then Exit Do
            backslashStart = backslashStart - 1
        Loop
        If (escapePos - backslashStart) Mod 2 = 0 Then
            Mid$(maskedText, escapePos + 1, 1) = "x"
        End If
        escapePos = InStr(escapePos + 2, text, "\$", vbBinaryCompare)
    Loop
    MaskEscapedLatexDollars = maskedText
End Function

Private Function IsValidLatexMatch(ByVal rawMatch As String) As Boolean
    If Len(rawMatch) = 0 Or Len(rawMatch) > 2000 Then Exit Function
    If InStr(1, rawMatch, ChrW$(1), vbBinaryCompare) > 0 Then Exit Function
    If InStr(1, rawMatch, ChrW$(&HFFFC), vbBinaryCompare) > 0 Then Exit Function
    If InStr(1, rawMatch, Chr$(7), vbBinaryCompare) > 0 Then Exit Function
    If InStr(1, rawMatch, vbCr & vbCr, vbBinaryCompare) > 0 Then Exit Function
    If InStr(1, rawMatch, vbLf & vbLf, vbBinaryCompare) > 0 Then Exit Function
    If Len(Trim$(StripDelimiters(rawMatch))) = 0 Then Exit Function
    IsValidLatexMatch = True
End Function

' Returns 1 if definitely an equation, -1 if definitely NOT an equation, 0 if XML check is needed.
Private Function QuickClassifyShape(ByVal shape As Object) As Long
    On Error Resume Next
    If shape.AlternativeText = "EqnTide equation" Then
        QuickClassifyShape = 1
        Exit Function
    End If
    Dim st As Long
    st = 0
    st = shape.Type
    If st = wdInlineShapeEmbeddedOLEObject Or st = wdInlineShapeLinkedOLEObject Then
        Dim progId As String
        progId = ""
        progId = LCase$(shape.OLEFormat.ProgID)
        If Len(progId) > 0 Then
            If progId Like "equation.*" Or progId Like "mathtype*" Then
                QuickClassifyShape = 1
            Else
                QuickClassifyShape = -1
            End If
            Exit Function
        End If
        QuickClassifyShape = 0
        Exit Function
    End If
    If st = wdInlineShapePicture Or st = wdInlineShapeLinkedPicture Then
        QuickClassifyShape = 0
        Exit Function
    End If
    If st > 0 Then
        QuickClassifyShape = -1
    Else
        QuickClassifyShape = 0
    End If
    On Error GoTo 0
End Function

Private Function IsBloatedFlatOpcPart(ByVal lowerPartName As String) As Boolean
    If Len(lowerPartName) = 0 Then Exit Function
    Select Case lowerPartName
        Case "/word/styles.xml", "/word/styleswitheffects.xml", "/word/numbering.xml", _
             "/word/fonttable.xml", "/word/settings.xml", "/word/websettings.xml"
            IsBloatedFlatOpcPart = True
            Exit Function
    End Select
    If Left$(lowerPartName, 12) = "/word/theme/" _
        Or Left$(lowerPartName, 10) = "/docprops/" _
        Or Left$(lowerPartName, 11) = "/customxml/" Then
        IsBloatedFlatOpcPart = True
    End If
End Function

Private Function CompactFlatOpc(ByVal xml As String) As String
    If Len(xml) < 16384 Then
        CompactFlatOpc = xml
        Exit Function
    End If
    Dim firstPart As Long
    firstPart = InStr(1, xml, "<pkg:part ", vbBinaryCompare)
    If firstPart = 0 Then
        CompactFlatOpc = xml
        Exit Function
    End If

    Dim parts() As String
    Dim cap As Long, cnt As Long
    cap = 16
    ReDim parts(0 To cap - 1)
    parts(0) = Left$(xml, firstPart - 1)
    cnt = 1

    Dim pos As Long, tagEnd As Long, partEnd As Long, tailPos As Long
    Dim partName As String, nameStart As Long, nameEnd As Long, headerText As String
    pos = firstPart
    tailPos = firstPart
    Do While pos > 0
        tagEnd = InStr(pos, xml, ">", vbBinaryCompare)
        If tagEnd = 0 Then
            CompactFlatOpc = xml
            Exit Function
        End If
        If Mid$(xml, tagEnd - 1, 1) = "/" Then
            partEnd = tagEnd + 1
        Else
            partEnd = InStr(tagEnd + 1, xml, "</pkg:part>", vbBinaryCompare)
            If partEnd = 0 Then
                CompactFlatOpc = xml
                Exit Function
            End If
            partEnd = partEnd + 11
        End If

        headerText = Mid$(xml, pos, tagEnd - pos + 1)
        partName = ""
        nameStart = InStr(1, headerText, "pkg:name=""", vbBinaryCompare)
        If nameStart = 0 Then nameStart = InStr(1, headerText, "name=""", vbBinaryCompare)
        If nameStart > 0 Then
            nameStart = InStr(nameStart, headerText, """", vbBinaryCompare) + 1
            nameEnd = InStr(nameStart, headerText, """", vbBinaryCompare)
            If nameEnd > nameStart Then
                partName = LCase$(Mid$(headerText, nameStart, nameEnd - nameStart))
                If Left$(partName, 1) <> "/" Then partName = "/" & partName
            End If
        End If

        If Not IsBloatedFlatOpcPart(partName) Then
            If cnt >= cap Then
                cap = cap * 2
                ReDim Preserve parts(0 To cap - 1)
            End If
            parts(cnt) = Mid$(xml, pos, partEnd - pos)
            cnt = cnt + 1
        End If

        tailPos = partEnd
        pos = InStr(partEnd, xml, "<pkg:part ", vbBinaryCompare)
    Loop

    If tailPos <= Len(xml) Then
        If cnt >= cap Then
            cap = cap + 1
            ReDim Preserve parts(0 To cap - 1)
        End If
        parts(cnt) = Mid$(xml, tailPos)
        cnt = cnt + 1
    End If

    ReDim Preserve parts(0 To cnt - 1)
    CompactFlatOpc = Join(parts, "")
End Function

Private Sub InsertFlatOpcInline(ByVal target As Object, ByVal flatOpc As String, ByVal moveCursorAfter As Boolean)
    TrimTrailingRangeBreaks target
    Dim tail As Object
    Set tail = target.Duplicate

    Dim insertPos As Long
    insertPos = target.Start
    Dim replaced As Long
    replaced = target.End - insertPos
    Dim originalEnd As Long
    originalEnd = target.StoryLength
    Dim suffixLen As Long
    suffixLen = 32
    If target.End + suffixLen > originalEnd Then suffixLen = originalEnd - target.End
    Dim suffix As String
    If suffixLen > 0 Then
        tail.SetRange target.End, target.End + suffixLen
        suffix = tail.Text
    Else
        suffix = ""
    End If

    target.InsertXML flatOpc

    Dim newEnd As Long
    newEnd = target.StoryLength
    Dim insertedEnd As Long
    insertedEnd = insertPos + newEnd - originalEnd + replaced

    If insertedEnd > insertPos And insertedEnd <= newEnd Then
        Dim verifyLen As Long
        verifyLen = Len(suffix)
        Dim actualSuffix As String
        If verifyLen > 0 And insertedEnd + verifyLen <= newEnd Then
            tail.SetRange insertedEnd, insertedEnd + verifyLen
            actualSuffix = tail.Text
        ElseIf verifyLen = 0 Then
            actualSuffix = ""
        Else
            actualSuffix = vbNullString
        End If
        If actualSuffix = suffix Then
            Dim breaks As Long
            Dim ch As String
            For breaks = 1 To 2
                If insertedEnd <= insertPos Then Exit For
                tail.SetRange insertedEnd - 1, insertedEnd
                ch = tail.Text
                If ch <> vbCr And ch <> vbLf Then Exit For
                tail.Delete
                insertedEnd = insertedEnd - 1
            Next breaks
        End If
    End If

    If moveCursorAfter And insertedEnd >= insertPos Then
        Dim afterRng As Object
        Set afterRng = target.Duplicate
        afterRng.SetRange insertedEnd, insertedEnd
        afterRng.Select
        On Error Resume Next
        Selection.Collapse wdCollapseEnd
        Selection.Font.Position = 0
        Selection.Font.Subscript = False
        Selection.Font.Superscript = False
        On Error GoTo 0
    End If
End Sub

Private Function IsEquationShape(ByVal shape As Object, ByVal shapeXml As String) As Boolean
    Dim cls As Long
    cls = QuickClassifyShape(shape)
    If cls = 1 Then
        IsEquationShape = True
        Exit Function
    ElseIf cls = -1 Then
        Exit Function
    End If
    If InStr(1, shapeXml, "Equation.", vbTextCompare) > 0 _
        Or InStr(1, shapeXml, "MathType", vbTextCompare) > 0 _
        Or InStr(1, shapeXml, "EqnTide equation", vbTextCompare) > 0 Then
        IsEquationShape = True
    End If
End Function

Private Function ResolveMatchRange(ByVal scope As Object, ByVal expected As String, ByVal guessStart As Long, ByVal minStart As Long, ByVal maxEnd As Long) As Object
    Dim matchLen As Long
    matchLen = Len(expected)
    If matchLen = 0 Or maxEnd - minStart < matchLen Then Exit Function

    Dim probe As Object
    Set probe = scope.Duplicate
    If guessStart >= minStart And guessStart + matchLen <= maxEnd Then
        probe.SetRange guessStart, guessStart + matchLen
        If probe.Text = expected Then
            Set ResolveMatchRange = probe
            Exit Function
        End If
    End If

    Dim winStart As Long, winEnd As Long
    winStart = guessStart - 256
    If winStart < minStart Then winStart = minStart
    winEnd = guessStart + matchLen + 256
    If winEnd > maxEnd Then winEnd = maxEnd
    If winEnd - winStart >= matchLen Then
        Set ResolveMatchRange = ScanRangeForMatch(probe, expected, winStart, winEnd)
        If Not ResolveMatchRange Is Nothing Then Exit Function
    End If

    Set ResolveMatchRange = ScanRangeForMatch(probe, expected, minStart, maxEnd)
End Function

Private Function ScanRangeForMatch(ByVal probe As Object, ByVal expected As String, ByVal rangeStart As Long, ByVal rangeEnd As Long) As Object
    Dim matchLen As Long
    matchLen = Len(expected)
    If rangeEnd - rangeStart < matchLen Then Exit Function

    Dim firstCh As String
    firstCh = Left$(expected, 1)
    Dim chunkSpan As Long, overlap As Long
    chunkSpan = 512
    overlap = matchLen + 8
    Dim chunkStart As Long, chunkEnd As Long, chunkText As String
    Dim pos As Long, scanEnd As Long, hitIdx As Long, guessPos As Long
    For chunkStart = rangeStart To rangeEnd - matchLen Step chunkSpan
        chunkEnd = chunkStart + chunkSpan + overlap
        If chunkEnd > rangeEnd Then chunkEnd = rangeEnd
        probe.SetRange chunkStart, chunkEnd
        chunkText = probe.Text
        hitIdx = InStr(1, chunkText, expected, vbBinaryCompare)
        If hitIdx > 0 Then
            If Len(chunkText) = (chunkEnd - chunkStart) Then
                guessPos = chunkStart + hitIdx - 1
                If guessPos + matchLen <= rangeEnd Then
                    probe.SetRange guessPos, guessPos + matchLen
                    If probe.Text = expected Then
                        Set ScanRangeForMatch = probe
                        Exit Function
                    End If
                End If
            End If
            scanEnd = chunkEnd - matchLen
            If scanEnd > chunkStart + chunkSpan + overlap - matchLen Then scanEnd = chunkStart + chunkSpan + overlap - matchLen
            For pos = chunkStart To scanEnd
                probe.SetRange pos, pos + 1
                If probe.Text = firstCh Then
                    probe.SetRange pos, pos + matchLen
                    If probe.Text = expected Then
                        Set ScanRangeForMatch = probe
                        Exit Function
                    End If
                End If
                YieldIfNeeded m_LastYield
            Next pos
        End If
        YieldIfNeeded m_LastYield
        If chunkEnd >= rangeEnd Then Exit For
    Next chunkStart
End Function

Private Function StripDelimiters(ByVal s As String) As String
    s = Trim(s)
    If Left(s, 2) = "$$" And Right(s, 2) = "$$" Then
        StripDelimiters = Mid(s, 3, Len(s) - 4)
    ElseIf Left(s, 2) = "\[" And Right(s, 2) = "\]" Then
        StripDelimiters = Mid(s, 3, Len(s) - 4)
    ElseIf Left(s, 2) = "\(" And Right(s, 2) = "\)" Then
        StripDelimiters = Mid(s, 3, Len(s) - 4)
    ElseIf Left(s, 1) = "$" And Right(s, 1) = "$" Then
        StripDelimiters = Mid(s, 2, Len(s) - 2)
    Else
        StripDelimiters = s
    End If
End Function

Private Function ReadBridgeToken() As String
    If Len(m_CachedBridgeToken) > 0 Then
        ReadBridgeToken = m_CachedBridgeToken
        Exit Function
    End If
    On Error GoTo TokenError
    Dim shell As Object, fso As Object, tokenPath As String, ts As Object
    Set shell = CreateObject("WScript.Shell")
    tokenPath = shell.ExpandEnvironmentStrings("%LOCALAPPDATA%") & "\eqntide\EqnTide\data\bridge.token"
    Set fso = CreateObject("Scripting.FileSystemObject")
    If fso.FileExists(tokenPath) Then
        Set ts = fso.OpenTextFile(tokenPath, 1, False)
        If Not ts.AtEndOfStream Then
            m_CachedBridgeToken = Trim(Replace(Replace(ts.ReadAll, vbCr, ""), vbLf, ""))
        End If
        ts.Close
    End If
    ReadBridgeToken = m_CachedBridgeToken
    Exit Function
TokenError:
    ReadBridgeToken = ""
End Function

Private Sub SetBridgeTokenHeader(ByVal http As Object)
    Dim token As String
    token = ReadBridgeToken()
    If Len(token) > 0 Then
        http.setRequestHeader "X-EqnTide-Token", token
    End If
End Sub

Private Function CallBridge(ByVal dataStr As String, ByVal fromFmt As String, ByVal toFmt As String) As String
    Dim errKind As Long, errMsg As String
    CallBridge = CallBridgeEx(dataStr, fromFmt, toFmt, True, errKind, errMsg)
End Function

Private Function CallBridgeEx(ByVal dataStr As String, ByVal fromFmt As String, ByVal toFmt As String, ByVal showHttpErrors As Boolean, ByRef errKind As Long, ByRef errMsg As String) As String
    errKind = BRIDGE_ERR_NONE
    errMsg = ""
    Dim payload As String
    payload = "{""data"":" & JsonEscape(dataStr) & ",""from"":""" & fromFmt & """,""to"":""" & toFmt & """}"

    Dim attempt As Long
    For attempt = 1 To 2
        On Error GoTo HttpError
        If m_SnippetHttp Is Nothing Then
            Set m_SnippetHttp = CreateObject("MSXML2.ServerXMLHTTP.6.0")
        End If
        m_SnippetHttp.setTimeouts 2000, 2000, 5000, 15000
        m_SnippetHttp.Open "POST", BRIDGE_URL, False
        m_SnippetHttp.setRequestHeader "Content-Type", "application/json"
        SetBridgeTokenHeader m_SnippetHttp
        m_SnippetHttp.send payload

        If m_SnippetHttp.Status = 200 Then
            CallBridgeEx = ExtractJsonResult(m_SnippetHttp.responseText)
        Else
            errKind = BRIDGE_ERR_HTTP
            errMsg = "Bridge Server returned HTTP " & m_SnippetHttp.Status & ": " & m_SnippetHttp.responseText
            If showHttpErrors Then
                MsgBox errMsg, vbExclamation, "EqnTide Bridge"
            End If
            CallBridgeEx = ""
        End If
        Exit Function

HttpError:
        Set m_SnippetHttp = Nothing
        Resume RetryNext
RetryNext:
    Next attempt

    errKind = BRIDGE_ERR_CONNECT
    errMsg = "Cannot connect to Bridge Server at " & BRIDGE_URL
    MsgBox errMsg & vbCrLf & _
           "Please ensure eqntide-desktop.exe (or start-server.bat) is running!", vbCritical, "EqnTide"
    CallBridgeEx = ""
End Function

Private Function JsonEscape(ByVal s As String) As String
    If Len(s) = 0 Then
        JsonEscape = """"""
        Exit Function
    End If
    Dim out As String
    out = s
    If InStr(1, out, "\", vbBinaryCompare) > 0 Then out = Replace$(out, "\", "\\", 1, -1, vbBinaryCompare)
    If InStr(1, out, """", vbBinaryCompare) > 0 Then out = Replace$(out, """", "\""", 1, -1, vbBinaryCompare)
    If InStr(1, out, vbCr, vbBinaryCompare) > 0 Then out = Replace$(out, vbCr, "\r", 1, -1, vbBinaryCompare)
    If InStr(1, out, vbLf, vbBinaryCompare) > 0 Then out = Replace$(out, vbLf, "\n", 1, -1, vbBinaryCompare)
    If InStr(1, out, vbTab, vbBinaryCompare) > 0 Then out = Replace$(out, vbTab, "\t", 1, -1, vbBinaryCompare)
    If InStr(1, out, ChrW$(8), vbBinaryCompare) > 0 Then out = Replace$(out, ChrW$(8), "\b", 1, -1, vbBinaryCompare)
    If InStr(1, out, ChrW$(12), vbBinaryCompare) > 0 Then out = Replace$(out, ChrW$(12), "\f", 1, -1, vbBinaryCompare)
    Dim c As Long
    For c = 0 To 31
        If c <> 8 And c <> 9 And c <> 10 And c <> 12 And c <> 13 Then
            If InStr(1, out, ChrW$(c), vbBinaryCompare) > 0 Then
                out = Replace$(out, ChrW$(c), "\u" & Right$("0000" & Hex$(c), 4), 1, -1, vbBinaryCompare)
            End If
        End If
    Next c
    JsonEscape = """" & out & """"
End Function

Private Function ExtractJsonResult(ByVal json As String) As String
    ExtractJsonResult = ExtractJsonField(json, "result")
End Function

Private Function ExtractJsonField(ByVal json As String, ByVal field As String) As String
    Dim pos As Long, i As Long, ch As String
    pos = InStr(1, json, """" & field & """", vbBinaryCompare)
    If pos = 0 Then Exit Function
    pos = InStr(pos + Len(field) + 2, json, ":", vbBinaryCompare)
    If pos = 0 Then Exit Function
    pos = pos + 1
    Do While pos <= Len(json)
        ch = Mid$(json, pos, 1)
        If ch <> " " And ch <> vbTab And ch <> vbCr And ch <> vbLf Then Exit Do
        pos = pos + 1
    Loop
    If pos > Len(json) Then Exit Function
    If Mid$(json, pos, 1) Like "#" Then
        i = pos
        Do While i <= Len(json)
            If Not (Mid$(json, i, 1) Like "#") Then Exit Do
            i = i + 1
        Loop
        ExtractJsonField = Mid$(json, pos, i - pos)
        Exit Function
    End If
    If Mid$(json, pos, 1) <> """" Then Exit Function

    i = pos + 1
    Dim chunks() As String
    Dim cap As Long, cnt As Long
    cap = 16
    cnt = 0
    ReDim chunks(0 To cap - 1)

    Dim nextQuote As Long, nextSlash As Long, escChar As String, decoded As String
    Do While i <= Len(json)
        nextQuote = InStr(i, json, """", vbBinaryCompare)
        If nextQuote = 0 Then Exit Function
        nextSlash = InStr(i, json, "\", vbBinaryCompare)
        If nextSlash = 0 Or nextQuote < nextSlash Then
            If cnt = 0 Then
                ExtractJsonField = Mid$(json, i, nextQuote - i)
                Exit Function
            End If
            If nextQuote > i Then
                If cnt >= cap Then
                    cap = cap * 2
                    ReDim Preserve chunks(0 To cap - 1)
                End If
                chunks(cnt) = Mid$(json, i, nextQuote - i)
                cnt = cnt + 1
            End If
            ReDim Preserve chunks(0 To cnt - 1)
            ExtractJsonField = Join(chunks, "")
            Exit Function
        End If

        If nextSlash > i Then
            If cnt >= cap Then
                cap = cap * 2
                ReDim Preserve chunks(0 To cap - 1)
            End If
            chunks(cnt) = Mid$(json, i, nextSlash - i)
            cnt = cnt + 1
        End If

        If nextSlash + 1 > Len(json) Then Exit Function
        escChar = Mid$(json, nextSlash + 1, 1)
        Select Case escChar
            Case "n": decoded = vbLf: i = nextSlash + 2
            Case "r": decoded = vbCr: i = nextSlash + 2
            Case "t": decoded = vbTab: i = nextSlash + 2
            Case "b": decoded = ChrW$(8): i = nextSlash + 2
            Case "f": decoded = ChrW$(12): i = nextSlash + 2
            Case "u"
                If nextSlash + 5 > Len(json) Then Exit Function
                decoded = ChrW$(CLng("&H" & Mid$(json, nextSlash + 2, 4)))
                i = nextSlash + 6
            Case Else
                decoded = escChar
                i = nextSlash + 2
        End Select
        If cnt >= cap Then
            cap = cap * 2
            ReDim Preserve chunks(0 To cap - 1)
        End If
        chunks(cnt) = decoded
        cnt = cnt + 1
    Loop
End Function
