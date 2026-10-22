; Ao atualizar por cima de uma versão anterior, o caminho do .exe não muda e
; o Explorer continua mostrando o ícone antigo que está no cache. Avisa o shell
; para descartar o cache e redesenhar atalhos, menu Iniciar e barra de tarefas.
;
; Também apaga a marca que a 0.6.x deixava quando o usuário recusava o driver
; PawnIO, que o Silício não usa mais.
!macro NSIS_HOOK_POSTINSTALL
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
  nsExec::Exec '"$SYSDIR\ie4uinit.exe" -show'
  DeleteRegValue HKCU "Software\Silicio" "PawnIODeclined"
  DeleteRegKey /ifempty HKCU "Software\Silicio"
!macroend
