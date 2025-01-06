# example showcase scripts

when recording videos for showcasing examples, follow these scripts to highlight all available features

TODO: this should be able to be automated with a selenium type abstraction layer using the engine itself, e.g. by ingesting a list of tweened mouse movements and clicks, etc.

## `align`
1. click through all the alignments for both self alignment and content alignment, waiting a second between each

## `main_menu`
1. hover the audio button, wait a second, then hover the graphics button
1. click the audio button
1. hover over each menu item up and down, and then again faster
1. click the dropdown button, wait a second, then click it again
1. click the dropdown button, hover over every option and then click option 1
1. click the dropdown button, click option 2
1. click the dropdown button, click the x button within the dropdown button
1. click the dropdown button, click option 4
1. click the x button within the dropdown button
1. hover over each mutually exclusive option
1. click option 1, then click it again
1. click option 2, then click option 3
1. hover the checkbox, then unhover it
1. click the checkbox, then uhover it
1. click the checkbox
1. click the iterable options button on the right until cycling around back to option 1
1. click the iterable options button on the left until cycling around back to option 1
1. jiggle each volume option for a second
1. click the x button in the top right to close the audio menu
1. click the audio button, wait a second, then click the x button in the top right
1. click the graphics button
1. click the x button in the preset quality dropdown button
1. click the preset quality dropdown button and click low option
1. click the preset quality dropdown button and then click the x button and then click the medium option
1. click the preset quality dropdown button and then click the high option
1. click through the non preset quality dropdown buttons and click on the ultra option for each
1. click the x button in the top right to close the graphics menu
1. click the graphics button, wait a second, then click the x button in the top right

## `inventory`
1. slowly hover over various items, then faster
1. click on an item and then hover it over other items/slots, then hover it outside the inventory
1. place the item in an empty slot, then pick it up and place it in another empty slot
1. pick up the item and swap it with other items a few times
1. right click the item to split the stack into two, and place the split in an empty slot, do this a few times
1. recombine the split stacks
1. take an item and right click on empty slots to place a single item from the stack
1. fill the crafting slots with items
1. replace the items in the crafting slots a few times
1. take the item from the crafting output and place it in the upper inventory

## `healthbar`
1. move the character to each corner of the screen, then respawn and wiggle a bit

## `responsive_menu`
1. hover over each button up and down, and then again faster
1. click on a few of the buttons
1. slowly resize the window down to the minimum width
1. resize the window back to slightly above 400 and then wiggle it over and under a few times

## `character_editor`
1. click the sphere option, then click the plane option
1. scroll the options up and down a few times
1. scroll all the way down and click through those options
1. type a few names into the text input
1. type "sphere" into the text input, then click the plane option
1. type "cuboid" into the text input
1. type "torus" into the text input

## `ecs_ui_sync`
1. let the example run for a few seconds
1. increase the spawn rate to 3.0 and let the example run for a few seconds
1. increase the despawn rate to 4.0 and let the example run for a few seconds
1. increase the spawn rate to 10.0 and then increase the despawn rate to 12.0 and let the example run for a few seconds

## `snake`
1. eat a few food
1. increase the grid size to 40 and increase the tick rate to 30
1. eat a few food
1. lose
1. respawn and eat one more food

## `button`
1. hover/unhover the button
1. press/unpress the button

## `counter`
1. click the plus button to count 3
1. click the minus button to count -3
1. click the plus button to count 0

## `key_values_sorted`
1. click a few text inputs
1. scroll up and down a few times
1. add an "a" to an input that starts with a "d" and press tab
1. add an "a" to the current input and press escape
1. tab to the next input that starts with a "d", add an "a" and press escape
1. scroll down such that the bottom text input is approximately clipped in half, then click on the key input
1. press tab a few times
1. scroll up such that the top text input is approximately clipped in half, then click on the value input
1. press shift tab until wrapping around to the bottom
1. press tab until wrapping around to the top
1. press the plus button
1. fill the key and value with the word "test"
1. click the sort by value button
1. press tab until at a value input with starts with the letter "d", add an "a" and press escape
1. click between the sort by key and value button a few times
1. remove rows until only a few are left
1. tab to cycle through the remaining inputs, then shift tab to cycle in the other direction
1. click the plus button until it causes a scroll

## `scroll_grid`
1. scroll each column a bit
1. hold shift and scroll each row a bit
1. alternate pressing shift and scroll all over the place

## `scroll`
1. scroll the first four columns to the top
1. scroll the fifth column to the bottom
1. hold shift and scroll right
1. scroll the sixth and seventh columns to the top
1. hold shift and scroll left to right 
