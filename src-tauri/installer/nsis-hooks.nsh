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
  ${If} $UpdateMode <> 1
    SetFileAttributes "$INSTDIR\Uninstall Stream Pad.lnk" NORMAL
    SetFileAttributes "$INSTDIR\stream-pad-maintenance.exe" NORMAL
    SetFileAttributes "$INSTDIR\uninstall.exe" NORMAL
    Delete "$INSTDIR\Uninstall Stream Pad.lnk"
    Delete "$INSTDIR\stream-pad-maintenance.exe"
    Delete "$INSTDIR\uninstall.exe"
    RMDir /r "$INSTDIR"

    StrCpy $0 "$TEMP\stream-pad-cleanup.cmd"
    FileOpen $1 "$0" w
    FileWrite $1 "@echo off$\r$\n"
    FileWrite $1 "ping -n 4 127.0.0.1 >nul$\r$\n"
    FileWrite $1 "attrib -R -S -H $\"$INSTDIR\*$\" /S /D >nul 2>nul$\r$\n"
    FileWrite $1 "rmdir /S /Q $\"$INSTDIR$\" >nul 2>nul$\r$\n"
    FileWrite $1 "del $\"%~f0$\" >nul 2>nul$\r$\n"
    FileClose $1
    ExecShell "open" "$SYSDIR\cmd.exe" "/C $\"$0$\"" SW_HIDE

    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty SHCTX "${MANUKEY}"
    DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"
    DeleteRegKey /ifempty HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    SetShellVarContext current
    RMDir /r "$APPDATA\${BUNDLEID}"
    RMDir /r "$LOCALAPPDATA\${BUNDLEID}"
  ${EndIf}

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
