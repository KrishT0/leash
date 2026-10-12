; The installer hooks for Leash.
;
; The installer registers a scheduled task, so Leash starts at logon with the
; rights it needs and without a prompt. The uninstaller removes the task.
;
; The task XML is written here instead of being shipped as a file, because the
; command inside it has to name the installed program, and the install folder is
; not known until the user picks one. installer/leash-task.xml holds the same
; XML in a readable form, with placeholders where the paths go.
;
; The copy is written as UTF-16 with a byte order mark, and carries no XML
; declaration. Task Scheduler refuses a UTF-8 byte order mark, and it refuses a
; declaration that does not match the bytes of the file, so these two details are
; the ones it accepts. installer/leash-task.xml explains the same rule.
;
; A failure to register the task is logged and does not stop the install. Leash
; still runs, and it can be started by hand from an elevated shell.

!define LEASH_TASK_NAME "Leash"
!define LEASH_TASK_XML "$INSTDIR\leash-task.xml"

!macro LEASH_WRITE_TASK_XML
  FileOpen $0 "${LEASH_TASK_XML}" w
  FileWriteUTF16LE /BOM $0 '<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">$\r$\n'
  FileWriteUTF16LE $0 '  <RegistrationInfo>$\r$\n'
  FileWriteUTF16LE $0 '    <Description>Runs Leash at logon with the rights it needs to stop services.</Description>$\r$\n'
  FileWriteUTF16LE $0 '  </RegistrationInfo>$\r$\n'
  FileWriteUTF16LE $0 '  <Triggers>$\r$\n'
  FileWriteUTF16LE $0 '    <LogonTrigger><Enabled>true</Enabled></LogonTrigger>$\r$\n'
  FileWriteUTF16LE $0 '  </Triggers>$\r$\n'
  FileWriteUTF16LE $0 '  <Principals>$\r$\n'
  FileWriteUTF16LE $0 '    <Principal id="Author">$\r$\n'
  FileWriteUTF16LE $0 '      <LogonType>InteractiveToken</LogonType>$\r$\n'
  FileWriteUTF16LE $0 '      <RunLevel>HighestAvailable</RunLevel>$\r$\n'
  FileWriteUTF16LE $0 '    </Principal>$\r$\n'
  FileWriteUTF16LE $0 '  </Principals>$\r$\n'
  FileWriteUTF16LE $0 '  <Settings>$\r$\n'
  FileWriteUTF16LE $0 '    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>$\r$\n'
  FileWriteUTF16LE $0 '    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>$\r$\n'
  FileWriteUTF16LE $0 '    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>$\r$\n'
  FileWriteUTF16LE $0 '    <AllowStartOnDemand>true</AllowStartOnDemand>$\r$\n'
  FileWriteUTF16LE $0 '    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>$\r$\n'
  FileWriteUTF16LE $0 '    <Enabled>true</Enabled>$\r$\n'
  FileWriteUTF16LE $0 '  </Settings>$\r$\n'
  FileWriteUTF16LE $0 '  <Actions Context="Author">$\r$\n'
  FileWriteUTF16LE $0 '    <Exec>$\r$\n'
  FileWriteUTF16LE $0 '      <Command>$INSTDIR\${MAINBINARYNAME}.exe</Command>$\r$\n'
  FileWriteUTF16LE $0 '      <Arguments>--autostart</Arguments>$\r$\n'
  FileWriteUTF16LE $0 '      <WorkingDirectory>$INSTDIR</WorkingDirectory>$\r$\n'
  FileWriteUTF16LE $0 '    </Exec>$\r$\n'
  FileWriteUTF16LE $0 '  </Actions>$\r$\n'
  FileWriteUTF16LE $0 '</Task>$\r$\n'
  FileClose $0
!macroend

; Ends Leash before anything tries to replace or delete it.
;
; Both the installer and the uninstaller check for a running Leash, by product
; name, and try to close it. Leash traps a window close and hides instead of
; exiting, which is deliberate, so their close would never end it and the check
; would keep failing.
;
; Ending the process is safe: the config is written as settings change, and the
; activity log line by line, so nothing is waiting in memory to be lost.
!macro LEASH_STOP_RUNNING
  nsExec::ExecToLog 'taskkill /IM ${MAINBINARYNAME}.exe /F'
  Pop $0
  Sleep 500
!macroend

; Runs before the installer copies anything over the running program.
!macro NSIS_HOOK_PREINSTALL
  !insertmacro LEASH_STOP_RUNNING
!macroend

; Runs after the program and the shortcuts are in place.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro LEASH_WRITE_TASK_XML

  DetailPrint "Registering the Leash scheduled task"
  nsExec::ExecToLog 'schtasks /Create /TN "${LEASH_TASK_NAME}" /XML "${LEASH_TASK_XML}" /F'
  Pop $0
  ${If} $0 != 0
    DetailPrint "Leash: the scheduled task was not registered (schtasks returned $0)."
    DetailPrint "Leash: start the program from an elevated shell instead, or import ${LEASH_TASK_XML} yourself."
  ${EndIf}
!macroend

; Runs first in the uninstall section, while the task still points at a program
; that exists.
!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro LEASH_STOP_RUNNING

  DetailPrint "Removing the Leash scheduled task"
  nsExec::ExecToLog 'schtasks /Delete /TN "${LEASH_TASK_NAME}" /F'
  Pop $0
  Delete "${LEASH_TASK_XML}"
!macroend
