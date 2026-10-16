extends RefCounted
## Optional candidate-only expression controls; absent on unsupported face assets.

var host
var row: HBoxContainer
var sliders := {}
var labels := {}


func setup(control, column: VBoxContainer, pair_row: HBoxContainer) -> void:
	host = control
	if host.panes.size() != 2 or not host.panes[0].supports_blinks():
		return
	row = HBoxContainer.new()
	row.name = "BlinkControls"
	column.add_child(row)
	column.move_child(row, pair_row.get_index())
	for key in ["blink.L", "blink.R"]:
		host.state[key] = 0.0
		var label := Label.new()
		label.custom_minimum_size.x = 125
		row.add_child(label)
		labels[key] = label
		var slider := HSlider.new()
		slider.name = "BlinkLeft" if key == "blink.L" else "BlinkRight"
		slider.min_value = 0.0
		slider.max_value = 1.0
		slider.step = 0.01
		slider.scrollable = false
		slider.custom_minimum_size.x = 150
		slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		slider.value_changed.connect(set_weight.bind(key))
		row.add_child(slider)
		sliders[key] = slider
	host.button(row, "睁眼复位", reset)
	sync_labels()


func set_weight(value: float, key: String) -> void:
	if host._sweep_running():
		return
	host.state[key] = clampf(value, 0.0, 1.0)
	host.apply()


func reset() -> void:
	if host._sweep_running():
		return
	for key in sliders:
		host.state[key] = 0.0
	host.apply()


func sync_labels() -> void:
	for key in sliders:
		var value: float = host.state.get(key, 0.0)
		sliders[key].set_value_no_signal(value)
		labels[key].text = key + " %.2f" % value
