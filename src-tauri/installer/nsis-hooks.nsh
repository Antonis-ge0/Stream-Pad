!macro NSIS_HOOK_POSTINSTALL
  SetShellVarContext current
  Delete "$INSTDIR\StreamPad.exe"
  Delete "$DESKTOP\Stream Pad.lnk"
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
  Delete "$SMPROGRAMS\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Deck.lnk"
  Delete "$SMPROGRAMS\Stream Pad\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Pad\Stream Deck.lnk"
  CreateShortCut "$DESKTOP\Stream Pad.lnk" "$INSTDIR\Stream Pad.exe" "" "$INSTDIR\Stream Pad.exe" 0

  SetShellVarContext all
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  SetShellVarContext current
  Delete "$DESKTOP\Stream Pad.lnk"
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
  Delete "$SMPROGRAMS\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Deck.lnk"
  Delete "$SMPROGRAMS\Stream Pad\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Pad\Stream Deck.lnk"

  SetShellVarContext all
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
!macroend
