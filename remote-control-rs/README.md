# EV3 Remote Control Drive

A simple remote controlled driving routine for the
  [ev3dev platform](https://www.ev3dev.org/),
  where the robot:

1. Hosts a small HTTP webserver on port 8080.
    * This web application contains buttons for driving the robot.
    * Other devices on the network can access this webpage at
        <http://ev3dev.local:8080/>.
2. The robot waits for commands from the web application,
     executes them, and then waits for more commands.

This routine is implemented in [./src/main.rs](./src/main.rs).
See it for more details.


## The Lego Build

This program was built for any EV3 driving build that includes:

* Front-wheel drive, where the two front wheels
    are independently powered by medium motors,
    and the back "wheel" is just a marble thingy.
* A wifi adapter that is connected to a local network.
  Specifically, the
    [Penguin Wireless N USB Adapter (TPE-N150USB)](https://www.thinkpenguin.com/gnu-linux/penguin-wireless-n-usb-adapter-gnu-linux-tpe-n150usb)
    worked flawlessly for me,
    though others recommended for the ev3dev platform did not.

Specifically, this is all based off the basic EV3 Mindstorms
  educational driving platform from the tutorials.