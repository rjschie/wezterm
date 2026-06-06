---
tags:
  - tab_bar
---
# `use_fancy_tab_bar = true`

{{since('20220101-133340-7edc5b5a')}}

When set to `true` (the default), the tab bar is rendered in
a native style with proportional fonts.

When set to `false`, the tab bar is rendered using a retro
aesthetic using the main terminal font.

## Styling the Fancy Tab Bar

Config example:

```lua
config.fancy_bar = {
	font = wezterm.font "Roboto",
	font_size = 14,

    -- Container
	background = "", -- the background of the tab bar area
    padding = { -- spacing around all of the tabs
        top = 0,
        bottom = 0,
        left = 0,
        right = 0,
    },
    -- use_full_width fills the space with the tabs so that together they fill the entire window width
    -- basically it sets the padding to automatically be figured out to fill the space (other separators and marings accounted for)
    -- if set, ignores left/right padding for active_tab/inactive_tab and centers text
	use_full_width = false,

    -- Tabs:
	active_tab = {
		bg_color = "", -- color type
		fg_color = "", -- color type
		padding = { -- spacing within the tab; all tabs share greatest top/bottom padding; left/right are unused if use_full_width
			top = 0,
			bottom = 0,
			left = 0,
			right = 0,
		},
		margin = { -- spacing around the active tab; all tabs share greatest top/bottom margin
			left = 0,
			right = 0,
		},
		rounding = { -- rounding each of the corners
			top_left = 0,
			top_right = 0,
			bottom_right = 0,
			bottom_left = 0,
		},
	},
	inactive_tab = active_tab, -- has same structure as active_tab
	tab_separator = { -- settings for the separate between tabs
		bg_color = "",
		fg_color = "",
		text = "", -- can be nerdfont too
	},

    -- Buttons:
	active_close_tab_button = { -- settings specifically for active tab's close_tab button
		bg_color = "",
		fg_color = "",
		text = "", -- can be nerdfont too
	},
	inactive_close_tab_button = { -- settings specifically for inactive tab's close_tab button
		bg_color = "",
		fg_color = "",
		text = "", -- can be nerdfont too
	},
	new_tab_button = active_close_tab_button, -- settings for new tab button, matches structure of active_close_tab_button
}
```
