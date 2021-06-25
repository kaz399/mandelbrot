# Mandelbrot

Self-study program for drawing the Mandelbrot set.

This code based on the "[pixels](https://github.com/parasyte/pixels)" sample code.

<img src="image/IMG-2021-06-22-18-36-07.png" width="320">
<img src="image/IMG-2021-06-22-18-37-06.png" width="320">

## Multi Platform

This program works on multi platforms. (Windows, MacOS, Linux)

## Run

```
cargo run --release
```


## Operation

* Mouse left double click : set the double-clicked point to the center
* Mouse dragging (with holding down the left button) : move the center to the drag direction
* Mouse wheel : zoom in/out
* <kbd>Space</kbd> : reset the center position and the zoom scale
* <kbd>PageUp</kbd>/<kbd>PageDown</kbd> : zoom in/out (with holding down the shift key, the moving distance is small)
* <kbd>Alt</kbd><kbd>PageUp</kbd>/<kbd>Alt</kbd><kbd>PageDown</kbd> : auto zoom in/out
* <kbd>Up</kbd>/<kbd>Down</kbd>/<kbd>Left</kbd>/<kbd>Right</kbd> : move the center position
* <kbd>I</kbd> : toggle information display
* <kbd>Escape</kbd> : stop auto zoom
* <kbd>Q</kbd> : quit

## History

June 25, 2021: Support auto zoom function  
June 24, 2021: Support keyboard control and information display  
June 23, 2021: Improve performance  
June 22, 2021: 1st Release  
