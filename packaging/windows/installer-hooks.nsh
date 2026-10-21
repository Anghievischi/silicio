; Ao atualizar por cima de uma versão anterior, o caminho do .exe não muda e
; o Explorer continua mostrando o ícone antigo que está no cache. Avisa o shell
; para descartar o cache e redesenhar atalhos, menu Iniciar e barra de tarefas.
!macro NSIS_HOOK_POSTINSTALL
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0x1000, p 0, p 0)'
  nsExec::Exec '"$SYSDIR\ie4uinit.exe" -show'
!macroend
