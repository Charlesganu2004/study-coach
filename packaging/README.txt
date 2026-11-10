STUDY COACH

Windows: extract this archive, then double click StartStudyCoach.cmd.
To install for your account, open PowerShell in this folder and run:
  .\install.ps1
To install without a Start menu shortcut:
  .\install.ps1 -NoShortcut
To remove installed program files, close Study Coach and run the installed:
  .\uninstall.ps1

Linux or macOS native package: extract the archive, then run ./start.sh.
Optional installation for your account:
  sh ./install.sh
Removal of installed program files:
  sh ./uninstall.sh

The browser interface is served on 127.0.0.1 only. Keep the launcher running
while using the application. Click the X labelled Close Study Coach at the top
right to stop the local service. A current answer finishes before shutdown.
If the browser keeps the closed page open, press Ctrl+W to close that tab.
Reopen StartStudyCoach.cmd or the Start menu shortcut to return to saved data.
If shutdown cannot be confirmed, press Ctrl+C in the launcher terminal.
No Rust installation is required to run a prebuilt package. curl must be on
PATH. Windows includes curl.exe; Linux and macOS users can check curl --version.

First launch creates a local application data folder and a blank .env file.
Windows: %APPDATA%\StudyCoach
Linux: ${XDG_DATA_HOME:-$HOME/.local/share}/study-coach
macOS: $HOME/Library/Application Support/StudyCoach
An explicit --data-dir overrides the location. The interface Connections page
accepts your keys locally; they are never included in this package. Removal of
the application preserves this data folder.

Groq key: https://console.groq.com/keys
Selected model: openai/gpt-oss-120b
NVIDIA key and free endpoint:
https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b
Free access depends on each provider's account eligibility and quotas. It is
not unlimited. The application does not enable a paid fallback.

SHA256SUMS detects file corruption. It does not authenticate a publisher.
This package adds no license; the repository owner chooses distribution terms.
