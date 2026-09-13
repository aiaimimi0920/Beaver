extends GutTest

const Counter = preload("res://counter.gd")


func test_increment_accumulates() -> void:
	var counter := Counter.new()
	counter.increment()
	counter.increment()
	assert_eq(counter.value, 2)
