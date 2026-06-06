# `window:toast_notification(title, message, [url, [timeout_milliseconds, [sound]]])`

{{since('20210502-154244-3f7122cb')}}

Generates a desktop "toast notification" with the specified *title* and *message*.

An optional *url* parameter can be provided; clicking on the notification will
open that URL.

An optional *timeout* parameter can be provided; if so, it specifies how long
the notification will remain prominently displayed in milliseconds.  To specify
a timeout without specifying a url, set the url parameter to `nil`.  The timeout
you specify may not be respected by the system, particularly in X11/Wayland
environments, and Windows will always use a fixed, unspecified, duration.

An optional *sound* boolean parameter can be provided; if `true`, the
notification will play the system default notification sound. Defaults to
`false`.  To specify sound without specifying a timeout, set timeout to `nil`.

The notification will persist on screen until dismissed or clicked, or until its
timeout duration elapses.

This example will display a notification whenever a window has its configuration
reloaded.  The notification should remain on-screen for approximately 4 seconds
(4000 milliseconds), but may remain longer depending on the system.

It's not an ideal implementation because there may be multiple windows and thus
multiple notifications:

```lua
local wezterm = require 'wezterm'

wezterm.on('window-config-reloaded', function(window, pane)
  window:toast_notification('wezterm', 'configuration reloaded!', nil, 4000)
end)

return {}
```
