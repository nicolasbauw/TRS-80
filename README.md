# TRuSt-80

TRuSt-80 is a cross-platform TRS-80 (model 1) emulator, based on my [Z80 emulator](https://github.com/nicolasbauw/ZilogZ80).
It has a working keyboard, can run Level 1 and Level 2 basic, and load .cas tape images.
You will need a ROM, and the [AnotherMansTreasureMIB64C2X3Y.ttf](https://www.kreativekorp.com/swdownload/fonts/retro/amtreasure.zip) font. If not already installed, you will need sdl2 and sdl2_ttf libraries.


![Screenshot](assets/TRuSt-80.png)

You can customize the RAM, ROM and resolution, among other things, in the config.toml file.
The F12 key quits the emulator.
In the console, the `reset` command reboots the TRS-80.
The `tape rewind` command is used to "rewind" the tape.
The `tape` command followed by a filename is used to "insert" a tape:

```
tape seawar4k.cas
```
