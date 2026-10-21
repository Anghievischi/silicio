; Ao atualizar por cima de uma versão anterior, o caminho do .exe não muda e
; o Explorer continua mostrando o ícone antigo que está no cache. Avisa o shell
; para descartar o cache e redesenhar atalhos, menu Iniciar e barra de tarefas.
;
; Depois oferece instalar o driver PawnIO (https://pawnio.eu), que o Silício usa
; para ler temperatura e clock máximo do processador. O instalador oficial dele
; (PawnIO_setup.exe 2.2.0, redistribuível sem modificação) vai embutido aqui e
; pede o UAC por conta própria. A pergunta não aparece se o driver já está
; instalado, em instalação silenciosa, ou numa atualização automática quando o
; usuário já recusou antes.
!define SILICIO_PAWNIO_KEY "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO"
!define SILICIO_KEY "Software\Silicio"
; Dentro da macro, __FILEDIR__ seria a pasta do script que a expande. O
; caminho usa "\": o File do NSIS no Windows separa a pasta pela última
; barra invertida e não acha o arquivo com "/".
!define SILICIO_HOOKS_DIR "${__FILEDIR__}"

!macro NSIS_HOOK_POSTINSTALL
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
  nsExec::Exec '"$SYSDIR\ie4uinit.exe" -show'

  IfSilent silicio_pawnio_done
  ${IfNot} ${IsNativeAMD64}
    Goto silicio_pawnio_done
  ${EndIf}
  SetRegView 64
  ReadRegStr $0 HKLM "${SILICIO_PAWNIO_KEY}" "DisplayVersion"
  SetRegView lastused
  StrCmp $0 "" 0 silicio_pawnio_done
  ReadRegStr $0 HKCU "${SILICIO_KEY}" "PawnIODeclined"
  ${If} $0 == "1"
  ${AndIf} $PassiveMode = 1
    Goto silicio_pawnio_done
  ${EndIf}

  ; 1046 = português do Brasil.
  ${If} $LANGUAGE = 1046
    StrCpy $0 "Instalar o driver PawnIO?$\r$\n$\r$\nCom ele o Silício mostra a temperatura e o clock máximo do processador. É um driver assinado e de código aberto, usado também pelo LibreHardwareMonitor e outros programas de monitoramento. O Windows vai pedir permissão de administrador."
  ${Else}
    StrCpy $0 "Install the PawnIO driver?$\r$\n$\r$\nIt lets Silício show the processor temperature and max clock. It is a signed, open-source driver, also used by LibreHardwareMonitor and other monitoring tools. Windows will ask for administrator permission."
  ${EndIf}
  MessageBox MB_YESNO|MB_ICONQUESTION "$0" IDYES silicio_pawnio_install
  WriteRegStr HKCU "${SILICIO_KEY}" "PawnIODeclined" "1"
  Goto silicio_pawnio_done

  silicio_pawnio_install:
  DeleteRegValue HKCU "${SILICIO_KEY}" "PawnIODeclined"
  InitPluginsDir
  File "/oname=$PLUGINSDIR\PawnIO_setup.exe" "${SILICIO_HOOKS_DIR}\PawnIO_setup.exe"
  DetailPrint "PawnIO_setup.exe -install -silent"
  ; ShellExecute (e não CreateProcess) para o Windows mostrar o UAC: o
  ; instalador exige administrador e o Silício instala só para o usuário.
  ExecShellWait "open" "$PLUGINSDIR\PawnIO_setup.exe" "-install -silent" SW_HIDE
  Delete "$PLUGINSDIR\PawnIO_setup.exe"

  silicio_pawnio_done:
!macroend

; A recusa só vale enquanto o Silício está instalado. O PawnIO fica: outros
; programas podem estar usando o driver.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "${SILICIO_KEY}" "PawnIODeclined"
    DeleteRegKey /ifempty HKCU "${SILICIO_KEY}"
  ${EndIf}
!macroend
