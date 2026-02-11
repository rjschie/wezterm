# `EqualizePanes`

{{since('nightly')}}

Distributes space equally among all panes in the current tab. Each pane receives
a proportional share of the available space based on the split tree structure.

```lua
config.keys = {
  {
    key = '=',
    mods = 'CTRL|SHIFT',
    action = wezterm.action.EqualizePanes,
  },
}
```

See also: [tab:equalize_panes()](../MuxTab/equalize_panes.md).
