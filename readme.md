# J-Pong

## How to play

It's first to 9 points

Player 1: 
 * W to move up
 * S to move down

Player 2:
 * Up to move up
 * Down to move down

## Compilier Requirements
Instructions mostly here: 

https://github.com/vulkano-rs/vulkano

You need rust installed.

### Windows

* Cmake, Ninja, Python needs to be installed.

* The Build Tools for Visual Studio 2017 needs to be installed

* msys2 

* pacman --noconfirm -Syu mingw-w64-x86_64-cmake mingw-w64-x86_64-python2 

* However use the Windows version of ninja rather than msys2

### Linux 

```sudo apt-get install build-essential git python cmake libvulkan-dev vulkan-utils```


## Compiling

Run:
```cargo run --release```

## vulkano-text
I couldn't figure out how to get around the mismatched dependancies of vulkano-text so I just copied it into a separate workspace and changed the cargo.toml

