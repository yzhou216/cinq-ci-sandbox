cinq-show(5) "cinq" "File Formats Manual"

; SPDX-FileCopyrightText: 2026 Yiyu Zhou <yiyu@yiyuzhou.io>
; SPDX-License-Identifier: GPL-3.0-or-later

; The lines after "; if" belong only to the pages of the platforms that it
; names, xdg, darwin, or windows, until the next "; if" or "; end".

# NAME

cinq-show - show files of cinq

# SYNOPSIS

_show_.cinq.toml

# DESCRIPTION

A show of *cinq* lives in a file in TOML, which *cinq* opens from its command
line or from the file chooser of the desktop, and which saving writes.
The file lists the cue lists of the show in order, each with its cues, and
each cue with the settings that its kind uses.
What belongs to a session rather than to the show stays out of the file: the
cue on standby, which is the first of each list once the show opens, the cues
that run, and how long each sound or video file plays.

Saving leaves out the settings that the kind of a cue does not use, and those
that keep their defaults, apart from *level*, *duration*, and *stop_target*,
which it writes for every cue whose kind uses them.
A file can leave out any setting but the *name* of a list and the *id* and the
*kind* of a cue.
*cinq* passes over settings that it does not know.

# LISTS

Each cue list is a table in the array *list*, in order:

*name*
	Name of the list, as the sidebar shows it.

*cue*
	Array of the cues of the list, in order.

A file without lists holds one empty list.

# CUES

Each cue is a table in the array *cue* of its list:

*id*
	Identity of the cue, a whole number that no other cue of the show has,
	by which fade and stop cues name their targets.

*number*
	Number of the cue, as the cue list shows it, which can be any text.

*kind*
	What the cue does: *audio*, *video*, *fade*, *stop*, *wait*, or *memo*.
	An audio cue plays a sound file, and a video cue plays a video file, as
	*VIDEO* describes.

*name*
	Name of the cue.

*file*
	Sound file that an audio cue plays, or video file that a video cue plays:
	an absolute path, or one relative to the directory of the show file.
	Sound files hold WAV, FLAC, Ogg Vorbis, MP3, or AAC in MP4.

*start*
	Seconds into its file at which an audio or video cue starts, which are
	none by default.

*end*
	Seconds into its file at which an audio or video cue ends, which is the
	end of the file by default.
	A cue that starts at or after its end cannot play.

*loop*
	Whether an audio or video cue plays from its start again each time it
	reaches its end, which it does not by default.
	A cue that loops plays until something stops it, such as a fade cue that
	stops it, a stop cue, or a panic.

*target*
	Identity of the cue that a fade or stop cue acts on.
	A fade cue acts on an audio or video cue, and a stop cue on any other
	cue.

*pre_wait*
	Seconds between the cue firing and its action, which are none by
	default.

*duration*
	Seconds that the action of a fade or wait cue takes, which are five by
	default.

*post_wait*
	Seconds between the action of the cue beginning, once its pre-wait is
	over, and the next cue firing, for a cue that continues to it, which are
	none by default.

*advance*
	What happens to the next cue once this one has fired: *manual*, the
	default, leaves it waiting for GO, *continue* fires it once the
	post-wait is over, and *follow* fires it once the action of this cue
	ends, which that of an audio cue that loops never does.

*level*
	Level in dB that an audio or video cue plays at, which is 0 by default, or
	that a fade cue fades its target to, which is -60, silence, by default.
	It lies between -60 and 12.

*stop_target*
	Whether a fade cue stops its target once the fade is over, which it does
	by default.

*notes*
	Notes of the cue, for the operator.

A file in which two cues share an identity, or a level lies outside its range,
cannot be opened.

# VIDEO

A video cue plays H.264 video in MP4, as files ending in _.mp4_, _.m4v_, or
_.mov_ hold it, along with the sound of its AAC track, in mono or stereo.
A file without a sound track plays its picture alone.
A video cue takes as long as the picture of its file, and its *start*, *end*,
*loop*, *level*, and waits work as those of an audio cue do.

The pictures of video cues show in a stage window of their own, which opens
beside the main window once the show holds a video cue.
It shows black while no video plays, and otherwise the picture of the video
cue that fired last, whole and as large as the window allows, with black
around it where its shape differs from that of the window.
A fade cue that stops a video cue fades its picture out along with its sound,
as a panic does: to black, or to the picture of an earlier video cue that
plays on.
Pausing holds the picture, and a cue that loops loops its picture with its
sound.

Closing the stage window hides it until a video cue fires again, while closing
the main window quits *cinq*.
The key of *fullscreen*, as *cinq*(5) describes, toggles the stage window
between fullscreen and a window, and every other key acts the same in either
window.

# UNSAVED CHANGES

Changes to a show that are not saved yet never go away unasked.
As they happen, *cinq* keeps them in a file of their own, in the directory

; if xdg
	_$XDG_STATE_HOME/cinq/unsaved_

where *XDG_STATE_HOME* is _~/.local/state_ when it is unset or is not an
absolute path, as the XDG Base Directory Specification sets out.
; if darwin
	_~/Library/Application Support/cinq/unsaved_
; if windows
	_%LOCALAPPDATA%\\cinq\\unsaved_
; end

The changes to a show file go under _shows_ in that directory, by the
absolute path of the file, and those to a show that has no file yet go in
_untitled.cinq.toml_.

Closing the window, opening another show, or a crash leaves the kept changes
in place.
The next time that the show opens, they come back as one change, which undo
takes back to the show as saved.
Saving the show, or taking back each of its changes, removes them.

# EXAMPLES

A list in which a storm loops from half a second into its file, and continues
to a fade cue, which fades it out over eight seconds and then stops it:

```
[[list]]
name = "Main"

[[list.cue]]
id = 1
number = "1"
kind = "audio"
name = "Storm"
file = "sounds/storm.ogg"
start = 0.5
loop = true
level = -4.0
advance = "continue"
post_wait = 2.0

[[list.cue]]
id = 2
number = "2"
kind = "fade"
name = "Storm out"
target = 1
duration = 8.0
level = -60.0
stop_target = true
```

A list in which a video plays from two seconds into its file, a little below
the level of its sound, until a fade cue fades its picture to black and its
sound to silence over three seconds, and then stops it:

```
[[list]]
name = "Main"

[[list.cue]]
id = 1
number = "1"
kind = "video"
name = "Opening titles"
file = "video/titles.mp4"
start = 2.0
level = -6.0

[[list.cue]]
id = 2
number = "2"
kind = "fade"
name = "Titles out"
target = 1
duration = 3.0
level = -60.0
stop_target = true
```

# SEE ALSO

*cinq*(1), *cinq*(5)

TOML
	https://toml.io/
; if xdg

XDG Base Directory Specification
	https://specifications.freedesktop.org/basedir-spec/latest/
; end
