!macro NSIS_HOOK_POSTINSTALL
  SetShellVarContext current
  Delete "$INSTDIR\StreamPad.exe"
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
  Delete "$SMPROGRAMS\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Deck.lnk"
  Delete "$SMPROGRAMS\Stream Pad\StreamPad.lnk"
  Delete "$SMPROGRAMS\Stream Pad\Stream Deck.lnk"

  IfFileExists "$INSTDIR\stream-pad-maintenance.exe" 0 stream_pad_maintenance_done
    WriteRegStr SHCTX "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\stream-pad-maintenance.exe$\" --uninstall $\"$INSTDIR\uninstall.exe$\""
    WriteRegStr SHCTX "${UNINSTKEY}" "QuietUninstallString" "$\"$INSTDIR\stream-pad-maintenance.exe$\" --uninstall-quiet $\"$INSTDIR\uninstall.exe$\""
    CreateShortCut "$INSTDIR\Uninstall Stream Pad.lnk" "$INSTDIR\stream-pad-maintenance.exe" "--uninstall $\"$INSTDIR\uninstall.exe$\"" "$INSTDIR\stream-pad-maintenance.exe" 0
    SetFileAttributes "$INSTDIR\uninstall.exe" HIDDEN
  stream_pad_maintenance_done:

  SetShellVarContext all
  Delete "$DESKTOP\StreamPad.lnk"
  Delete "$DESKTOP\Stream Deck.lnk"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  SetFileAttributes "$INSTDIR\uninstall.exe" NORMAL
  Delete "$INSTDIR\Uninstall Stream Pad.lnk"
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
