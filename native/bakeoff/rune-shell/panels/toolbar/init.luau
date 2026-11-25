-- The toolbar as a script (spike step S7, ADR 0055 lane 1).
--
-- It reads one thing: the toolbar snapshot the host sends on `app.toolbar`,
-- plain data made from `toolbar(&App)`. It changes the app one way: by
-- `app.run(id)`, where `id` is a control of that snapshot. Which list is
-- open is this panel's own state, and survives a reload.
--
-- Save this file while `rune-shell --chrome luau` runs and the window
-- redraws from it.
local ui = require("@rune/ui")
local app = require("@specular/app")

local model = nil
local open: string? = nil

app.toolbar:Connect(function(next)
	model = next
end)

local BUTTON_HEIGHT = 28

local LABEL = table.freeze({ size = 12, weight = "medium", color = "ink" })
local LABEL_LIT = table.freeze({ size = 12, weight = "medium", color = "ink-strong" })
local BUTTON = table.freeze({
	role = "button",
	focusable = true,
	h = BUTTON_HEIGHT,
	px = 8,
	align = "center",
	radius = 6,
})
local OPTION = table.freeze({
	role = "menuitemradio",
	h = BUTTON_HEIGHT,
	px = 10,
	align = "center",
	radius = 4,
	hover = "hover",
})
local LIST = table.freeze({
	column = true,
	role = "menu",
	pin = { left = 0, top = BUTTON_HEIGHT + 6 },
	pad = 4,
	radius = 8,
	fill = "surface",
	border = "edge",
	borderWidth = 1,
})
local RIBBON = table.freeze({
	key = "toolbar",
	role = "toolbar",
	label = "Tools",
	pad = 4,
	gap = 2,
	align = "center",
	radius = 10,
	fill = "surface",
	border = "edge",
	borderWidth = 1,
})
local DIVIDER = table.freeze({ w = 1, h = 16, fill = "edge" })
local WINDOW = table.freeze({ w = "full", h = "full", align = "center", pad = 8 })

-- The last part of a control's name: `shape` of `tool.shape`.
local function short(id: string): string
	return string.match(id, "[^.]+$") or id
end

-- One pressable control: keyed by its id, named by its label, showing
-- `word`. `on` is nil for a plain button.
local function pressable(control, word: string, on: boolean?)
	local lit = on == true
	return ui.control(BUTTON, {
		key = control.id,
		label = control.label,
		checked = on,
		disabled = control.enabled == false,
		fill = if lit then "raised" else nil,
		hover = if lit then "raised" else "hover",
		onPress = function()
			open = nil
			app.run(control.id)
		end,
	}, { ui.text(word, if lit then LABEL_LIT else LABEL) })
end

local function option(item)
	return ui.control(OPTION, {
		key = item.id,
		label = item.label,
		checked = item.selected,
		fill = if item.selected then "raised" else nil,
		onPress = function()
			open = nil
			app.run(item.id)
		end,
	}, { ui.text(item.text or short(item.id), LABEL) })
end

local control

-- A dropdown: its trigger in the ribbon and, while it is open, its options
-- in a list under it.
local function dropdown(model, canOpen: boolean)
	local trigger = ui.control(BUTTON, {
		key = model.id,
		label = model.label,
		hasPopup = "menu",
		hover = "hover",
		onPress = function()
			open = if open == model.id then nil else model.id
		end,
	}, { ui.text(model.text or short(model.id), LABEL) })
	if not canOpen or open ~= model.id then
		return trigger
	end
	local items = {}
	for _, item in model.options do
		table.insert(items, if item.kind == "option" then option(item) else control(item))
	end
	local list = ui.control(LIST, { key = model.id .. ".list", label = model.label }, items)
	return ui.box({}, { trigger, list })
end

function control(model)
	if model.kind == "button" then
		return pressable(model, model.text or short(model.id), nil)
	elseif model.kind == "toggle" then
		return pressable(model, model.text or short(model.id), model.on)
	elseif model.kind == "dropdown" then
		return dropdown(model, false)
	elseif model.kind == "separator" then
		return ui.box(DIVIDER)
	end
	return nil
end

local function ribbon()
	local items = {}
	for _, section in model.sections do
		if #items > 0 then
			table.insert(items, ui.box(DIVIDER))
		end
		for _, item in section.controls do
			if section.kind == "zoom" then
				table.insert(items, dropdown(item, true))
			else
				table.insert(items, pressable(item, short(item.id), item.on))
			end
		end
	end
	table.insert(items, pressable(model.theme, "theme", nil))
	table.insert(items, pressable(model.share, "share", nil))
	if model.chat then
		table.insert(items, pressable(model.chat, "panel", model.chat.on))
	end
	if model.view and #model.view > 0 then
		table.insert(items, ui.box(DIVIDER))
		for _, item in model.view do
			table.insert(items, control(item))
		end
	end
	return ui.control(RIBBON, items)
end

return {
	-- The whole window: nothing but the ribbon, centred at the top.
	render = function(view)
		if not model then
			return ui.col(WINDOW, {})
		end
		return ui.col(WINDOW, { ribbon() })
	end,
	saveHotState = function()
		return { open = open }
	end,
	loadHotState = function(state)
		open = state.open
	end,
}
