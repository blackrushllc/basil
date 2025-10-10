REM Live poly synth: MIDI keyboard -> audio output. Stop with DAW_STOP from another Basil instance or Ctrl+C.

DAW_RESET;


REM LET rc% = SYNTH_LIVE%("launchkey", "usb", 16);
REM LET rc% = SYNTH_LIVE%("MIDIIN2 (LKMK3 MIDI)", "LC27T55 (NVIDIA High Definition Audio)", 16);
LET rc% = SYNTH_LIVE%("LKMK3 MIDI", "LC27T55 (NVIDIA High Definition Audio)", 6);
IF rc% <> 0 THEN PRINT "Error: ", DAW_ERR$();


