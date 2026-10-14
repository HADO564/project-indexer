# Supported commands and programs

dexily's observer (`dexily <command>`) runs a command, then records the project
it created. It runs **only** the commands listed here — an allowlist, so a
script or an agent given dexily can never use it to run arbitrary programs.
Separately, dexily **detects** a project's type from the files on disk, for any
folder it tracks. This page is the full list of both.

For how the observer *behaves* (the prompt, the project flags, the override,
exit codes, `--json`), see the observer section of the
[CLI README](../../crates/cli/README.md#observing); for the `--json` contract,
see [`agents.md`](agents.md).

## Recognized commands — the observer runs these

Each command runs exactly as typed, then dexily tracks the folder it leaves a
project in. Anything else is refused before it starts.

| Command | Tracks | Proof |
|---|---|---|
| `mkdir <folders…>` | each folder it actually creates | the new folder |
| `git init [dir]` | the working folder, or the one named (`-C` moves it) | a `.git` |
| `git clone <repo> [dir]` | the folder named, or the one git derives from the address | a `.git` |
| `cargo new <path>` | the new folder | the new folder |
| `cargo init [path]` | the folder named, or the working one | a `Cargo.toml` |
| `go mod init [module]` | the working folder | a `go.mod` |
| `gh repo create <name> --clone` | the folder gh clones the new repo into | a `.git` |
| `npm init` · `pnpm init` · `yarn init` | the working folder | a `package.json` |
| `poetry init` | the working folder | a `pyproject.toml` |
| `uv init [path]` | the folder named, or the working one | a `pyproject.toml` |
| `flutter create <dir>` | the folder named | a `pubspec.yaml` |
| `deno init [dir]` | the folder named, or the working one | a `deno.json` |
| `zig init` | the working folder | a `build.zig` |

### The rules every one of them follows

- **Allowlist.** Only the commands above run; even a path to a program that
  merely shares a name (`/bin/mkdir`) is refused.
- **Fail-closed options.** For `git` and the scaffolders, only the options
  dexily models are passed on — each spelled in full, one at a time. Anything
  else (an abbreviation, clustered short options, an option dexily does not know
  or one added later) is refused before the command starts, so no option can
  turn the command into a way to run another program.
- **No downloaded code.** The JS package managers are allowed only at their
  **bare** `init`: `npm init <initializer>` / `yarn init <pkg>` runs a downloaded
  package (`npm init vite` ≡ `npx create-vite`), so a line with a positional
  initializer is refused. `uv init`, `flutter create` and `deno init` take a
  *folder* positional (a safe local scaffold), not a package. `npm create`,
  `pnpm dlx`, `npx` and the like are not recognized at all.
- **The override, for a person at a terminal only.** When dexily refuses a
  command, someone who typed it can answer `run it anyway? [y/N]` to run it
  regardless — never a pipe, a script, a redirect or `--json`. A refused but
  *recognized* git line still tracks the repo it makes; anything else tracks
  nothing.
- **No shell.** Arguments are handed to the command as a list, never joined into
  a shell line; on Windows, `mkdir`/`md` run through `cmd.exe` with its special
  characters refused before anything runs.

## Detected project types — trackers

For **any** folder dexily tracks — however it got there (`add`, `scan`, a clone,
a `mkdir`, or the desktop app) — its detectors read the files on disk and attach
a tracker for each kind they find. Detection is independent of the commands
above: a Godot project made in the editor is recognised just the same.

| Kind | Detected by |
|---|---|
| `git` | a git repository (`.git`) |
| `unreal` | a `*.uproject` |
| `node` | `package.json` |
| `python` | `pyproject.toml`, `setup.py` or `requirements.txt` |
| `go` | `go.mod` |
| `godot` | `project.godot` |
| `unity` | `ProjectSettings/ProjectVersion.txt` |
| `rust` | `Cargo.toml` |

A folder can carry several kinds at once — a cloned Unreal project comes out with
both `git` and `unreal`. Each kind's fields under `--json` are in
[`agents.md`](agents.md#trackers).

---

**Not recognized, on purpose.** Any command that runs or downloads third-party
code as part of scaffolding — `npm create` / `yarn create` / `npx` / `pnpm dlx` /
`bunx`, `rails new` (which runs `bundle install`), and installers like
`pip install` or `cargo install`. Recognizing them would let dexily be used to
run arbitrary programs, which the allowlist exists to prevent. `dotnet new` is
left out for a related reason — its templates carry open-ended options that
cannot be vetted fail-closed — but .NET projects are still recognised by the
`*.csproj` detector once tracked.
