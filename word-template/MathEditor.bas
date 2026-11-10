Attribute VB_Name = "MathEditor"
Option Explicit

Private Const BRIDGE_URL As String = "http://127.0.0.1:17890/convert"

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

    ' Case 2: Selected text contains LaTeX or plain text selected
    Dim txt As String
    txt = sel.Text

    If Len(Trim(txt)) = 0 Then
        ' Expand selection to current paragraph or word if nothing selected
        MsgBox "Please select a LaTeX expression (e.g. $x+1=2$) or an equation.", vbInformation, "Math Editor"
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
            flatOpc = CallBridge(rawLatex, "latex", "flat_opc_omml")

            If Len(flatOpc) > 0 Then
                Dim subRng As Range
                Set subRng = ActiveDocument.Range(startOffset + m.FirstIndex, startOffset + m.FirstIndex + m.Length)
                subRng.InsertXML flatOpc
            End If
        Next i
    Else
        ' No delimiters, assume entire selected text is pure LaTeX formula
        Dim cleanLatex As String
        cleanLatex = Trim(txt)
        If Len(cleanLatex) > 0 Then
            Dim opc As String
            opc = CallBridge(cleanLatex, "latex", "flat_opc_omml")
            If Len(opc) > 0 Then
                sel.Range.InsertXML opc
            End If
        End If
    End If

    Exit Sub

ErrorHandler:
    MsgBox "Error in Toggle LaTeX: " & Err.Description, vbExclamation, "Math Editor"
End Sub

' =========================================================================
' Insert / Edit Equations
' =========================================================================

Public Sub InsertEquationMacro()
    On Error Resume Next
    Dim wsh As Object
    Set wsh = CreateObject("WScript.Shell")
    ' Open local desktop editor or bridge session
    wsh.Run "http://127.0.0.1:17890/word-addin/taskpane.html", 1, False
End Sub

Public Sub EditEquationMacro()
    ToggleLatexMacro
End Sub

Public Sub UpgradeMathTypeMacro()
    MsgBox "Upgrading MathType equations in active document...", vbInformation, "Math Editor"
End Sub

' =========================================================================
' Helper Functions
' =========================================================================

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
        MsgBox "Bridge Server returned HTTP " & http.Status & ": " & http.responseText, vbExclamation, "Math Editor Bridge"
        CallBridge = ""
    End If
    Exit Function

HttpError:
    MsgBox "Cannot connect to Bridge Server at " & BRIDGE_URL & vbCrLf & _
           "Please ensure omml-editor-desktop.exe (or start-server.bat) is running!", vbCritical, "Math Editor"
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
    Dim pos As Long
    pos = InStr(json, """result"":""")
    If pos > 0 Then
        pos = pos + 10
        Dim endPos As Long
        Dim inEscape As Boolean
        inEscape = False
        Dim i As Long
        For i = pos To Len(json)
            Dim c As String
            c = Mid(json, i, 1)
            If inEscape Then
                inEscape = False
            ElseIf c = "\" Then
                inEscape = True
            ElseIf c = """" Then
                endPos = i
                Exit For
            End If
        Next i

        If endPos > pos Then
            Dim rawVal As String
            rawVal = Mid(json, pos, endPos - pos)
            rawVal = Replace(rawVal, "\""", """")
            rawVal = Replace(rawVal, "\\", "\")
            rawVal = Replace(rawVal, "\n", vbCrLf)
            rawVal = Replace(rawVal, "\r", "")
            rawVal = Replace(rawVal, "\t", vbTab)
            ExtractJsonResult = rawVal
            Exit Function
        End If
    End If
    ExtractJsonResult = ""
End Function
