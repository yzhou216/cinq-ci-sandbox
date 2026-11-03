cinq(5) "cinq" "File Formats Manual"

; SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
; SPDX-License-Identifier: GPL-3.0-or-later

; The lines after "; if" belong only to the pages of the platforms that it
; names, xdg, macos, or windows, until the next "; if" or "; end".

# NAME

cinq - configuration file for the keys of cinq

# SYNOPSIS

; if xdg
_$XDG_CONFIG_HOME/cinq/config.toml_
; if macos
_~/Library/Application Support/cinq/config.toml_
; if windows
_%APPDATA%\\cinq\\config.toml_
; end

# DESCRIPTION

*cinq* is a cue-based show controller in the spirit of QLab, and the keyboard
runs most of it.
It ships two keymaps, each of which binds keys to every action, and a
configuration file in TOML chooses one of them, or builds keymaps of its own,
and binds actions to other keys.
*cinq* reads the file when it starts and again whenever it changes, so that the
keys, and the help page that lists them, follow each change as soon as it is
saved.

The help page, which the key of *help* opens, names the keymap in use, lists
every action with its keys and its name in the file, links to the file that
the keys come from, or would come from, and ends with the notice of the
license of cinq.
Following the link opens the file in
; if xdg windows
the program that the desktop opens it with,
; if macos
the default text editor,
; end
and first makes the file when it is not there yet, holding only a line
that points to this page.

# FILES

; if xdg
_$XDG_CONFIG_HOME/cinq/config.toml_
	The configuration file, in the directory of the user's configuration,
	which is _~/.config_ when *XDG_CONFIG_HOME* is unset or is not an
	absolute path, as the XDG Base Directory Specification sets out.
; if macos
_~/Library/Application Support/cinq/config.toml_
	The configuration file, in the directory of the user's application
	support files.
; if windows
_%APPDATA%\\cinq\\config.toml_
	The configuration file, in the directory of the user's roaming
	application data, such as _C:\\Users\\Alice\\AppData\\Roaming_.
; end

Making or deleting the file applies while *cinq* runs as well, so that
deleting it brings back the keys of vi.

Started with *--config-dir*=_directory_, *cinq* reads _directory/config.toml_
in its place, and follows it as it changes in the same way.

# SETTINGS

*keymap*
	Name of the keymap that the keys start from: one that *cinq* ships, *vi*
	or *cua*, as *KEYMAPS* describes them, or one of the file's own under
	*keymaps*.
	It is *vi* unless the file names another.

*keymaps*
	Table of keymaps of the file's own, each of them a table named after its
	keymap.
	The *from* of such a keymap names the keymap that it starts from, a
	shipped one or another of the file's own, and without *from* it starts
	with no keys at all.
	Its other entries bind actions to keys as those of *keys* do.
	A keymap of the file's own cannot take the name of a shipped one.

*keys*
	Table that binds actions to keys in the keymap in use.
	Each entry is named after an action, as listed under *ACTIONS*, and holds
	a key sequence in a string, or several in an array of strings, any of
	which carries out the action.
	An action that the table names loses the keys that the keymap gives it,
	so that an empty array leaves it with no keys at all, and the actions
	that the table leaves out keep the keys of the keymap.

# KEYMAPS

*vi*
	Keys in the manner of vi: letters move through cues and lists and act on
	them, some commands take two keys, such as *d d*, and the arrows and
	Control with *n* and *p* move as well.

*cua*
	Keys of IBM's Common User Access, which most desktop programs follow: the
	arrows, Home, and End move, Control with a letter edits, and F1 shows the
	help page, which Control+w closes.

Both keep GO on the space bar and Panic on Escape, as in QLab.
The table lists the keys that each keymap binds to every action, in the order
of the help page.

|[ *Action*
:< *vi*
:< *cua*
|  go
:  Space
:  Space
|  panic
:  Escape
:  Escape
|  next
:  j, ArrowDown, Control+n
:  ArrowDown
|  previous
:  k, ArrowUp, Control+p
:  ArrowUp
|  next_list
:  l, ArrowRight
:  Control+PageDown
|  previous_list
:  h, ArrowLeft
:  Control+PageUp
|  first
:  g g
:  Home
|  last
:  G
:  End
|  next_tab
:  g t
:  Control+Tab
|  previous_tab
:  g T
:  Control+Shift+Tab
|  standby
:  Enter
:  Enter
|  edit
:  i
:  F2
|  next_field
:  Tab
:  Tab
|  previous_field
:  Shift+Tab
:  Shift+Tab
|  add_below
:  o
:  Control+n
|  add_above
:  O
:  Control+N
|  move_down
:  Alt+j, Alt+ArrowDown
:  Alt+ArrowDown
|  move_up
:  Alt+k, Alt+ArrowUp
:  Alt+ArrowUp
|  delete
:  d d
:  Delete
|  undo
:  u
:  Control+z
|  redo
:  Control+r
:  Control+y, Control+Z
|  open
:  Control+o
:  Control+o
|  save
:  Control+s
:  Control+s
|  save_as
:  Control+S
:  Control+S
|  help
:  ?
:  F1
|  close_help
:  q
:  Control+w

# KEYS

A key goes by its value in the W3C's UI Events KeyboardEvent key Values.

- A key that types a character is that character, as in *j*, *G*, or *?*.
  Letters tell the case apart, so that *G* is the key that Shift and *g* type
  together.
- Any other key goes by its name, as in *Enter*, *Escape*, *Tab*, *ArrowDown*,
  *PageUp*, or *F5*.
- The space bar is *Space*, since its value, the space it types, would read as
  the gap between two keys.

Control, Alt, and Shift can be held with a key, each joined to it by a *+*, as
in *Control+d*, *Alt+ArrowUp*, or *Control+Shift+Tab*.
A *+* with nothing after it is the plus key itself, as in *Control++*.
Shift with a letter is the capital that it types, so that *Shift+g* is *G* and
*Control+Shift+n* is *Control+N*.
Any other key that types a character is written as the character it types
with Shift held, as in *?* rather than *Shift+/*, and Shift cannot be added to
it.
Shift can be held with the space bar and with any key that goes by its name,
as in *Shift+Space* or *Shift+Tab*.

A modifier key, such as Shift or Control, cannot be bound by itself.
Keys held with the logo key belong to the system, and *cinq* leaves them
alone, so a key cannot hold Meta or Super.

A key sequence is one or more keys separated by spaces, which are typed one
after another, as in *g g*.

# TYPING

The keys of a sequence wait until the sequence is complete, and the status bar
shows the keys typed so far, while a box above it lists the keys that can come
next and what each of them does.
A key that begins no sequence is dropped, but a key that breaks off a sequence
typed halfway starts a new one, so that a stray key never swallows the next.
Anything done other than typing, such as a click, drops a sequence typed
halfway, and so does a change to the file.

Digits typed before a sequence count it, as in *3 j* with the vi keymap.
The count repeats *next*, *previous*, *next_list*, *previous_list*,
*move_down*, and *move_up*, deletes as many cues for *delete*, and gives the
row, counted from one, that *first* and *last* select.
Other actions leave the count unused.
Since digits count, no sequence can start with one.

A key bound by itself to *go* or *panic* acts at once, even halfway through
another sequence, and when it is bound without modifiers, whatever modifiers
are held with it, unless the other of the two is bound to the key with those
very modifiers.
Holding either key down acts only once, so that holding the key of Panic does
not cut a panic short, while every other key repeats as it is held.

# WHERE KEYS ACT

Keys act on the cue list, apart from the keys typed while one of these has
the keyboard:

A text field of the inspector, or a drop-down list
	A click gives a text field the keyboard, and the keys of *next_field* and
	*previous_field* move it from one text field or drop-down list to the
	next, which shows a border in the color of focus.
	It keeps the keys typed, apart from the keys bound by themselves to *go*,
	*panic*, *next_field*, *previous_field*, *open*, *save*, and *save_as*,
	which act unless a text field takes them for typing.
	Enter leaves a text field, and opens the menu of a drop-down list, which
	takes the keyboard from there.

The help page
	The keys of *next* and *previous* scroll the page, a line for each one
	and as many lines as a count, and those of *first* and *last* go to its
	top and its bottom.  Of the rest, only the keys of *go*, *panic*, *help*,
	and *close_help* act.  The keys of *go* and *panic* close the page as
	well, so that the show is in view.

The menu of a drop-down list
	The keys bound by themselves to *next* and *previous* move the highlight
	one option at a time, whatever the count, and the highlight stays put past
	either end.
	Enter picks the highlighted option.
	The keys of *go* and *panic* act as they do everywhere and close the menu
	as well.
	The menu keeps every other key until it closes.

Show mode locks editing: *edit*, *add_below*, *add_above*, *move_down*,
*move_up*, and *delete* do nothing in it, and *undo* and *redo* only move
standby back and forth.

# ACTIONS

The actions go by their names in the *keys* table, in the order of the help
page.

*go*
	GO: fire the standby cue, together with every cue that auto-continue or
	auto-follow chains to it.

*panic*
	Panic: fade every playing cue out over one second, and stop the rest.
	A second panic while the first is still fading stops everything at once.

*next*
	Select the next cue, move the highlight of an open menu down, or scroll
	the help page down.

*previous*
	Select the previous cue, move the highlight of an open menu up, or
	scroll the help page up.

*next_list*
	Show the next cue list.

*previous_list*
	Show the previous cue list.

*first*
	Select the first cue, or the cue on the row counted, or go to the top of
	the help page.

*last*
	Select the last cue, or the cue on the row counted, or go to the bottom
	of the help page.

*next_tab*
	Show the next tab of the inspector.

*previous_tab*
	Show the previous tab of the inspector.

*standby*
	Put the selected cue on standby, so that the next GO fires it.

*edit*
	Edit the name of the selected cue.

*next_field*
	Move the keyboard to the next field: a text field or drop-down list of the
	inspector, or the list of the kind of cue that *add_below* and *add_above*
	add.

*previous_field*
	Move the keyboard to the previous field.

*add_below*
	Add a cue of the kind chosen in the toolbar below the selection, or at the
	end of the list without one.

*add_above*
	Add a cue of the kind chosen in the toolbar above the selection, or at the
	end of the list without one.

*move_down*
	Move the selected cue one place down its list, or as many places as the
	count.

*move_up*
	Move the selected cue one place up its list, or as many places as the
	count.

*delete*
	Delete the selected cue, or as many cues from it down as the count.

*undo*
	Undo the latest change to the show.

*redo*
	Redo the change undone last.

*open*
	Open a show that the file chooser of the desktop picks.
	The show on screen keeps its unsaved changes, as *cinq-show*(5)
	describes.

*save*
	Save the show in its file, or in a file that the file chooser of the
	desktop picks for a show that has none yet.

*save_as*
	Save the show in a file that the file chooser of the desktop picks,
	which the show goes by from then on.

*help*
	Show the help page, or hide it.

*close_help*
	Hide the help page.

# DIAGNOSTICS

Problems with the file show among the warnings in the sidebar, each after
*config.toml:*, and the status bar counts them.

- A file that cannot be read, or that is not TOML, leaves the keys of the vi
  keymap.
- A *keymap* that names no keymap leaves the keys of vi, and a *keys* that is
  not a table leaves those of the keymap.
  A keymap of the file's own whose *from* names no keymap starts from no keys,
  as does the one whose *from* closes a loop of *from*s followed from the
  keymap in use, and one named after a shipped keymap goes unused.
- A setting other than *keymap*, *keymaps*, and *keys*, and an entry of a
  table that names no action, are ignored, and the rest of the file applies.
- An empty string, a key that names no key, a sequence that starts with a
  digit, and a value that is not a string are left out, and the action keeps
  the other keys listed for it, which may be none.
- A sequence bound to two actions goes to the one listed first under
  *ACTIONS*, and the other goes without it.
  A sequence that begins a longer one, as *g* begins *g g*, leaves the longer
  one impossible to type.
  A key bound by itself to *go* or *panic* makes every other sequence that
  holds it impossible to type, and when it is bound without modifiers,
  whatever modifiers are held with it there, apart from a key that the other
  of the two has by itself, as *TYPING* describes.

The help page shows the keys as they ended up.

# EXAMPLES

Use the cua keymap, with GO on F5 as well as on the space bar:

```
keymap = "cua"

[keys]
go = ["Space", "F5"]
```

Keep the vi keymap, but move through cues and menus with the arrows alone,
and let no key delete a cue:

```
[keys]
next = "ArrowDown"
previous = "ArrowUp"
delete = []
```

Build a keymap of one's own from no keys at all, in which only GO, Panic, and
the arrows act:

```
keymap = "booth"

[keymaps.booth]
go = "Space"
panic = "Escape"
next = "ArrowDown"
previous = "ArrowUp"
```

# SEE ALSO

*cinq*(1), *cinq-show*(5)

UI Events KeyboardEvent key Values
	https://www.w3.org/TR/uievents-key/

TOML
	https://toml.io/
; if xdg

XDG Base Directory Specification
	https://specifications.freedesktop.org/basedir-spec/latest/
; end
