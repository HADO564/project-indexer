# The demo GIF at the top of the CLI's READMEs. To record it again, from the
# repository root, with VHS (https://github.com/charmbracelet/vhs) and dexily
# installed:
#
#     vhs docs/cli/demo/demo.tape
#
# setup.sh fills a throwaway home with made-up projects first.

Output docs/cli/demo/dexily.gif

Require dexily
Require git

Set Shell "bash"
Set FontSize 15
Set Width 1100
Set Height 600
Set Padding 16
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms

Hide
Type "source docs/cli/demo/setup.sh && clear" Enter
Sleep 2s
Show

# Every project, as a table.
Type "dexily list" Sleep 300ms Enter
Sleep 3s

# `.`: the project you are in, from anywhere inside it.
Type "cd work/acme/api/src && dexily show ." Sleep 300ms Enter
Sleep 3.5s

# The TUI: move, search, see the details.
Type "clear && dexily" Sleep 300ms Enter
Sleep 1.5s
# It opens on the views; `Tab` jumps to the project list.
Tab Sleep 1.2s
Type "j" Sleep 1.2s
Type "j" Sleep 1.2s
Type "j" Sleep 1.2s
Type "/" Sleep 400ms
Type "acme" Sleep 900ms
Enter Sleep 900ms
Type "j" Sleep 1.5s
Type "k" Sleep 1.5s
Type "q"
Sleep 500ms
