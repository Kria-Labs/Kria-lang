; Kria-lang NSIS Installer
; Build with: makensis kria-setup.nsi
; Requires: NSIS 3.x (https://nsis.sourceforge.io)

!define PRODUCT_NAME "Kria"
!define PRODUCT_VERSION "1.0.0"
!define PRODUCT_PUBLISHER "Piotriox"
!define PRODUCT_EXE "kria.exe"
!define PRODUCT_REGKEY "Software\Kria"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "release\kria-${PRODUCT_VERSION}-windows-x86_64-setup.exe"
InstallDir "$LOCALAPPDATA\Kria"
InstallDirRegKey HKCU "${PRODUCT_REGKEY}" "InstallDir"
RequestExecutionLevel user

; Modern UI
!include "MUI2.nsh"
!define MUI_ICON "kria.ico"
!define MUI_UNICON "kria.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section "Kria" SecMain
    SectionIn RO

    SetOutPath $INSTDIR

    ; Copy binary
    File "target\release\${PRODUCT_EXE}"

    ; Copy docs
    File "README.md"
    File "LICENSE"
    File "test.krx"

    ; Write registry
    WriteRegStr HKCU "${PRODUCT_REGKEY}" "InstallDir" $INSTDIR
    WriteRegStr HKCU "${PRODUCT_REGKEY}" "Version" "${PRODUCT_VERSION}"

    ; Add to PATH
    EnVar::AddValue "PATH" $INSTDIR

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "Publisher" "${PRODUCT_PUBLISHER}"
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria" \
        "NoRepair" 1
SectionEnd

Section ".krx file association (Recommended)" SecAssoc
    ; Associate .krx files with Kria
    WriteRegStr HKCU "Software\Classes\.krx" "" "Kria.Script"
    WriteRegStr HKCU "Software\Classes\Kria.Script" "" "Kria Script"
    WriteRegStr HKCU "Software\Classes\Kria.Script\DefaultIcon" "" "$INSTDIR\${PRODUCT_EXE},0"
    WriteRegStr HKCU "Software\Classes\Kria.Script\shell\open\command" "" '"$INSTDIR\${PRODUCT_EXE}" "%1"'
SectionEnd

Section Uninstall
    ; Remove from PATH
    EnVar::DeleteValue "PATH" $INSTDIR

    ; Remove file association
    DeleteRegKey HKCU "Software\Classes\.krx"
    DeleteRegKey HKCU "Software\Classes\Kria.Script"

    ; Remove registry entries
    DeleteRegKey HKCU "${PRODUCT_REGKEY}"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kria"

    ; Remove files
    Delete "$INSTDIR\${PRODUCT_EXE}"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\test.krx"
    Delete "$INSTDIR\uninstall.exe"

    ; Remove directory
    RMDir "$INSTDIR"
SectionEnd

; Description text
LangString DESC_SecMain ${LANG_ENGLISH} "Install Kria programming language runtime."
LangString DESC_SecAssoc ${LANG_ENGLISH} "Associate .krx files with Kria so you can run scripts by double-clicking them. (Recommended)"

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} $(DESC_SecMain)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecAssoc} $(DESC_SecAssoc)
!insertmacro MUI_FUNCTION_DESCRIPTION_END
