# A throwaway home holding a few made-up projects, for demo.tape: the GIF never
# shows anyone's real project list. Sourced by the tape, not run, since it sets
# HOME, the prompt and the folder for the rest of the recording.
#
# Uses whichever `dexily` is on PATH, so the GIF shows the installed release.

# A short fixed name rather than mktemp's random one, which would show in
# every path and wrap in the TUI's detail pane. Emptied first, so each
# recording starts from the same projects, but only when the marker says an
# earlier run made it: a /tmp/demo of anyone else's is left alone.
demo=/tmp/demo
if [ -e "$demo" ] && [ ! -e "$demo/.dexily-demo" ]; then
    echo "setup.sh: $demo exists and was not made by this script" >&2
    return 1
fi
rm -rf "$demo" && mkdir -p "$demo" && touch "$demo/.dexily-demo" || return
# Resolved (`pwd -P`), as dexily stores folders: on macOS /tmp is a link.
export HOME="$(cd "$demo" && pwd -P)"
export PS1='\[\e[38;5;245m\]\w\[\e[0m\] \$ '
cd "$HOME" || return

# Folders, each with whatever marks it as a project.
mkdir -p work/acme/api/src work/acme/web/src work/globex/app play/game notes
printf '[package]\nname = "api"\nversion = "0.1.0"\n' > work/acme/api/Cargo.toml
printf '{ "name": "web" }\n' > work/acme/web/package.json
printf 'module globex/app\n' > work/globex/app/go.mod
printf '[application]\nconfig/name="game"\n' > play/game/project.godot
for repo in work/acme/api work/acme/web work/globex/app play/game; do
    git -C "$repo" init -q -b main
    git -C "$repo" -c user.name=demo -c user.email=demo@example.com \
        commit -q --allow-empty -m "start"
done
echo "changed" > work/acme/web/src/app.ts

for project in work/acme/api work/acme/web work/globex/app play/game notes; do
    dexily add "$project" > /dev/null 2>&1
done
dexily edit api --description "Billing API for Acme" --add-tag client --color cyan > /dev/null 2>&1
dexily edit web --description "Acme's customer dashboard" --add-tag client --color violet > /dev/null 2>&1
dexily edit app --add-tag client --color gold > /dev/null 2>&1
dexily edit game --description "A weekend jam game" --color green > /dev/null 2>&1
dexily favorite api > /dev/null 2>&1
