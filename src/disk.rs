/* Disk controller registers

$37E1 (W) Select disk drive :
1 --> Drive 1
2 --> Drive 2
4 --> Drive 3
8 --> Drive 4

$37EC Command (W) / Status (R) register
For a positioning command (RESTORE, SEEK, STEP, STEP-IN, STEP-OUT), the Status Register holds status as follows:
Status Register : 7 6 5 4 3 2 1 0
                  | | | | | | | |
                  | | | | | | | - BUSY
                  | | | | | | --- INDEX
                  | | | | | ----- TRACK 0
                  | | | | ------- CRC ERROR
                  | | | --------- SEEK ERROR
                  | | ----------- HEAD ENGAGED
                  | ------------- WRITE PROTECT
                  --------------- NOT READY

During a READ or WRITE command, the status register holds status as follows:
Status Register : 7 6 5 4 3 2 1 0
                  | | | | | | | |
                  | | | | | | | - BUSY
                  | | | | | | --- DRQ
                  | | | | | ----- LOST DATA
                  | | | | ------- CRC ERROR
                  | | | --------- RECORD NOT FOUND
                  | | ----------- RECORD TYPE(RD) OR WRITE FAULT
                  | ------------- RECORD TYPE(RD) OR WRITE PROTECT
                  --------------- NOT READY

$37ED (R/W)
Track register : May be automatically updated with each STEP.
May be read by cpu.

$37EE (R/W)
Sector register : Setup with sector number prior to a read or
write. May be read by cpu.

$37EF (R/W)
Data register : Used to hold data passing between the cpu and disc during reads or writes. Used continuously as data is transferred one byte
at a time.

*/
