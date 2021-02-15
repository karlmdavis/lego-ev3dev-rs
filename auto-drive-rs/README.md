# EV3 Auto Drive

A simple "auto pilot" driving routine for the
  [ev3dev platform](https://www.ev3dev.org/),
  where the robot:

1. Drives in a straight line until it encounters an obstacle.
2. When an obstacle is encountered, it turns a bit, backwards,
     and then starts driving straight again.

This routine is implemented in [./src/main.rs](./src/main.rs).
See it for more details.


## The Lego Build

This program was built for any EV3 driving build that includes:

* Front-wheel drive, where the two front wheels
    are independently powered by medium motors,
    and the back "wheel" is just a marble thingy.
* The ultrasonic sensor is mounted on the front of the robot.
* A touch sensor is also mounted on the front of the robot,
    extending just a bit further than everything else.

Specifically, this is all based off the basic EV3 Mindstorms
  educational driving platform from the tutorials.