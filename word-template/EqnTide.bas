Attribute VB_Name = "EqnTide"
Option Explicit

Private Const BRIDGE_ROOT As String = "http://127.0.0.1:17890"
Private Const BRIDGE_URL As String = "http://127.0.0.1:17890/convert/snippet"

Private mRibbon As IRibbonUI
Private mWordEvents As EqnTideEvents
Private mHandlingSelection As Boolean
Private mLastEquationDoc As Word.Document
Private mLastEquationStart As Long
Private mLastEquationEnd As Long

Public Sub RibbonOnLoad(ByVal ribbon As IRibbonUI)
    Set mRibbon = ribbon
    InitEquationEvents
    If Application.Documents.Count > 0 Then HandleEquationSelection Application.Selection
End Sub

Private Sub InitEquationEvents()
    If mWordEvents Is Nothing Then Set mWordEvents = New EqnTideEvents
    Set mWordEvents.App = Application
End Sub

Public Sub HandleEquationSelection(ByVal Sel As Word.Selection)
    If mHandlingSelection Then Exit Sub
    On Error GoTo SelectionFailed
    If mRibbon Is Nothing Then Exit Sub
    If Sel.InlineShapes.Count <> 1 Then GoTo NotEquation
    Dim picture As Word.InlineShape
    Set picture = Sel.InlineShapes(1)
    If Sel.Start <> picture.Range.Start Or Sel.End <> picture.Range.End Then GoTo NotEquation
    If Not IsEquationPicture(picture) Then GoTo NotEquation
    If Not mLastEquationDoc Is Nothing Then
        If mLastEquationDoc Is Sel.Document Then
            If mLastEquationStart = Sel.Start And mLastEquationEnd = Sel.End Then Exit Sub
        End If
    End If
    Set mLastEquationDoc = Sel.Document
    mLastEquationStart = Sel.Start
    mLastEquationEnd = Sel.End
    mHandlingSelection = True
    mRibbon.ActivateTab "EqnTideTab"
    mHandlingSelection = False
    Exit Sub
NotEquation:
    Set mLastEquationDoc = Nothing
    Exit Sub
SelectionFailed:
    mHandlingSelection = False
    Set mLastEquationDoc = Nothing
End Sub

Private Function IsEquationPicture(ByVal picture As Word.InlineShape) As Boolean
    On Error GoTo NotEquation
    If picture.AlternativeText = "EqnTide equation" Then
        IsEquationPicture = True
        Exit Function
    End If
    Dim progId As String
    On Error Resume Next
    progId = picture.OLEFormat.ProgID
    Err.Clear
    On Error GoTo NotEquation
    If LCase$(Left$(progId, 9)) = "equation." Or LCase$(Left$(progId, 8)) = "mathtype" Then
        IsEquationPicture = True
        Exit Function
    End If
    ' Standalone EMF equations have no OLE object. Recognize the embedded
    ' MathType comment, not the image extension or generic drawing markup.
    Dim xml As Object, binary As Object, payload As String
    Set xml = CreateObject("MSXML2.DOMDocument.6.0")
    xml.async = False
    xml.resolveExternals = False
    If Not xml.LoadXML(picture.Range.WordOpenXML) Then Exit Function
    For Each binary In xml.SelectNodes("//*[local-name()='binaryData']")
        binary.DataType = "bin.base64"
        payload = StrConv(binary.nodeTypedValue, vbUnicode)
        If InStr(1, payload, "AppsMFCC", vbBinaryCompare) > 0 Then
            IsEquationPicture = True
            Exit Function
        End If
    Next binary
NotEquation:
End Function

' =========================================================================
' Ribbon Callbacks
' =========================================================================

Public Sub RibbonToggleLatex(control As IRibbonControl)
    ToggleLatexMacro
End Sub

Public Sub RibbonInsertEquation(control As IRibbonControl)
    InsertEquationMacro
End Sub

Public Sub RibbonEditEquation(control As IRibbonControl)
    EditEquationMacro
End Sub

Public Sub RibbonUpgradeMathType(control As IRibbonControl)
    UpgradeMathTypeMacro
End Sub

' =========================================================================
' AutoExec / Setup Shortcuts (Alt + \)
' =========================================================================

Public Sub AutoExec()
    On Error Resume Next
    InitEquationEvents
    CustomizationContext = NormalTemplate
    KeyBindings.Add KeyCode:=BuildKeyCode(wdKeyAlt, wdKeyBackslash), _
                    KeyCategory:=wdKeyCategoryMacro, _
                    Command:="ToggleLatexMacro"
End Sub

' =========================================================================
' Core Macro: Toggle LaTeX <-> Equation (Alt + \)
' =========================================================================

Public Sub ToggleLatexMacro()
    On Error GoTo ErrorHandler

    Dim sel As Selection
    Set sel = Selection

    ' Case 1: Cursor is on / inside an existing OMML equation -> Convert to LaTeX text
    If sel.OMaths.Count > 0 Then
        Dim om As OMath
        Set om = sel.OMaths(1)
        Dim omXml As String
        omXml = om.Range.WordOpenXML

        Dim latexStr As String
        latexStr = CallBridge(omXml, "flat_opc", "latex")

        If Len(latexStr) > 0 Then
            om.Range.Text = "$" & latexStr & "$"
        End If
        Exit Sub
    End If

    ' MathType OLE created by Toggle/Insert/Edit must also toggle back to text.
    If sel.InlineShapes.Count = 1 Then
        Dim shapeRange As Range
        Set shapeRange = sel.InlineShapes(1).Range.Duplicate
        latexStr = CallBridge(shapeRange.WordOpenXML, "flat_opc", "latex")
        If Len(latexStr) > 0 Then shapeRange.Text = "$" & latexStr & "$"
        Exit Sub
    End If

    ' Case 2: Selected text contains LaTeX or plain text selected
    Dim txt As String
    txt = sel.Text

    If Len(Trim(txt)) = 0 Then
        ' Expand selection to current paragraph or word if nothing selected
        MsgBox "Please select a LaTeX expression (e.g. $x+1=2$) or an equation.", vbInformation, "EqnTide"
        Exit Sub
    End If

    ' Check if text has LaTeX delimiters $...$, $$...$$, \[...\], \(...\)
    Dim reg As Object
    Set reg = CreateObject("VBScript.RegExp")
    reg.Global = True
    reg.IgnoreCase = True
    reg.MultiLine = True
    ' Pattern to match $...$, $$...$$, \[...\], \(...\)
    reg.Pattern = "(\$\$[\s\S]+?\$\$|\\\[[\s\S]+?\\\]|\\\([\s\S]+?\\\)|\$(?!\$)(?:\\.|[^$\\\n])+\$)"

    Dim matches As Object
    Set matches = reg.Execute(txt)

    If matches.Count > 0 Then
        ' Batch convert all matches in selection
        Dim i As Long
        Dim startOffset As Long
        startOffset = sel.Range.Start

        ' Process backwards so character positions dont shift
        For i = matches.Count - 1 To 0 Step -1
            Dim m As Object
            Set m = matches(i)
            Dim fullMatch As String
            fullMatch = m.Value

            Dim rawLatex As String
            rawLatex = StripDelimiters(fullMatch)

            Dim flatOpc As String
            flatOpc = CallBridge(rawLatex, "latex", "flat_opc_mathtype")

            If Len(flatOpc) > 0 Then
                Dim subRng As Range
                Set subRng = ActiveDocument.Range(startOffset + m.FirstIndex, startOffset + m.FirstIndex + m.Length)
                InsertFlatOpcInline subRng, flatOpc, False
            End If
        Next i
    Else
        ' No delimiters, assume entire selected text is pure LaTeX formula
        Dim cleanLatex As String
        cleanLatex = Trim(Replace(Replace(txt, vbCr, ""), vbLf, ""))
        If Len(cleanLatex) > 0 Then
            Dim opc As String
            opc = CallBridge(cleanLatex, "latex", "flat_opc_mathtype")
            If Len(opc) > 0 Then
                InsertFlatOpcInline sel.Range.Duplicate, opc, True
            End If
        End If
    End If

    Exit Sub

ErrorHandler:
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
    On Error GoTo LaunchError
    CreateObject("WScript.Shell").Run "eqntide://open?tab=batch", 1, False
    Exit Sub
LaunchError:
    MsgBox "Please install EqnTide desktop and register its URI handler.", vbExclamation, "EqnTide"
End Sub

Private Sub EditInNativeDesktop(ByVal editExisting As Boolean)
    On Error GoTo EditError
    Dim target As Range
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
        http.send "{}"
    End If
    MsgBox "Cannot complete native editing: " & message, vbExclamation, "EqnTide"
End Sub

' =========================================================================
' Helper Functions
' =========================================================================

Private Sub InsertFlatOpcInline(ByVal target As Range, ByVal flatOpc As String, ByVal moveCursorAfter As Boolean)
    Dim doc As Document
    Set doc = ActiveDocument

    ' Never swallow an existing trailing paragraph mark if the selection ends with one.
    Do While target.End > target.Start
        Dim tailCh As String
        tailCh = doc.Range(target.End - 1, target.End).Text
        If tailCh <> vbCr And tailCh <> vbLf Then Exit Do
        target.End = target.End - 1
    Loop

    Dim insertPos As Long
    insertPos = target.Start
    Dim replaced As Long
    replaced = target.End - insertPos
    Dim originalEnd As Long
    originalEnd = doc.Content.End
    Dim suffixLen As Long
    suffixLen = 32
    If target.End + suffixLen > originalEnd Then suffixLen = originalEnd - target.End
    Dim suffix As String
    If suffixLen > 0 Then
        suffix = doc.Range(target.End, target.End + suffixLen).Text
    Else
        suffix = ""
    End If

    target.InsertXML flatOpc

    Dim newEnd As Long
    newEnd = doc.Content.End
    Dim insertedEnd As Long
    insertedEnd = insertPos + newEnd - originalEnd + replaced

    If insertedEnd > insertPos And insertedEnd <= newEnd Then
        Dim verifyLen As Long
        verifyLen = Len(suffix)
        Dim actualSuffix As String
        If verifyLen > 0 And insertedEnd + verifyLen <= newEnd Then
            actualSuffix = doc.Range(insertedEnd, insertedEnd + verifyLen).Text
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
                ch = doc.Range(insertedEnd - 1, insertedEnd).Text
                If ch <> vbCr And ch <> vbLf Then Exit For
                doc.Range(insertedEnd - 1, insertedEnd).Delete
                insertedEnd = insertedEnd - 1
            Next breaks
        End If
    End If

    If moveCursorAfter And insertedEnd >= insertPos Then
        Dim afterRng As Range
        Set afterRng = doc.Range(insertedEnd, insertedEnd)
        afterRng.Select
        On Error Resume Next
        Selection.Collapse wdCollapseEnd
        Selection.Font.Position = 0
        Selection.Font.Subscript = False
        Selection.Font.Superscript = False
        On Error GoTo 0
    End If
End Sub

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

Private Function CallBridge(ByVal dataStr As String, ByVal fromFmt As String, ByVal toFmt As String) As String
    On Error GoTo HttpError

    Dim http As Object
    Set http = CreateObject("MSXML2.ServerXMLHTTP.6.0")

    http.Open "POST", BRIDGE_URL, False
    http.setRequestHeader "Content-Type", "application/json"

    Dim payload As String
    payload = "{""data"":" & JsonEscape(dataStr) & ",""from"":""" & fromFmt & """,""to"":""" & toFmt & """}"

    http.send payload

    If http.Status = 200 Then
        Dim resp As String
        resp = http.responseText
        ' Simple JSON extract of 'result' field
        CallBridge = ExtractJsonResult(resp)
    Else
        MsgBox "Bridge Server returned HTTP " & http.Status & ": " & http.responseText, vbExclamation, "EqnTide Bridge"
        CallBridge = ""
    End If
    Exit Function

HttpError:
    MsgBox "Cannot connect to Bridge Server at " & BRIDGE_URL & vbCrLf & _
           "Please ensure eqntide-desktop.exe (or start-server.bat) is running!", vbCritical, "EqnTide"
    CallBridge = ""
End Function

Private Function JsonEscape(ByVal s As String) As String
    Dim res As String
    res = Replace(s, "\", "\\")
    res = Replace(res, """", "\""")
    res = Replace(res, vbCrLf, "\n")
    res = Replace(res, vbCr, "\n")
    res = Replace(res, vbLf, "\n")
    res = Replace(res, vbTab, "\t")
    JsonEscape = """" & res & """"
End Function

Private Function ExtractJsonResult(ByVal json As String) As String
    ExtractJsonResult = ExtractJsonField(json, "result")
End Function

Private Function ExtractJsonField(ByVal json As String, ByVal field As String) As String
    Dim pos As Long, i As Long, value As String, ch As String
    pos = InStr(json, """" & field & """")
    If pos = 0 Then Exit Function
    pos = InStr(pos + Len(field) + 2, json, ":")
    If pos = 0 Then Exit Function
    pos = pos + 1
    Do While pos <= Len(json) And (Mid(json, pos, 1) = " " Or Mid(json, pos, 1) = vbTab)
        pos = pos + 1
    Loop
    If Mid(json, pos, 1) <> """" Then Exit Function
    i = pos + 1
    Do While i <= Len(json)
        ch = Mid(json, i, 1)
        If ch = """" Then
            ExtractJsonField = value
            Exit Function
        ElseIf ch = "\" Then
            i = i + 1
            ch = Mid(json, i, 1)
            Select Case ch
                Case "n": value = value & vbLf
                Case "r": value = value & vbCr
                Case "t": value = value & vbTab
                Case "u"
                    value = value & ChrW(CLng("&H" & Mid(json, i + 1, 4)))
                    i = i + 4
                Case Else: value = value & ch
            End Select
        Else
            value = value & ch
        End If
        i = i + 1
    Loop
End Function
