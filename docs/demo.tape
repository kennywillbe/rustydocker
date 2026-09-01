# docs/demo.gif is generated from this file:
#
#   cargo build --release
#   PATH="$PWD/target/release:$PATH" vhs docs/demo.tape
#
#   # a scrolling log pane costs ~1.7MB raw; the palette pass halves it and the
#   # terminal has nowhere near 32 colours to lose
#   ffmpeg -i docs/demo.gif -filter_complex \
#     "[0:v]split[a][b];[a]palettegen=max_colors=32:stats_mode=diff[p];\
#      [b][p]paletteuse=dither=none:diff_mode=rectangle" -y /tmp/demo.gif
#   mv /tmp/demo.gif docs/demo.gif
#
# The release binary has to be on PATH before vhs runs — the tape types plain
# `rustydocker`, the same thing a viewer would type in a Compose directory.
#
# DOCKER_HOST points at the throwaway daemon that docs/demo-setup.sh brings up.
# The sidebar lists every container, image, volume and network on whatever
# daemon it is attached to, so recording against a real machine would leak that
# machine's containers into the GIF. The hidden setup below is idempotent and
# the hidden teardown removes the daemon again, so a run leaves nothing behind.

Output docs/demo.gif

Set Shell bash
Set FontSize 16
Set Width 1200
Set Height 700
Set Padding 15
Set TypingSpeed 45ms

# The ember palette, so the recording matches docs/screenshot.png instead of
# vhs's default (blue-grey) terminal theme. The TUI hardcodes its own RGB via
# src/ui/theme.rs, so what the terminal palette actually decides here is the
# background behind every unpainted cell and the default text of the shell
# lines around the app — those two are the ones that have to be exact:
#   background  #161310  the screenshot's terminal background
#   foreground  #d7cbac  Theme::EMBER.fg          Rgb(215, 203, 172)
# The 16 ANSI slots barely show (the app paints over them) but a mismatched
# one leaks through the shell prompt, so they are mapped onto the same
# palette: red/green/yellow from err/ok/warn, the blue-magenta-cyan trio onto
# the warm accents rather than left cold, brights a lightened step up.
Set Theme { "name": "rustydocker ember", "background": "#161310", "foreground": "#d7cbac", "selection": "#3a2e22", "cursor": "#c6501a", "cursorAccent": "#161310", "black": "#1a130c", "red": "#c24a2e", "green": "#8aa84a", "yellow": "#c6a14a", "blue": "#7a6f57", "magenta": "#b23020", "cyan": "#c6501a", "white": "#d7cbac", "brightBlack": "#3a2e22", "brightRed": "#d4603f", "brightGreen": "#a3c05e", "brightYellow": "#e4b363", "brightBlue": "#9c8f74", "brightMagenta": "#cf4a33", "brightCyan": "#e0733c", "brightWhite": "#f0e5c6" }
# A full-screen TUI whose log pane scrolls every tick makes a heavy GIF;
# 24fps keeps it smooth at roughly half the frames.
Set Framerate 24

Env DOCKER_HOST "tcp://127.0.0.1:12375"
# Keep the release-check modal off the recording; it is network-dependent and
# would pop over the UI at an arbitrary moment.
Env RUSTYDOCKER_NO_UPDATE_CHECK "1"

Hide
# The setup script blocks until all five services report healthy, so the
# visible take starts on a settled project. A cold start has to pull images and
# a warm one returns at once, so wait on the script's own "ready" line instead
# of guessing a duration — overrun a fixed Sleep and every later keystroke ends
# up in the shell's typeahead buffer instead of the app. </dev/null everywhere
# for the same reason: anything that parks on stdin in vhs's pty swallows the
# rest of the tape.
Type "REPO=$PWD"
Enter
Type "bash docs/demo-setup.sh up </dev/null 2>&1 | tail -1"
Enter
Wait+Screen@5m /ready: rustydocker-demo/
Type "cd /tmp/rustydocker-demo"
Enter
Type "clear"
Enter
Show

Sleep 1s

# --- a Compose project, not a flat list of containers ----------------------

Type "rustydocker"
Sleep 600ms
Enter
Sleep 4s

# --- real-time logs, and a regex over them ---------------------------------

# Services are listed alphabetically, so `api` is already selected and its log
# stream is the busiest one. `/` only searches logs when the main panel has
# focus — on the sidebar the same key is the fuzzy service filter — so move
# focus across with `l` first.
Type "l"
Sleep 1s

Type "/"
Sleep 500ms
Type "WARN"
Sleep 1.5s
Enter
Sleep 3s
Escape
Sleep 1s

# --- live CPU / memory / network charts ------------------------------------

Type "2"
Sleep 4.5s

# --- the environment the service is actually running with -------------------

Type "4"
Sleep 3.5s

# --- depends_on, drawn ------------------------------------------------------

Type "6"
Sleep 4.5s

Type "q"
Sleep 1.5s

Hide
# printf splits the sentinel so the echoed command line cannot satisfy the
# Wait on its own — otherwise the recording would stop before the scratch
# daemon is actually gone.
Type `cd "$REPO" && bash docs/demo-setup.sh down </dev/null; printf 'TEAR%s\n' DOWNOK`
Enter
Wait+Screen@2m /TEARDOWNOK/
Show
