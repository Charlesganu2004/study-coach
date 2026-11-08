# Install Study Coach

## Windows package

Extract the Windows ZIP into a folder. Double click `StartStudyCoach.cmd` to run a portable copy. A prebuilt package needs no Rust installation. Windows `curl.exe` must be available on `PATH`.

To install for your Windows account, open PowerShell in the extracted folder:

```powershell
.\install.ps1
```

The default destination is `%LOCALAPPDATA%\Programs\StudyCoach`. The installer creates a Start menu shortcut and requires no administrator account. It verifies every packaged file against `SHA256SUMS` before copying files. The checksums detect corruption; they do not authenticate the publisher. If Windows or your organization blocks an unsigned script, follow your organization's script policy or use the portable launcher. The package does not change execution policy.

An optional destination and shortcut choice are available:

```powershell
.\install.ps1 -InstallDir 'C:\Users\YourName\Apps\StudyCoach' -NoShortcut
```

Close the launcher terminal to stop the local service. Close Study Coach before replacing an installed binary during an upgrade. Rerun the new package installer with the same destination to update the program files.

## Keys and user data

First launch creates a separate application data folder and a blank `.env` file. The browser setup accepts API keys locally, and the program never includes developer keys in a package. The default model is Groq `openai/gpt-oss-120b`. NVIDIA `nvidia/nemotron-3.5-lightning-30b-a3b` is an alternative.

* Windows: `%APPDATA%\StudyCoach`
* Linux: `${XDG_DATA_HOME:-$HOME/.local/share}/study-coach`
* macOS: `$HOME/Library/Application Support/StudyCoach`

An explicit `--data-dir` overrides the location. Notes, history, settings, and keys remain in this folder when program files are removed. Back it up privately if needed; do not upload it to GitHub.

Create your own key at [Groq Console](https://console.groq.com/keys) or the [NVIDIA model page](https://build.nvidia.com/nvidia/nemotron-3.5-lightning-30b-a3b). Free access depends on account eligibility and quotas. It is not unlimited and requires an internet connection. No paid fallback is enabled. Provider requests include the question and the notes supplied to the model.

## Linux and macOS

The source supports native packaging on Linux and macOS. A package built for another operating system cannot run on yours. Native archives contain a compiled binary, `start.sh`, an installer, and an uninstaller. With a matching archive extracted:

```sh
sh ./start.sh
sh ./install.sh
```

The installer defaults to `$HOME/.local/lib/study-coach` and adds a command link at `$HOME/.local/bin/study-coach`. To install elsewhere without changing that command link:

```sh
sh ./install.sh --prefix "$HOME/Apps/StudyCoach" --no-link
```

`curl`, plus `sha256sum` or `shasum` for package verification, must be installed. Linux and macOS artifacts must be built and tested on their native platforms before release; the Windows verification receipt does not establish that they work.

## Build from source

Install Rust 1.89 or newer and `curl`, then work inside the repository:

```sh
cargo install --path . --locked --bin study-coach-app
study-coach-app
```

To create a Windows package using an existing working Rust toolchain:

```powershell
.\scripts\package.ps1
```

On Linux or macOS:

```sh
sh ./scripts/package.sh
```

Build scripts use the locked, locally cached dependencies and make no dependency changes. A fresh checkout may first need `cargo fetch --locked`. Build output goes into ignored `dist/`. ZIP or archive files contain only the compiled application and the named launcher, installer, uninstaller, instructions, and checksum list. User data, `.env`, evaluation ledgers, build caches, and source files are excluded.

## Remove the application

Close the running service, open the installation folder, then run:

```powershell
.\uninstall.ps1
```

On Linux or macOS:

```sh
sh ./uninstall.sh
```

Removal deletes only recognized application files and its own shortcut or command link. It preserves the application data folder. Portable copies have no installation marker and can be removed by deleting their extracted program folder. The package adds no license; the repository owner must choose distribution terms before a public release.
