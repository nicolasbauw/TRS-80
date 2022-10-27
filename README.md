# TRuSt-80

This is a "Work in Progress" TRS-80 emulator, based on my [Z80 emulator](https://github.com/nicolasbauw/ZilogZ80).
It has a working keyboard and can run Level 1 and Level 2 basic.
You will need a ROM (this [diagnostic ROM](https://github.com/misterblack1/trs80-diagnosticrom/blob/main/trs80m13diag.bin) for example), and the [AnotherMansTreasureMIB64C2X3Y.ttf](https://www.kreativekorp.com/swdownload/fonts/retro/amtreasure.zip) font.
![Screenshot](assets/TRuSt-80.png)

To run:
```
cargo run
```

You can customize the RAM, ROM and resolution in the config.toml file.