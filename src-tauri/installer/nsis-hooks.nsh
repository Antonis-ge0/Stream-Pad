!macro NSIS_HOOK_POSTINSTALL
  Delete "$INSTDIR\StreamPad.exe"
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
  Delete "$SMPROGRAMS\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Deck.lnk"
  Delete "$SMPROGRAMS\Stream Pad\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Pad\Stream Deck.lnk"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
  Delete "$SMPROGRAMS\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Deck.lnk"
  Delete "$SMPROGRAMS\Stream Pad\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Pad\Stream Deck.lnk"
!macroend
