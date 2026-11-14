Attribute VB_Name = "EqnTideEvents"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = False
Attribute VB_Exposed = False
Option Explicit

Public WithEvents App As Word.Application

Private Sub App_WindowSelectionChange(ByVal Sel As Word.Selection)
    EqnTide.HandleEquationSelection Sel
End Sub
