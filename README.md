# Space Gfx
![alt text](https://github.com/sirkibsirkib/space_gfx/tree/master/resources/screencap.png)

## What is this?
It's a game where you pilot a ship through _spaaaaace_. There isn't much to do
just yet. There are planets around that just kinda sit there.

## What is there to see?
The universe is just a bunch of planets. The space is in 2D, but stretches
"infinitely" in all directions. Using a sigmoid projection, you can see it 
all at once, as if its wrapped over a globe.

## Trajectory
The main appeal of the game is to mess with your trajectory. At all times there
is a little pink stripe from your ship into the abyss. It predicts your
motion in the next seconds / minutes. It tends to curl and bend with the
pull of the planets. You can bump your trajectory by tapping or holding WASD.

## Messing with params
The projection is somewhat complicated. In simple terms its a sigmoid function:
```
fn(x) = 2 / (1 + e^(-x)) - 1 
```
This has the effect of wrapping an infinite x domain onto a bounded y domain.
however, you can mess with a few of these parameters. You can scale and shift x,
which has the effect of warping and pinching space around your ship. These
variables, along with the length of your trajectory can be manipulated by
selecting a variable with numbers 1-6, and scrolling the mousewheel.
